//! Tests for the event loop's javascript tick semantics.

use crate::chan::JsMessage;
use crate::cli::CliOptions;
use crate::evloop::TickOutcome;
use crate::evloop::is_ctrl_d;
use crate::evloop::writer::StdoutWriterValue;
use crate::prelude::*;
use crate::tests::evloop::*;
use crate::tests::log::init as test_log_init;
use assert_fs::prelude::*;
use crossterm::event::Event;
use std::time::Duration;

/// Reads the command-line messages that were emitted during the test.
fn cmdline_messages(event_loop: &crate::evloop::EventLoop) -> Vec<String> {
  let cmdline_text = lock!(event_loop.editor.cmdline_text);
  cmdline_text.message_history().iter().cloned().collect()
}

/// Regression test for P0-1: an unhandled promise rejection must be reported
/// exactly once and must not leave the runtime thinking there's pending work
/// (which used to make the old tick-again request/response loop spin
/// forever).
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn rejection_does_not_spin() -> IoResult<()> {
  test_log_init();

  let src: &str = r#"
  Promise.reject(new Error('x'));
"#;
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop = make_event_loop(10, 10, CliOptions::empty());
  event_loop.initialize()?;

  // `initialize` already ran one tick (the "extra tick for pending imports"),
  // so the rejection has been captured and reported by now.
  assert!(
    !event_loop
      .kernel
      .borrow()
      .exceptions
      .has_promise_rejection(),
    "the rejection must have been flushed, otherwise the loop would spin"
  );

  // Two more ticks must be idle and must not report the error again.
  assert_eq!(event_loop.tick_js(vec![]), TickOutcome::Idle);
  assert_eq!(event_loop.tick_js(vec![]), TickOutcome::Idle);

  let messages = cmdline_messages(&event_loop);
  let reported: Vec<&String> = messages
    .iter()
    .filter(|m| m.contains("Uncaught (in promise) Error: x"))
    .collect();
  assert_eq!(
    reported.len(),
    1,
    "the rejection must be reported exactly once, got: {messages:?}"
  );

  event_loop.shutdown()?;
  Ok(())
}

/// All the rejections must be reported, not just the first one.
///
/// Before the `check_exceptions -> Vec<JsError>` fix, the whole
/// `promise_rejections` vector was `mem::take`n but every branch returned
/// `Some(error)`, so only the first rejection ever reached the user.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn all_rejections_are_reported() -> IoResult<()> {
  test_log_init();

  let src: &str = r#"
  Promise.reject(new Error('first'));
  Promise.reject(new Error('second'));
  Promise.reject(new Error('third'));
"#;
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop = make_event_loop(10, 10, CliOptions::empty());
  event_loop.initialize()?;

  let messages = cmdline_messages(&event_loop);
  for expected in ["first", "second", "third"] {
    let needle = format!("Uncaught (in promise) Error: {expected}");
    assert!(
      messages.iter().any(|m| m.contains(&needle)),
      "expected {needle:?} to be reported, got: {messages:?}"
    );
  }

  event_loop.shutdown()?;
  Ok(())
}

/// A plain (rejection-free) config must leave the tick idle.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn clean_config_tick_is_idle() -> IoResult<()> {
  test_log_init();

  let src: &str = r#"
  const a = 1 + 1;
"#;
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop = make_event_loop(10, 10, CliOptions::empty());
  event_loop.initialize()?;

  assert_eq!(event_loop.tick_js(vec![]), TickOutcome::Idle);
  assert!(
    !event_loop
      .kernel
      .borrow()
      .exceptions
      .has_promise_rejection()
  );

  event_loop.shutdown()?;
  Ok(())
}

/// How many times the event loop asked the writer to repaint.
fn write_count(event_loop: &crate::evloop::EventLoop) -> usize {
  match &event_loop.writer {
    StdoutWriterValue::DevNullWriter(w) => w.write_count(),
    _ => unreachable!("tests always run with the dev-null writer"),
  }
}

