//! Tests for the javascript host: the single entrance, the timer table and the
//! operation table.
//!
//! Each test pins down one property the host exists to guarantee; the comments
//! say which defect of the previous design it was written against.

use crate::js::JsRuntimeOptions;
use crate::prelude::*;
use crate::tests::js::make_js_host;
use crate::tests::js::make_js_host_with;
use crate::tests::log::init as test_log_init;
use std::time::Duration;
use std::time::Instant;

fn ms(n: u64) -> Duration {
  Duration::from_millis(n)
}

// ------------------------------------------------------------------ the entrance

/// Two timers due at the same instant: the microtasks the first one queued run
/// before the second one starts. Node, Deno, Dune, LLRT and txiki.js all print
/// `t1, m1, t2`; the previous design (every timer a spawned future) printed
/// `t1, t2, m1`.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn timers_drain_microtasks_between_callbacks() {
  test_log_init();
  let mut js = make_js_host();

  js.run(
    r#"
    globalThis.log = [];
    setTimeout(() => { log.push('t1'); Promise.resolve().then(() => log.push('m1')); }, 5);
    setTimeout(() => { log.push('t2'); }, 5);
    "#,
  );
  js.run_until(ms(2000), |js| js.eval::<i32>("log.length") == 3)
    .await;

  let log: Vec<String> = js.eval("log");
  assert_eq!(log, vec!["t1", "m1", "t2"]);
}

/// Timers that share a deadline fire in the order they were created.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn timers_with_the_same_delay_fire_in_creation_order() {
  test_log_init();
  let mut js = make_js_host();

  js.run(
    r#"
    globalThis.log = [];
    for (let i = 0; i < 20; i++) setTimeout(() => log.push(i), 3);
    "#,
  );
  js.run_until(ms(2000), |js| js.eval::<i32>("log.length") == 20)
    .await;

  let log: Vec<i32> = js.eval("log");
  assert_eq!(log, (0..20).collect::<Vec<i32>>());
}

/// Whatever javascript does, the host reports that the editor may have changed.
/// This is what makes a timer callback repaint the screen: the previous design
/// ran callbacks inside a future's `poll`, where nobody could set the flag.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn running_javascript_marks_the_host_dirty() {
  test_log_init();
  let mut js = make_js_host();
  assert!(!js.host.take_dirty(), "a fresh host is clean");

  js.run("setTimeout(() => {}, 5);");
  assert!(
    js.host.take_dirty(),
    "evaluating a script is a javascript entry"
  );
  assert!(!js.host.take_dirty(), "the flag is consumed");

  js.run_until(ms(2000), |js| js.host.ops().timers_len() == 0)
    .await;
  assert!(
    js.host.take_dirty(),
    "a timer callback is a javascript entry"
  );
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn a_finished_operation_marks_the_host_dirty() {
  test_log_init();
  let mut js = make_js_host();
  let file = assert_fs::NamedTempFile::new("host-dirty.txt").unwrap();
  std::fs::write(file.path(), "x").unwrap();
  let path = file.path().to_string_lossy().to_string();

  js.run(&format!(
    "__InternalRsvimGlobalObject.fs_read_text_file({path:?}).then(() => {{}});"
  ));
  js.host.take_dirty();
  js.run_until(ms(5000), |js| js.host.ops().pending_len() == 0)
    .await;
  assert!(
    js.host.take_dirty(),
    "settling a promise is a javascript entry"
  );
}

// ------------------------------------------------------------------ timers

/// `clearTimeout` releases the entry (and the callback it holds) at once. A
/// debounce -- clear, then set again, on every keystroke -- must leave one live
/// timer, not one per keystroke until the delay runs out.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn clear_timeout_releases_everything() {
  test_log_init();
  let mut js = make_js_host();

  js.run(
    r#"
    globalThis.id = 0;
    for (let i = 0; i < 1000; i++) {
      clearTimeout(id);
      id = setTimeout(() => {}, 300);
    }
    "#,
  );
  assert_eq!(js.host.ops().timers_len(), 1);

  js.run("clearTimeout(id);");
  assert_eq!(js.host.ops().timers_len(), 0);
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn interval_repeats_until_it_clears_itself() {
  test_log_init();
  let mut js = make_js_host();

  js.run(
    r#"
    globalThis.n = 0;
    globalThis.iv = setInterval(() => { n++; if (n === 3) clearInterval(iv); }, 5);
    "#,
  );
  js.run_until(ms(2000), |js| {
    js.eval::<i32>("n") == 3 && js.host.ops().timers_len() == 0
  })
  .await;
  js.run_for(ms(30)).await;

  assert_eq!(
    js.eval::<i32>("n"),
    3,
    "it must not fire after clearing itself"
  );
  assert_eq!(js.host.ops().timers_len(), 0);
}

