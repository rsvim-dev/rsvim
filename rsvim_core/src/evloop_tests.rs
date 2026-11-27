//! Tests for the event loop's javascript tick semantics.

use crate::chan::ExCommandReq;
use crate::chan::JsMessage;
use crate::cli::CliOptions;
use crate::evloop::Rsvim;
use crate::evloop::is_ctrl_d;
use crate::evloop::writer::StdoutWriterValue;
use crate::prelude::*;
use crate::tests::evloop::*;
use crate::tests::log::init as test_log_init;
use crate::ui::tree::internal::inode::Inodify;
use assert_fs::prelude::*;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyModifiers;
use crossterm::event::MouseEvent;
use crossterm::event::MouseEventKind;
use futures::StreamExt;
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
  Promise.reject(new Error('boom'));
"#;
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop = make_event_loop(10, 10, CliOptions::empty());
  event_loop.initialize()?;

  // Evaluating the config entered javascript, which drained the job queue and
  // reported the rejection, so it has been reported by now.
  //
  // Two more ticks must be idle and must not report the error again: a tick
  // that keeps reporting the same rejection would mean the engine still holds
  // it, i.e. the loop would never go quiet.
  assert!(!event_loop.tick_event_loop());
  assert!(!event_loop.tick_event_loop());

  let messages = cmdline_messages(&event_loop);
  let reported: Vec<&String> = messages
    .iter()
    .filter(|m| m.contains("Uncaught (in promise)") && m.contains("boom"))
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
    assert!(
      messages
        .iter()
        .any(|m| m.contains("Uncaught (in promise)") && m.contains(expected)),
      "expected the {expected:?} rejection to be reported, got: {messages:?}"
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

  assert!(
    !event_loop.tick_event_loop(),
    "a config with nothing pending must leave the tick idle"
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

/// A timer is an entry in the host's timer table, not a message on a channel.
///
/// The v8 backend needed three hops for this: `global_create_timer` parked the
/// callback in `pending_timers`, `MasterMessage::TimeoutReq` asked the loop to
/// wake it, and the tick popped it out of `pending_futures`. Now the loop looks
/// at the table itself: a due timer is one of the events
/// [`crate::evloop::Rsvim::tick_event_loop`] handles.
///
/// So the property to pin down is "a tick runs a due timer without anyone
/// sending a message".
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn a_due_timer_runs_without_anyone_sending_a_message() -> IoResult<()> {
  test_log_init();

  let src: &str = r#"
  import cmd from "rsvim:cmd";
  setTimeout(() => { cmd.echo("fired"); }, 1);
"#;
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop = make_event_loop(10, 10, CliOptions::empty());
  event_loop.initialize()?;

  assert!(
    !cmdline_messages(&event_loop)
      .iter()
      .any(|m| m.contains("fired")),
    "the timer must not have fired yet"
  );

  // Drive the loop by hand. Nothing is written to `jsrt_tx` here: the only
  // thing that happens is the timer coming due.
  let deadline = std::time::Instant::now() + Duration::from_secs(5);
  while !cmdline_messages(&event_loop)
    .iter()
    .any(|m| m.contains("fired"))
  {
    assert!(
      std::time::Instant::now() < deadline,
      "the timer never fired, got: {:?}",
      cmdline_messages(&event_loop)
    );
    event_loop.tick_event_loop();
    tokio::time::sleep(Duration::from_millis(2)).await;
  }

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
  let moved = event_loop.tick_event_loop();
  assert!(!moved, "an idle tick reports that nothing moved");
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
/// This is [`crate::evloop::Rsvim::close_trackers`], which every exit path runs: the
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
  tokio::time::timeout(Duration::from_secs(5), event_loop.close_trackers())
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
/// The promise never settles, so the module waits forever on work that will
/// never come. Node and Deno both let the process exit here, and so does
/// rsvim: the unfinished module is not a reason to keep the window open.
///
/// The v8 backend could name the stalled module
/// (`Module::get_stalled_top_level_await_message` + a module registry).
/// QuickJS exposes neither, so the diagnosis is gone and this test only pins
/// down the property that actually matters to the user -- the loop must not
/// hang on it.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn exit_does_not_wait_for_a_pending_top_level_await() -> IoResult<()> {
  test_log_init();

  let src: &str = r#"
  await new Promise(() => {});
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

/// A self-sustaining microtask chain runs to completion inside a single tick.
///
/// There is no budget on the job queue: `JsHost::enter` drains it until it is
/// empty. A chain that keeps re-queuing itself therefore runs to its natural end
/// instead of stopping at an arbitrary round count.
///
/// The chain is plain javascript -- `Promise.resolve().then(loop)` -- so the
/// work lives on the engine's own job queue. (The loop's own budget, see
/// `SETTLE_MAX_EVENTS`, counts events, not jobs.)
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn bounded_chain_runs_to_completion_in_one_tick() -> IoResult<()> {
  test_log_init();

  /// How many rounds the chain runs. Two orders of magnitude above any budget
  /// that ever existed here, small enough to keep the test fast.
  const LIMIT: usize = 1000;

  let src: &str = r#"
  import cmd from "rsvim:cmd";
  let runs = 0;
  function step() {
    runs += 1;
    if (runs >= 1000) {
      cmd.echo(`runs:${runs}`);
      return;
    }
    Promise.resolve().then(step);
  }
  step();
"#;
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop = make_event_loop(10, 10, CliOptions::empty());
  event_loop.initialize()?;

  // The module's `enter` drained the job queue, so the chain is finished by
  // now. What matters is that it finished at its natural end instead of being
  // cut short, and that the loop is idle afterwards -- a leftover budget would
  // leave work queued here.
  let did_work = event_loop.tick_event_loop();
  event_loop.shutdown()?;

  assert!(
    !did_work,
    "the chain must already be drained, otherwise the previous tick stopped early"
  );
  let messages = cmdline_messages(&event_loop);
  assert!(
    messages
      .iter()
      .any(|m| m.contains(&format!("runs:{LIMIT}"))),
    "the chain must run to completion in a single tick, not stop at a budget; got {messages:?}"
  );

  Ok(())
}

/// A request the config script issues is served before the loop waits.
///
/// `process.exit` is the cheapest such request: it never becomes a background
/// task, it just lands on `master_rx`. The settle step of the tick therefore
/// has to drain it, notice the cancellation, and break out -- all before the
/// loop ever polls an event source.
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

  /// Records whether the loop ever asked for a terminal event.
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
  let noop = |_: &mut crate::evloop::Rsvim, _: Option<IoResult<Event>>| false;

  let flow = event_loop.tick(&mut io, &is_ctrl_d, &noop).await?;
  event_loop.shutdown()?;

  assert!(
    flow.is_break(),
    "an exit request must end the loop, not survive into the wait",
  );
  assert_eq!(
    event_loop.exit_code, 7,
    "the exit code carried by the request must be adopted",
  );
  assert!(
    !polled.get(),
    "the settle step must serve the request before the loop waits on any event source",
  );

  Ok(())
}

/// A timer that fires while the loop is waiting is noticed and run.
///
/// This is the property the v8 backend needed `v8::Platform::pump_message_loop`
/// for: a finished background task never called back into the embedder, so the
/// loop had to poke the platform's foreground queue or the promise would stay
/// pending forever. Here the loop waits on the timer table, so a due timer
/// wakes it.
///
/// The script arms a short timer and then does nothing, so the only way
/// `timer` can ever be echoed is the wait waking up for the timer.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn timer_fires_while_the_loop_waits() -> IoResult<()> {
  test_log_init();

  let src: &str = r#"
  import cmd from "rsvim:cmd";
  setTimeout(() => { cmd.echo("timer"); }, 20);
  cmd.echo("armed");
"#;
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop = make_event_loop(10, 10, CliOptions::empty());
  event_loop.initialize()?;

  let started = std::time::Instant::now();
  event_loop
    .run_with_mock_events(MockEventReader::new(vec![MockEvent::SleepFor(
      Duration::from_millis(200),
    )]))
    .await?;

  let messages = cmdline_messages(&event_loop);
  assert!(
    messages.iter().any(|m| m.contains("armed")),
    "the module itself must have run, got {messages:?}",
  );
  assert!(
    messages.iter().any(|m| m.contains("timer")),
    "the timer must be advanced by the engine scheduler, got {messages:?} after {:?}",
    started.elapsed(),
  );

  event_loop.shutdown()?;
  Ok(())
}

// ---------------------------------------------------------------------------
// The loop's shape: what one event costs, and what it cannot starve.
// ---------------------------------------------------------------------------

/// A terminal that delivers `items`, then never says anything again.
///
/// `tick` blocks waiting for an event, so the tests that drive it by hand need
/// a source that goes quiet instead of ending.
fn scripted(
  items: Vec<IoResult<Event>>,
) -> impl futures::Stream<Item = IoResult<Event>> + Unpin {
  futures::stream::iter(items).chain(futures::stream::pending())
}

fn key(c: char) -> IoResult<Event> {
  Ok(Event::Key(KeyEvent::new(
    KeyCode::Char(c),
    KeyModifiers::NONE,
  )))
}

/// A timer callback must leave the editor marked dirty.
///
/// The callback used to run inside a future's `poll`, where the loop could not
/// see it: `cmd.echo("fired")` landed in the message history, but nothing asked
/// for a repaint, so the line did not appear until the next key press.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn timer_callback_marks_the_editor_dirty() -> IoResult<()> {
  test_log_init();

  let src: &str = r#"
  import cmd from "rsvim:cmd";
  setTimeout(() => { cmd.echo("fired"); }, 5);
"#;
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop = make_event_loop(10, 10, CliOptions::empty());
  event_loop.initialize()?;
  event_loop.dirty = false;

  let mut io = scripted(vec![]);
  let noop = |_: &mut Rsvim, _: Option<IoResult<Event>>| false;
  let _ = tokio::time::timeout(
    Duration::from_secs(5),
    event_loop.tick(&mut io, &is_ctrl_d, &noop),
  )
  .await
  .expect("the timer must wake the loop")?;

  assert!(
    cmdline_messages(&event_loop)
      .iter()
      .any(|m| m.contains("fired"))
  );
  assert!(
    event_loop.dirty,
    "a timer callback may have changed the editor, so the screen is stale"
  );

  let before = write_count(&event_loop);
  event_loop.render_if_dirty()?;
  assert_eq!(write_count(&event_loop), before + 1);

  event_loop.shutdown()?;
  Ok(())
}