/// A completion message is the only thing that drives a tick.
///
/// P0-2 regression: the old loop answered "there is more work" by sending a
/// request through a channel and waiting for the response, which both spun and
/// added a whole select round-trip. Now a single `tick` runs Phase A to
/// quiescence, so the completion is consumed inside that one call -- no retry
/// loop, no budget.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn tick_is_driven_by_completion_only() -> IoResult<()> {
  test_log_init();

  let src: &str = r#"
  setTimeout(() => {}, 10000);
"#;
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop = make_event_loop(10, 10, CliOptions::empty());
  event_loop.initialize()?;

  // `initialize` evaluated the module, so the timer is registered and the
  // corresponding `MasterMessage::TimeoutReq` is still queued in the channel.
  let timer_id = {
    let kernel = event_loop.kernel.borrow();
    assert_eq!(kernel.pending_timers.len(), 1);
    *kernel.pending_timers.keys().next().unwrap()
  };

  // Deliver the completion by hand: no sleeping, no real timer.
  event_loop
    .jsrt_tx
    .send(JsMessage::TimeoutResp(crate::chan::TimeoutResp {
      timer_id,
      expire_at: tokio::time::Instant::now(),
      delay: 10000,
      repeated: false,
    }))
    .unwrap();

  // One ordinary keystroke is enough to wake Phase B without ending the loop,
  // so the assertion below is about the completion, not about quitting.
  let mut io = MockEventReader::new(vec![MockEvent::Event(Event::Key(
    crossterm::event::KeyEvent::new(
      crossterm::event::KeyCode::Char('x'),
      crossterm::event::KeyModifiers::NONE,
    ),
  ))]);
  let noop = |_: &mut crate::evloop::Rsvim, _: Option<IoResult<Event>>| {};

  // One call is enough: Phase A drains the javascript inbox and settles the
  // master channel, so the completion cannot survive into Phase B.
  let flow = event_loop.tick(&mut io, &is_ctrl_d, &noop).await?;
  assert!(
    flow.is_continue(),
    "handling a completion must not exit the loop",
  );
  assert!(
    !event_loop
      .kernel
      .borrow()
      .pending_timers
      .contains_key(&timer_id),
    "the completion must be consumed by the same tick",
  );

  event_loop.shutdown()?;
  Ok(())
}

/// Nothing is repainted unless something is actually dirty.
///
/// The pre-refactor loop ran `draw()` + `write()` after *every* select
/// iteration, so a busy javascript loop turned into a full repaint per tick.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn render_only_when_dirty() -> IoResult<()> {
  test_log_init();

  let src: &str = r#"
  const a = 1 + 1;
"#;
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop = make_event_loop(10, 10, CliOptions::empty());
  event_loop.initialize()?;

  let before = write_count(&event_loop);

  // A tick that runs no javascript cannot have touched the editor, so it must
  // not ask for a repaint. This is the property that stops a busy javascript
  // loop from repainting on every tick: `dirty` is owned by the tick itself,
  // not set unconditionally by whichever select branch fired.
  event_loop.dirty = false;
  let outcome = event_loop.tick_js(vec![]);
  assert_eq!(outcome, TickOutcome::Idle);
  assert!(
    !event_loop.dirty,
    "an idle tick must not mark the terminal dirty"
  );

  // 100 idle iterations must not reach the writer at all. This is checked at
  // `render_if_dirty` rather than through 100 real `step`s: `step` blocks in
  // `select!` when nothing happens, and the property under test is the gate,
  // not the scheduler.
  event_loop.dirty = false;
  for _ in 0..100 {
    event_loop.render_if_dirty()?;
  }
  assert_eq!(
    write_count(&event_loop),
    before,
    "a clean loop must not repaint"
  );

  // Dirty once => exactly one repaint, and the flag is consumed.
  event_loop.dirty = true;
  event_loop.render_if_dirty()?;
  assert_eq!(write_count(&event_loop), before + 1);
  event_loop.render_if_dirty()?;
  assert_eq!(
    write_count(&event_loop),
    before + 1,
    "the dirty flag must be reset by the repaint"
  );

  event_loop.shutdown()?;
  Ok(())
}