/// A timer callback that throws is reported, and the loop carries on.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn a_throwing_timer_callback_is_reported() {
  test_log_init();
  let mut js = make_js_host();

  js.run("setTimeout(() => { throw new Error('timer boom'); }, 1);");
  js.run_until(ms(2000), |js| {
    js.messages().iter().any(|m| m.contains("timer boom"))
  })
  .await;

  let reported = js
    .messages()
    .into_iter()
    .filter(|m| m.contains("timer boom"))
    .count();
  assert_eq!(reported, 1, "got: {:?}", js.messages());
}

// ------------------------------------------------------------------ rejections

fn count(js: &crate::tests::js::TestJs, needle: &str) -> usize {
  js.messages().iter().filter(|m| m.contains(needle)).count()
}

/// An exception that an `async` caller catches with `try/catch` is not
/// "uncaught". QuickJS reports a rejection twice -- `handled == false` when it
/// happens, `true` when somebody attaches a handler -- and the previous design
/// reported on the first.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn handled_rejection_is_not_reported() {
  test_log_init();
  let mut js = make_js_host();

  js.run(
    r#"
    globalThis.caught = null;
    async function f() { throw new Error('handled-boom'); }
    async function main() {
      try { await f(); } catch (e) { globalThis.caught = e.message; }
    }
    main();
    "#,
  );

  assert_eq!(js.eval::<String>("caught"), "handled-boom");
  assert_eq!(count(&js, "handled-boom"), 0, "got: {:?}", js.messages());
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn unhandled_rejection_is_reported_exactly_once() {
  test_log_init();
  let mut js = make_js_host();

  js.run("Promise.reject(new Error('nobody-listens'));");
  // Further entries must not report it again.
  js.run("1 + 1;");
  js.run("1 + 1;");

  assert_eq!(count(&js, "Uncaught (in promise)"), 1);
  assert_eq!(count(&js, "nobody-listens"), 1);
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn every_unhandled_rejection_is_reported_once() {
  test_log_init();
  let mut js = make_js_host();

  js.run(
    r#"
    Promise.reject(new Error('first'));
    Promise.reject(new Error('second'));
    Promise.reject(new Error('third'));
    "#,
  );

  for name in ["first", "second", "third"] {
    assert_eq!(count(&js, name), 1, "{name}: {:?}", js.messages());
  }
}

/// A `catch` attached in the same turn cancels the report: rejections are
/// reported after the microtask queue is drained, not when they happen.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn a_late_catch_in_the_same_turn_cancels_the_report() {
  test_log_init();
  let mut js = make_js_host();

  js.run(
    r#"
    globalThis.seen = false;
    const p = Promise.reject(new Error('late-handled'));
    Promise.resolve().then(() => p.catch(() => { globalThis.seen = true; }));
    "#,
  );

  assert!(js.eval::<bool>("seen"));
  assert_eq!(count(&js, "late-handled"), 0, "got: {:?}", js.messages());
}

/// A module that throws at the top level fails twice in QuickJS (the module's
/// own promise, and the one the evaluation hands back). The caller of
/// `execute_module` reports the returned error, so nothing else may.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn a_throwing_module_is_reported_once_by_its_caller() {
  test_log_init();
  let mut js = make_js_host();

  let result = js
    .host
    .execute_module("boom.js", Some("throw new Error('top-level-boom');"));

  let Err(TheErr::JsError(err)) = result else {
    panic!("expected a javascript error, got {result:?}");
  };
  assert!(err.message.contains("top-level-boom"), "got: {err:?}");
  assert_eq!(
    js.messages().len(),
    0,
    "the host must not report what it returned: {:?}",
    js.messages()
  );
}

/// A module parked on a top-level `await` is not an error.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn a_module_waiting_on_top_level_await_is_not_an_error() {
  test_log_init();
  let mut js = make_js_host();

  let result = js
    .host
    .execute_module("waits.js", Some("await new Promise(() => {});"));
  assert!(result.is_ok(), "got {result:?}");
  assert_eq!(js.messages().len(), 0);
}

/// A job that throws is reported, not swallowed. `Ctx::execute_pending_job`
/// swallows the exception; `Runtime::execute_pending_job` hands it back.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn a_throwing_job_is_reported() {
  test_log_init();
  let mut js = make_js_host();

  js.enter(|ctx| {
    let f = rquickjs::Function::new(ctx.clone(), |ctx: rquickjs::Ctx| {
      Err::<(), _>(rquickjs::Exception::throw_message(&ctx, "job-boom"))
    })
    .unwrap();
    f.defer(()).unwrap();
  });

  assert_eq!(count(&js, "job-boom"), 1, "got: {:?}", js.messages());
}