/// The same for the completion of an asynchronous operation.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn finished_operation_marks_the_editor_dirty() -> IoResult<()> {
  test_log_init();

  let data = assert_fs::NamedTempFile::new("loop-completion.txt").unwrap();
  std::fs::write(data.path(), "payload").unwrap();
  let src = format!(
    r#"
  import cmd from "rsvim:cmd";
  import fs from "rsvim:fs";
  fs.readTextFile({:?}).then((text) => cmd.echo("read:" + text));
"#,
    data.path()
  );
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

  let mut event_loop = make_event_loop(10, 10, CliOptions::empty());
  event_loop.initialize()?;
  event_loop.dirty = false;

  let mut io = scripted(vec![]);
  let noop = |_: &mut Rsvim, _: Option<IoResult<Event>>| false;
  let _ = tokio::time::timeout(
    Duration::from_secs(5),
    event_loop.tick(&mut io, &is_ctrl_d, &noop),
  )
  .await
  .expect("the completion must wake the loop")?;

  assert!(
    cmdline_messages(&event_loop)
      .iter()
      .any(|m| m.contains("read:payload")),
    "got: {:?}",
    cmdline_messages(&event_loop)
  );
  assert!(event_loop.dirty);

  event_loop.shutdown()?;
  Ok(())
}

