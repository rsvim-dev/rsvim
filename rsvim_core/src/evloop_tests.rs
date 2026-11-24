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

/// A stream that never yields an item.
///
/// Lets a single [`crate::evloop::Rsvim::step`] be driven deterministically by
/// the javascript channel: the `[C]` input branch stays pending, so the test
/// observes exactly the branch it cares about instead of racing the mock
/// reader's background thread.
struct PendingReader;

impl futures::Stream for PendingReader {
  type Item = IoResult<Event>;

  fn poll_next(
    self: std::pin::Pin<&mut Self>,
    _cx: &mut std::task::Context<'_>,
  ) -> std::task::Poll<Option<Self::Item>> {
    std::task::Poll::Pending
  }
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
/// added a whole select round-trip. Now the completion itself advances the
/// runtime, in place, once.
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

  let mut io = PendingReader;
  let noop = |_: &mut crate::evloop::Rsvim, _: Option<IoResult<Event>>| {};

  // `select!` is deliberately not `biased` (a biased loop lets a busy js
  // channel starve terminal input), so which ready branch wins is random. The
  // `MasterMessage::TimeoutReq` queuea by `initialize` is ready too, and
  // processing it only spawns a 10s sleep. Step until the completion has been
  // observed, asserting that it never took more than a handful of rounds.
  let mut rounds = 0;
  while event_loop
    .kernel
    .borrow()
    .pending_timers
    .contains_key(&timer_id)
  {
    rounds += 1;
    assert!(rounds <= 10, "the completion never drove a tick");
    let flow = event_loop.step(&mut io, &is_ctrl_d, &noop).await?;
    assert!(flow.is_continue(), "the loop must not exit on a tick");
  }

  assert_eq!(
    event_loop.js_spin, 0,
    "handling a completion must not self-drive extra ticks"
  );
  assert!(!event_loop.js_tick_again);

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

/// Exhausting the self-drive budget must not strand the remaining work.
///
/// P0-2 regression. `TickOutcome::MoreWork` means `pending_futures` is still
/// non-empty. Before the fix, the budget-exhausted path only cleared
/// `js_tick_again`, so a self-sustaining chain (each future queueing the next)
/// had no new message and no wake-up coming: it stopped forever, silently, with
/// the UI perfectly responsive. Now exhaustion schedules exactly one delayed
/// `js_wake`, which the `[E]` branch consumes and which resets the budget.
///
/// The chain is simulated with a `JsFuture` that re-queues itself -- the exact
/// shape of a recursive microtask chain. If the retry were missing, the loop
/// would run at most `JS_SPIN_BUDGET + 1` futures and then stall until the
/// reader's `CTRL_D`; the assertion below therefore fails without the fix.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn exhausted_budget_wakes_instead_of_stranding() -> IoResult<()> {
  use std::cell::Cell;

  test_log_init();

  struct SelfQueueing {
    kernel: crate::js::JsKernelRc,
    runs: std::rc::Rc<Cell<usize>>,
  }

  impl crate::js::JsFuture for SelfQueueing {
    fn run(&mut self, _scope: &mut v8::PinScope) {
      self.runs.set(self.runs.get() + 1);
      // `run_ready_futures` takes the vector before running anything, so the
      // borrow is free here and this lands in the next tick.
      self
        .kernel
        .borrow_mut()
        .pending_futures
        .push(Box::new(SelfQueueing {
          kernel: self.kernel.clone(),
          runs: self.runs.clone(),
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
    }));
  event_loop.js_tick_again = true;

  // The reader sleeps, so the loop has time to exhaust its budget at least
  // once, then appends a `CTRL_D` and lets the test finish.
  event_loop
    .run_with_mock_events(MockEventReader::new(vec![MockEvent::SleepFor(
      Duration::from_millis(50),
    )]))
    .await?;
  event_loop.shutdown()?;

  assert!(
    runs.get() > crate::evloop::Rsvim::JS_SPIN_BUDGET * 2,
    "the chain must keep making progress across budget bursts, got {} runs \
     (budget is {})",
    runs.get(),
    crate::evloop::Rsvim::JS_SPIN_BUDGET
  );

  Ok(())
}