// ------------------------------------------------------------------ operations

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn a_failing_operation_rejects_with_its_message() {
  test_log_init();
  let mut js = make_js_host();

  let outcome = js
    .settle("__InternalRsvimGlobalObject.fs_read_text_file('/nonexistent/rsvim-host')")
    .await;
  assert!(outcome.is_err());
}

/// If the operation's entry is gone by the time its completion arrives (the
/// table was cleared on the way out), the completion is ignored.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn a_completion_for_a_dropped_operation_is_ignored() {
  test_log_init();
  let mut js = make_js_host();
  let file = assert_fs::NamedTempFile::new("host-dropped.txt").unwrap();
  std::fs::write(file.path(), "x").unwrap();
  let path = file.path().to_string_lossy().to_string();

  js.run(&format!(
    "globalThis.done = false;
     __InternalRsvimGlobalObject.fs_read_text_file({path:?}).then(() => {{ globalThis.done = true; }});"
  ));
  assert_eq!(js.host.ops().pending_len(), 1);
  js.host.ops().clear();
  js.run_for(ms(100)).await;

  assert!(
    !js.eval::<bool>("done"),
    "nobody is waiting for it any more"
  );
}

/// Operations in flight are table entries: they can be counted.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn in_flight_operations_are_counted() {
  test_log_init();
  let mut js = make_js_host();
  let file = assert_fs::NamedTempFile::new("host-count.txt").unwrap();
  std::fs::write(file.path(), "x").unwrap();
  let path = file.path().to_string_lossy().to_string();

  js.run(&format!(
    "for (let i = 0; i < 3; i++) __InternalRsvimGlobalObject.fs_read_text_file({path:?});"
  ));
  assert_eq!(js.host.ops().pending_len(), 3);

  js.run_until(ms(5000), |js| js.host.ops().pending_len() == 0)
    .await;
  assert_eq!(js.host.ops().pending_len(), 0);
}

// ------------------------------------------------------------------ ownership

/// Dropping a host that still holds timers, operations and rejections must not
/// abort the process. QuickJS asserts in `JS_FreeRuntime` if a `Persistent`
/// outlives its runtime; `impl Drop for JsHost` is what prevents it. (Without
/// that impl this test kills the test binary with SIGABRT.)
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn dropping_a_host_with_pending_state_does_not_abort() {
  test_log_init();
  let mut js = make_js_host();
  let file = assert_fs::NamedTempFile::new("host-drop.txt").unwrap();
  std::fs::write(file.path(), "x").unwrap();
  let path = file.path().to_string_lossy().to_string();

  js.run(&format!(
    "setTimeout(() => {{}}, 60000);
     setInterval(() => {{}}, 60000);
     __InternalRsvimGlobalObject.fs_read_text_file({path:?}).then(() => {{}});"
  ));
  assert_eq!(js.host.ops().timers_len(), 2);
  assert_eq!(js.host.ops().pending_len(), 1);

  drop(js);
}

// ------------------------------------------------------------------ watchdog

/// With a timeout configured, a synchronous runaway is aborted and the engine
/// stays usable.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn the_watchdog_stops_a_synchronous_infinite_loop() {
  test_log_init();
  let mut js = make_js_host_with(JsRuntimeOptions {
    exec_timeout: Some(ms(50)),
    ..Default::default()
  });

  let started = Instant::now();
  let outcome = js.enter(|ctx| ctx.eval::<(), _>("while (true) {}"));
  assert!(outcome.is_err(), "the loop must be aborted");
  assert!(
    started.elapsed() < Duration::from_secs(2),
    "aborted promptly, took {:?}",
    started.elapsed()
  );

  // The runtime survives, and the deadline does not leak into the next entry.
  assert_eq!(js.eval::<i32>("1 + 1"), 2);
  js.run_for(ms(120)).await;
  assert_eq!(js.eval::<i32>("2 + 2"), 4);
}

/// The default is no limit: the behaviour `evloop-plan-v2.md` decided on.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn without_a_timeout_nothing_is_interrupted() {
  test_log_init();
  let mut js = make_js_host();

  let total: i32 =
    js.eval("let s = 0; for (let i = 0; i < 3000000; i++) s += 1; s");
  assert_eq!(total, 3_000_000);
}

/// A self-sustaining microtask chain runs to its natural end inside one entry:
/// there is no budget on the job queue.
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn a_long_microtask_chain_runs_to_completion() {
  test_log_init();
  let mut js = make_js_host();

  js.run(
    r#"
    globalThis.runs = 0;
    function step() {
      runs += 1;
      if (runs < 5000) Promise.resolve().then(step);
    }
    step();
    "#,
  );
  assert_eq!(js.eval::<i32>("runs"), 5000);
}