/// One event costs one frame.
///
/// Pressing Enter on `:probe` is one terminal event, but the command runs in
/// the *next* iteration (the state machine only queues it). Repainting right
/// after the key would draw "command line closed, command not run yet" and then
/// draw again once the command ran; the repaint belongs after the settle step.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn an_ex_command_costs_one_frame() -> IoResult<()> {
  test_log_init();

  let src: &str = r#"
  import cmd from "rsvim:cmd";
  cmd.create("probe", () => { cmd.echo("probe ran"); });
"#;
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop = make_event_loop(10, 10, CliOptions::empty());
  event_loop.initialize()?;
  event_loop.dirty = false;
  let before = write_count(&event_loop);

  // What the state machine does on Enter: queue the command for javascript.
  let enter = |rsvim: &mut Rsvim, _: Option<IoResult<Event>>| {
    let (current_buf_id, current_win_id) = {
      let tree = lock!(rsvim.editor.tree);
      let window = tree.current_window().unwrap();
      let buffer = window.buffer().upgrade().unwrap();
      let buf_id = lock!(buffer).id();
      (buf_id, window.id())
    };
    rsvim
      .jsrt_tx
      .send(JsMessage::ExCommandReq(ExCommandReq {
        payload: "probe".into(),
        current_buf_id,
        current_win_id,
      }))
      .unwrap();
    true
  };

  let mut io = scripted(vec![key('x')]);

  // Iteration 1 handles the key and returns.
  let _ = event_loop.tick(&mut io, &is_ctrl_d, &enter).await?;
  assert_eq!(
    write_count(&event_loop),
    before,
    "no frame between the key and the command it queued"
  );

  // Iteration 2 settles the command, repaints once, then waits for input.
  let waited = tokio::time::timeout(
    Duration::from_millis(100),
    event_loop.tick(&mut io, &is_ctrl_d, &enter),
  )
  .await;
  assert!(waited.is_err(), "the loop must be waiting for input by now");

  assert!(
    cmdline_messages(&event_loop)
      .iter()
      .any(|m| m.contains("probe ran")),
    "got: {:?}",
    cmdline_messages(&event_loop)
  );
  assert_eq!(
    write_count(&event_loop),
    before + 1,
    "exactly one frame for one event"
  );

  event_loop.shutdown()?;
  Ok(())
}