/// Shutting down waits for the write tasks that must not be interrupted.
///
/// This mirrors [`crate::evloop::Rsvim::process_cancellation_notify`]: the
/// detached tracker is closed without waiting, the blocked tracker is closed
/// and then awaited, so a write that is already in flight still lands.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn shutdown_waits_for_write() -> IoResult<()> {
  test_log_init();

  let src: &str = r#"
  const a = 1 + 1;
"#;
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop = make_event_loop(10, 10, CliOptions::empty());
  event_loop.initialize()?;

  let temp_dir = assert_fs::TempDir::new().unwrap();
  let path = temp_dir.child("written.txt").path().to_path_buf();
  let payload = "rsvim";

  event_loop.blocked_tracker.spawn_blocking(move || {
    std::thread::sleep(Duration::from_millis(10));
    std::fs::write(&path, payload).unwrap();
  });

  // The tracker is non-empty, so `wait()` must actually wait instead of
  // returning immediately.
  assert!(!event_loop.blocked_tracker.is_empty());
  tokio::time::timeout(
    Duration::from_secs(5),
    event_loop.process_cancellation_notify(),
  )
  .await
  .expect("shutdown must not hang");

  let written =
    std::fs::read_to_string(temp_dir.child("written.txt").path()).unwrap();
  assert_eq!(written, payload, "the in-flight write must be completed");

  event_loop.shutdown()?;
  Ok(())
}

/// A pending timer must not keep the process alive.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn exit_with_pending_timer() -> IoResult<()> {
  test_log_init();

  let src: &str = r#"
  setTimeout(() => { cmd.echo('never'); }, 10000);
"#;
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop = make_event_loop(10, 10, CliOptions::empty());
  event_loop.initialize()?;

  let started = std::time::Instant::now();
  let exit_code = tokio::time::timeout(
    Duration::from_millis(500),
    event_loop.run_with_mock_events(MockEventReader::new(vec![])),
  )
  .await
  .expect("the loop must not wait for a 10s timer")?;
  assert_eq!(exit_code, 0);
  assert!(
    started.elapsed() < Duration::from_millis(500),
    "the loop took {:?}",
    started.elapsed()
  );

  assert!(
    !cmdline_messages(&event_loop)
      .iter()
      .any(|m| m.contains("never")),
    "the pending timer's callback must not run on the way out"
  );

  event_loop.shutdown()?;
  Ok(())
}

/// The production runtime is multi-threaded, every other test is not.
///
/// Not a semantic test: it only proves the event loop works on the flavour
/// `rsvim_cli` actually builds (`tokio::runtime::Runtime::new()`).
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
#[cfg_attr(miri, ignore)]
async fn multi_thread_runtime_smoke() -> IoResult<()> {
  test_log_init();

  let src: &str = r#"
  import cmd from "rsvim:cmd";
  cmd.echo('hello from multi-thread');
"#;
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop = make_event_loop(10, 10, CliOptions::empty());
  event_loop.initialize()?;
  event_loop
    .run_with_mock_events(MockEventReader::new(vec![MockEvent::SleepFor(
      Duration::from_millis(20),
    )]))
    .await?;

  assert!(
    cmdline_messages(&event_loop)
      .iter()
      .any(|m| m.contains("hello from multi-thread")),
    "got: {:?}",
    cmdline_messages(&event_loop)
  );

  event_loop.shutdown()?;
  Ok(())
}

