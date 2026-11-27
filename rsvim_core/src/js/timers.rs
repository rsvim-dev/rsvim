//! The timer table.
//!
//! One ordered table and one `Sleep`, both owned by the host, instead of one
//! spawned future per timer:
//!
//! - `clearTimeout` is a `remove`, so the callback (and everything its closure
//!   captured) is released immediately. A debounce -- `clearTimeout(id); id =
//!   setTimeout(f, 300)` on every keystroke -- leaves one live entry, not one
//!   per keystroke until the delay runs out.
//! - Timers that share a deadline fire in creation order, which is what
//!   javascript promises. The table is keyed by `(deadline, sequence)` for that
//!   reason: `tokio_util::time::DelayQueue` was tried first and pops the entries
//!   of one millisecond slot in reverse order.
//! - The loop `select!`s on [`Timers::poll_due`], so a due timer is an event the
//!   loop sees, not something that happens inside a future's `poll`.

use crate::js::TimerId;
use crate::prelude::*;
use rquickjs::Function;
use rquickjs::Persistent;
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::task::Context;
use std::task::Poll;
use std::time::Duration;
use tokio::time::Instant;
use tokio::time::Sleep;

/// The largest delay javascript can ask for (`2^31 - 1` ms, about 24 days).
pub const MAX_DELAY: Duration = Duration::from_millis(i32::MAX as u64);

type Key = (Instant, u64);

struct Entry {
  key: Key,
  callback: Persistent<Function<'static>>,
  /// `Some(period)` for `setInterval`.
  repeat: Option<Duration>,
}

/// A timer that is due: the callback to run.
pub struct DueTimer {
  pub id: TimerId,
  pub callback: Persistent<Function<'static>>,
}

#[derive(Default)]
pub struct Timers {
  by_deadline: BTreeMap<Key, TimerId>,
  entries: FoldMap<TimerId, Entry>,
  seq: u64,
  /// Created on the first wait, because a `Sleep` needs a tokio runtime and a
  /// table can be built before there is one.
  sleep: Option<Pin<Box<Sleep>>>,
}

impl Timers {
  /// Arms a timer and returns its id.
  pub fn insert(
    &mut self,
    callback: Persistent<Function<'static>>,
    delay: Duration,
    repeat: bool,
  ) -> TimerId {
    let id = TimerId::next();
    let delay = delay.clamp(Duration::from_millis(1), MAX_DELAY);
    let key = self.key_at(Instant::now() + delay);
    self.by_deadline.insert(key, id);
    self.entries.insert(
      id,
      Entry {
        key,
        callback,
        repeat: repeat.then_some(delay),
      },
    );
    id
  }

  /// Disarms a timer. Returns whether it was armed.
  pub fn remove(&mut self, id: TimerId) -> bool {
    match self.entries.remove(&id) {
      Some(entry) => {
        self.by_deadline.remove(&entry.key);
        true
      }
      None => false,
    }
  }

  /// How many timers are armed.
  pub fn len(&self) -> usize {
    self.entries.len()
  }

  pub fn is_empty(&self) -> bool {
    self.entries.is_empty()
  }

  /// Drops every callback.
  pub fn clear(&mut self) {
    self.by_deadline.clear();
    self.entries.clear();
    self.sleep = None;
  }

  /// Takes the earliest timer if it is due at `now`.
  pub fn pop_due(&mut self, now: Instant) -> Option<DueTimer> {
    let (&key, &id) = self.by_deadline.first_key_value()?;
    if key.0 > now {
      return None;
    }
    self.by_deadline.remove(&key);

    let repeat = self.entries.get(&id)?.repeat;
    match repeat {
      None => {
        let entry = self.entries.remove(&id)?;
        Some(DueTimer {
          id,
          callback: entry.callback,
        })
      }
      Some(period) => {
        // Re-arm *before* the callback runs, so a `clearInterval` inside it
        // removes the entry that is actually armed. The rate is fixed (no drift
        // from the callback's own duration); ticks that were missed while the
        // loop was busy are skipped instead of fired back to back.
        let mut next = key.0 + period;
        if next <= now {
          next = now + period;
        }
        let next_key = self.key_at(next);
        self.by_deadline.insert(next_key, id);
        let entry = self.entries.get_mut(&id)?;
        entry.key = next_key;
        Some(DueTimer {
          id,
          callback: entry.callback.clone(),
        })
      }
    }
  }

  /// `Ready(Some(timer))` when one is due, `Pending` (with the waker armed for
  /// the earliest deadline) when some are armed, `Ready(None)` when none are.
  ///
  /// `Ready(None)` is what lets a `select!` pattern `Some(..)` disable the
  /// branch while the table is empty, instead of spinning on it.
  pub fn poll_due(&mut self, cx: &mut Context<'_>) -> Poll<Option<DueTimer>> {
    let Some((&key, _)) = self.by_deadline.first_key_value() else {
      return Poll::Ready(None);
    };
    let now = Instant::now();
    if key.0 <= now {
      return Poll::Ready(self.pop_due(now));
    }

    let sleep = self
      .sleep
      .get_or_insert_with(|| Box::pin(tokio::time::sleep_until(key.0)));
    if sleep.deadline() != key.0 {
      sleep.as_mut().reset(key.0);
    }
    match sleep.as_mut().poll(cx) {
      Poll::Ready(()) => match self.pop_due(Instant::now()) {
        Some(timer) => Poll::Ready(Some(timer)),
        None => {
          // The sleep fired but nothing is due (the clock moved under us):
          // ask to be polled again instead of reporting an empty table.
          cx.waker().wake_by_ref();
          Poll::Pending
        }
      },
      Poll::Pending => Poll::Pending,
    }
  }

  fn key_at(&mut self, deadline: Instant) -> Key {
    self.seq += 1;
    (deadline, self.seq)
  }
}