/// A burst of keys is applied before the screen is repainted once.
///
/// Without bracketed paste a paste arrives as one key event per character, and
/// each used to cost a full iteration and a repaint.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn a_burst_of_keys_is_one_iteration() -> IoResult<()> {
  test_log_init();

  let _tp = make_configs(vec![(Path::new("rsvim.js"), "const a = 1;")]);
  let mut event_loop = make_event_loop(10, 10, CliOptions::empty());
  event_loop.initialize()?;
  event_loop.dirty = false;
  let before = write_count(&event_loop);

  let handled = std::cell::Cell::new(0_usize);
  let on_input = |_: &mut Rsvim, _: Option<IoResult<Event>>| {
    handled.set(handled.get() + 1);
    true
  };
  let mut io = scripted((0..10).map(|_| key('x')).collect());

  let _ = event_loop.tick(&mut io, &is_ctrl_d, &on_input).await?;

  assert_eq!(
    handled.get(),
    10,
    "the whole burst belongs to one iteration"
  );
  assert_eq!(
    write_count(&event_loop),
    before,
    "and nothing is repainted yet"
  );
  assert!(event_loop.dirty);

  event_loop.shutdown()?;
  Ok(())
}

/// Mouse movement does not repaint.
///
/// `EnableMouseCapture` asks the terminal to report every movement of the mouse
/// (`?1003h`), and the state machines ignore all of it, so repainting on every
/// event repainted on every twitch.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn mouse_movement_does_not_mark_the_editor_dirty() -> IoResult<()> {
  test_log_init();

  let _tp = make_configs(vec![(Path::new("rsvim.js"), "const a = 1;")]);
  let mut event_loop = make_event_loop(10, 10, CliOptions::empty());
  event_loop.initialize()?;
  event_loop.dirty = false;

  let mut io = scripted(vec![Ok(Event::Mouse(MouseEvent {
    kind: MouseEventKind::Moved,
    column: 3,
    row: 4,
    modifiers: KeyModifiers::NONE,
  }))]);
  let _ = event_loop
    .tick(&mut io, &is_ctrl_d, &Rsvim::process_event)
    .await?;
  assert!(!event_loop.dirty, "a mouse move changes nothing on screen");

  // A key that does something still does.
  let mut io = scripted(vec![key('i')]);
  let _ = event_loop
    .tick(&mut io, &is_ctrl_d, &Rsvim::process_event)
    .await?;
  assert!(event_loop.dirty, "entering insert mode does");

  event_loop.shutdown()?;
  Ok(())
}

/// A source that never runs dry cannot starve the terminal.
///
/// 600 timers are all due at once. One settle step handles a bounded number of
/// events and returns, so the loop gets back to waiting on terminal input; what
/// is left is not lost -- it stays in the timer table, and the next step takes
/// it.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn a_flood_of_due_timers_is_taken_in_bounded_steps() -> IoResult<()> {
  test_log_init();

  let src: &str = r#"
  globalThis.fired = 0;
  for (let i = 0; i < 600; i++) setTimeout(() => { globalThis.fired++; }, 1);
"#;
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);
  let mut event_loop = make_event_loop(10, 10, CliOptions::empty());
  event_loop.initialize()?;

  tokio::time::sleep(Duration::from_millis(20)).await;

  let fired = |event_loop: &mut crate::evloop::EventLoop| -> i32 {
    event_loop
      .js
      .enter(|ctx| ctx.eval("globalThis.fired").unwrap())
  };

  assert!(event_loop.tick_event_loop());
  let first = fired(&mut event_loop);
  assert!(
    0 < first && first < 600,
    "one step must stop before the flood is over, handled {first}"
  );

  let mut steps = 1;
  while fired(&mut event_loop) < 600 {
    assert!(event_loop.tick_event_loop(), "nothing may be stranded");
    steps += 1;
    assert!(steps < 100);
  }
  assert!(!event_loop.tick_event_loop(), "and then it is quiet");

  event_loop.shutdown()?;
  Ok(())
}