/// A write that was already promised to the user survives an immediate quit.
///
/// Stage-7 regression, the reason `FsWriteReq` runs on `blocked_tracker`
/// instead of `detached_tracker`: `await file.write(buf)` resolves only after
/// the bytes hit the file, but the process can be asked to quit in the very
/// next tick (Ctrl-D here). Work parked on `detached_tracker` is closed and
/// *not* waited for, so the tail of the file used to vanish silently.
///
/// The script deliberately does not `await` the write: the request is queued
/// into `master_rx` while the config module is still being evaluated, so the
/// loop sees `[B]` (master) before `[C]` (Ctrl-D) on its very first iteration.
/// That makes "the write is in flight when the loop exits" deterministic
/// instead of a race against a background thread.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn write_survives_immediate_exit() -> IoResult<()> {
  test_log_init();

  let tmpfile =
    assert_fs::NamedTempFile::new("write-survives-exit.txt").unwrap();
  let src = format!(
    r#"
  import fs from "rsvim:fs";
  const f = fs.openSync({:?}, {{create:true,write:true}});
  const buf = new TextEncoder().encode("Hello World");
  f.write(buf);
    "#,
    tmpfile.path()
  );
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

  let mut event_loop = make_event_loop(10, 10, CliOptions::empty());
  event_loop.initialize()?;

  // `MockEventReader::new(vec![])` still appends a `CTRL_D`, i.e. "quit right
  // now". Anything the write needed must therefore happen on the way out.
  event_loop
    .run_with_mock_events(MockEventReader::new(vec![]))
    .await?;
  event_loop.shutdown()?;

  assert_eq!(
    std::fs::read_to_string(tmpfile.path()).unwrap(),
    "Hello World".to_string(),
    "the quit path must wait for a write that was already promised"
  );

  Ok(())
}

/// A configuration stuck on a top-level `await` still lets the user quit.
///
/// The promise never settles, so the module waits forever on ticks that will
/// never come. `await new Promise(() => {})` really does leave the module
/// pending: v8 still reports `ModuleStatus::Evaluated` (evaluation returned a
/// promise), so the only reliable signal is
/// `Module::get_stalled_top_level_await_message`.
///
/// Node and Deno both let the process exit here. Exiting silently would hide
/// the fact that the rest of the file never ran, so `shutdown_js` names the
/// module once instead of blocking. Both halves are asserted: the loop must
/// not hang, and the module must still be reported as pending.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn exit_diagnoses_pending_top_level_await() -> IoResult<()> {
  test_log_init();

  let src: &str = r#"
  await new Promise(() => {});
"#;
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop = make_event_loop(10, 10, CliOptions::empty());
  event_loop.initialize()?;

  {
    let context = event_loop.kernel.borrow().context.clone();
    v8::scope_with_context!(scope, &mut event_loop.isolate, context);
    let pending = event_loop.kernel.borrow().module_map.pending_paths(scope);
    assert_eq!(pending.len(), 1, "got: {pending:?}");
    assert!(pending[0].ends_with("rsvim.js"), "got: {pending:?}");
  }

  let started = std::time::Instant::now();
  let exit_code = tokio::time::timeout(
    Duration::from_millis(500),
    event_loop.run_with_mock_events(MockEventReader::new(vec![])),
  )
  .await
  .expect("an unsettled top-level await must not keep the loop alive")?;
  assert_eq!(exit_code, 0);
  assert!(
    started.elapsed() < Duration::from_millis(500),
    "the loop took {:?}",
    started.elapsed()
  );

  event_loop.shutdown()?;
  Ok(())
}