/// Dropping the event loop while a command is registered must not abort the
/// process.
///
/// A command holds a `Persistent` handle, and QuickJS aborts in `JS_FreeRuntime`
/// if one outlives its runtime. Clearing the registry used to be a step of an
/// orderly shutdown; an early `?` return, or a test that never runs the loop,
/// skipped it. It is now tied to ownership.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn dropping_the_loop_with_a_registered_command_does_not_abort()
-> IoResult<()> {
  test_log_init();

  let src: &str = r#"
  import cmd from "rsvim:cmd";
  cmd.create("probe", () => {});
  setTimeout(() => {}, 60000);
"#;
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop = make_event_loop(10, 10, CliOptions::empty());
  event_loop.initialize()?;
  assert_eq!(lock!(event_loop.editor.command_manager).len(), 1);

  // No `run`, no `shutdown`: just drop.
  drop(event_loop);
  Ok(())
}

/// Behaves like crossterm's `EventStream` where it matters here: when nothing
/// is ready it remembers the waker of *that* poll, wakes only that one when
/// input arrives, and registers no other while one is outstanding.
///
/// A stream that wakes whoever polled it last would hide the bug this exists to
/// catch, which is also why none of the other mocked streams can.
#[derive(Clone)]
struct OneWakerStream {
  inner: std::sync::Arc<std::sync::Mutex<OneWakerInner>>,
}

struct OneWakerInner {
  queue: std::collections::VecDeque<IoResult<Event>>,
  waker: Option<std::task::Waker>,
  registered: bool,
}

impl OneWakerStream {
  fn new() -> Self {
    Self {
      inner: std::sync::Arc::new(std::sync::Mutex::new(OneWakerInner {
        queue: Default::default(),
        waker: None,
        registered: false,
      })),
    }
  }

  /// What the terminal does when the user presses a key.
  fn feed(&self, event: IoResult<Event>) {
    let mut inner = self.inner.lock().unwrap();
    inner.queue.push_back(event);
    if let Some(waker) = inner.waker.take() {
      inner.registered = false;
      waker.wake();
    }
  }
}

impl futures::Stream for OneWakerStream {
  type Item = IoResult<Event>;

  fn poll_next(
    self: std::pin::Pin<&mut Self>,
    cx: &mut std::task::Context<'_>,
  ) -> std::task::Poll<Option<Self::Item>> {
    let mut inner = self.inner.lock().unwrap();
    if let Some(event) = inner.queue.pop_front() {
      return std::task::Poll::Ready(Some(event));
    }
    if !inner.registered {
      inner.registered = true;
      inner.waker = Some(cx.waker().clone());
    }
    std::task::Poll::Pending
  }
}

/// Taking a burst of keys must not park the terminal behind a dead waker.
///
/// `drain_input` peeks at the stream after the first key. Peeking with a no-op
/// waker (`now_or_never`) makes crossterm's `EventStream` remember that waker;
/// the key pressed next wakes it, nobody listens, and the editor stops reacting
/// to the keyboard until something else -- a timer -- happens to wake the loop.
/// Real terminals only: the other mocked streams wake whoever polled last.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn a_key_after_a_burst_still_wakes_the_loop() -> IoResult<()> {
  test_log_init();

  let _tp = make_configs(vec![(Path::new("rsvim.js"), "const a = 1;")]);
  let mut event_loop = make_event_loop(10, 10, CliOptions::empty());
  event_loop.initialize()?;

  let mut io = OneWakerStream::new();
  let feeder = io.clone();
  let keyboard = std::thread::spawn(move || {
    std::thread::sleep(Duration::from_millis(30));
    feeder.feed(key('x'));
    std::thread::sleep(Duration::from_millis(100));
    feeder.feed(key('y'));
  });

  let handled = std::cell::Cell::new(0_usize);
  let on_input = |_: &mut Rsvim, _: Option<IoResult<Event>>| {
    handled.set(handled.get() + 1);
    true
  };

  for pressed in 1..=2 {
    let started = std::time::Instant::now();
    // `timeout` polls its future once more when it expires, so a loop that was
    // never woken would still "succeed" after the full wait. Time it instead.
    let _ = tokio::time::timeout(
      Duration::from_secs(3),
      event_loop.tick(&mut io, &is_ctrl_d, &on_input),
    )
    .await
    .expect("the loop must handle the key");
    assert!(
      started.elapsed() < Duration::from_secs(1),
      "key #{pressed} took {:?} to reach the loop: the stream is parked behind a waker nobody listens to",
      started.elapsed()
    );
  }
  assert_eq!(handled.get(), 2);

  keyboard.join().unwrap();
  event_loop.shutdown()?;
  Ok(())
}