/// A self-sustaining chain runs to completion inside a single tick.
///
/// This is the whole point of v2: `TickOutcome::MoreWork` no longer needs a
/// budget, a retry timer, or a wake-up, because Phase A keeps going until the
/// runtime is quiet. The chain here is simulated with a [`crate::js::JsFuture`]
/// that re-queues itself -- the exact shape of a recursive microtask chain --
/// and it is allowed to run a thousand times before stopping.
///
/// Re-introducing a self-driving budget would cap this at that budget, so the
/// assertion below is what pins the "no bound" behaviour down. See
/// `evloop-plan-v2.md` §2 and §7.4.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn bounded_chain_runs_to_completion_in_one_tick() -> IoResult<()> {
  use std::cell::Cell;

  test_log_init();

  /// How many rounds the chain is allowed to run. Two orders of magnitude above
  /// any budget that ever existed here, small enough to keep the test fast.
  const LIMIT: usize = 1000;

  struct SelfQueueing {
    kernel: crate::js::JsKernelRc,
    runs: std::rc::Rc<Cell<usize>>,
    limit: usize,
  }

  impl crate::js::JsFuture for SelfQueueing {
    fn run(&mut self, _scope: &mut v8::PinScope) {
      self.runs.set(self.runs.get() + 1);
      if self.runs.get() >= self.limit {
        return;
      }
      // `run_ready_futures` takes the vector before running anything, so the
      // borrow is free here and this lands in the next round.
      self
        .kernel
        .borrow_mut()
        .pending_futures
        .push(Box::new(SelfQueueing {
          kernel: self.kernel.clone(),
          runs: self.runs.clone(),
          limit: self.limit,
        }));
    }
  }

  let src: &str = r#"
  const a = 1 + 1;
"#;
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop = make_event_loop(10, 10, CliOptions::empty());
  event_loop.initialize()?;

  let runs = std::rc::Rc::new(Cell::new(0));
  event_loop
    .kernel
    .borrow_mut()
    .pending_futures
    .push(Box::new(SelfQueueing {
      kernel: event_loop.kernel.clone(),
      runs: runs.clone(),
      limit: LIMIT,
    }));

  // One synchronous call, no `select!`, no sleeping: this is Phase A itself.
  let did_work = event_loop.tick_event_loop();
  event_loop.shutdown()?;

  assert!(did_work, "running the chain is work");
  assert_eq!(
    runs.get(),
    LIMIT,
    "the chain must run to completion in a single tick, not stop at a budget"
  );

  Ok(())
}

/// A request the config script issues is served before the loop waits.
///
/// `process.exit` is the cheapest such request: it never becomes a background
/// task, it just lands on `master_rx`. Phase A's `settle_master` therefore has
/// to drain it, notice the cancellation, and break out -- all before Phase B
/// ever polls an event source.
///
/// The probe below makes that ordering observable: if the loop reaches the
/// `select!`, the probe's `poll_next` runs. So a false `polled` flag is proof
/// that the request was served inside the tick that produced it, not by
/// winning a race against terminal input.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn master_request_is_served_in_the_same_tick() -> IoResult<()> {
  use std::cell::Cell;
  use std::rc::Rc;

  test_log_init();

  /// Records whether Phase B ever asked for an event.
  struct ProbeReader {
    polled: Rc<Cell<bool>>,
  }

  impl futures::Stream for ProbeReader {
    type Item = IoResult<Event>;

    fn poll_next(
      self: std::pin::Pin<&mut Self>,
      _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
      self.polled.set(true);
      std::task::Poll::Pending
    }
  }

  let src: &str = r#"
  import process from "rsvim:process";
  process.exit(7);
  "#;
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop = make_event_loop(10, 10, CliOptions::empty());
  event_loop.initialize()?;

  // The module ran during `initialize`, so the request is already queued and
  // nothing has consumed it yet.
  assert_eq!(event_loop.exit_code, 0, "nothing has been served yet");
  assert!(!event_loop.cancellation_token.is_cancelled());

  let polled = Rc::new(Cell::new(false));
  let mut io = ProbeReader {
    polled: polled.clone(),
  };
  let noop = |_: &mut crate::evloop::Rsvim, _: Option<IoResult<Event>>| {};

  let flow = event_loop.tick(&mut io, &is_ctrl_d, &noop).await?;
  event_loop.shutdown()?;

  assert!(
    flow.is_break(),
    "an exit request must end the loop, not survive into Phase B",
  );
  assert_eq!(
    event_loop.exit_code, 7,
    "the exit code carried by the request must be adopted",
  );
  assert!(
    !polled.get(),
    "Phase A must settle the request before the loop waits on any event source",
  );

  Ok(())
}
