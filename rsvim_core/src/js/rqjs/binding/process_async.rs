//! APIs for the `process` module — the promise-returning half.
//!
//! `ChildProcess.wait()` and `ChildProcessReadableStream.text()` block on the
//! operating system, so they run on [`crate::js::rqjs::runtime::JsKernel`]'s
//! `blocked_tracker`. That tracker is the clone of `Rsvim::blocked_tracker`, so
//! `close_trackers` waits for a `wait()` that was already promised to
//! javascript — exiting in the middle of one would leave the promise dangling
//! with no diagnostic.
//!
//! The v8 backend reaches the same guarantee the long way round: a
//! `MasterMessage` request, a `detached_tracker.spawn_blocking`, a `JsMessage`
//! response, and a `JsFuture` that resolves a `PromiseResolver`. QuickJS has a
//! scheduler of its own, so the whole relay collapses into "spawn the blocking
//! call, resolve the promise when it returns".

use crate::js::resource::Resource;
use crate::js::resource::ResourceId;
use crate::js::rqjs::binding::set_function_to;
use crate::js::rqjs::get_state;
use crate::js::rqjs::runtime::JsKernelRc;
use crate::prelude::*;
use rquickjs::Ctx;
use rquickjs::Exception;
use rquickjs::Object;
use rquickjs::Promise;
use rquickjs::Result as JsResult;

/// Clones the kernel, or raises if the context has none.
fn kernel(ctx: &Ctx<'_>) -> JsResult<JsKernelRc> {
  get_state(ctx).ok_or_else(|| {
    Exception::throw_message(ctx, "js kernel is not initialised")
  })
}

/// Turns a `TheErr` into a javascript exception.
fn throw(ctx: &Ctx<'_>, e: TheErr) -> rquickjs::Error {
  Exception::throw_message(ctx, &e.to_string())
}

/// Turns a `JoinError` into a javascript exception.
///
/// `TheErr` has no variant for "the blocking task died", and inventing one
/// would mean every consumer of `TheErr` learns about a shape only this module
/// produces. A plain message keeps the failure visible without that blast
/// radius.
fn throw_join(
  ctx: &Ctx<'_>,
  what: &str,
  e: tokio::task::JoinError,
) -> rquickjs::Error {
  Exception::throw_message(ctx, &format!("process.{what} task: {e}"))
}

/// `childProcess.wait()` -> `Promise<{success, code?, signal?}>`.
///
/// The object mirrors the v8 backend's: `success` is always present, `code` and
/// `signal` only when the platform reported them.
pub fn wait_child<'js>(ctx: Ctx<'js>, rid: i32) -> JsResult<Promise<'js>> {
  let kernel = kernel(&ctx)?;
  let resource_table = kernel.borrow().editor.resource_table.clone();
  let rid = ResourceId::from(rid);
  trace!("process.wait:{:?}", rid);

  let ctx_err = ctx.clone();
  Promise::wrap_future(&ctx, async move {
    let table = resource_table.clone();
    let tracker = kernel.borrow().blocked_tracker.clone();
    let joined = tracker
      .spawn_blocking(move || -> TheResult<(bool, Option<i32>, Option<i32>)> {
        let child = lock!(table).get(&rid).cloned();
        let Some(child) = child else {
          return Err(TheErr::ChildProcessNotFound(rid));
        };
        let Resource::ChildProcess(resource) = child else {
          unreachable!()
        };
        let resource = resource.data();
        let mut resource = lock!(resource);
        let exit_status = resource
          .wait()
          .map_err(|e| TheErr::WaitChildProcessFailed(rid, e))?;
        let success = exit_status.success();
        let exit_code = exit_status.code();
        let signal: Option<i32> = {
          #[cfg(target_family = "unix")]
          {
            use std::os::unix::process::ExitStatusExt;
            exit_status.signal()
          }
          #[cfg(not(target_family = "unix"))]
          {
            None
          }
        };
        Ok((success, exit_code, signal))
      })
      .await
      .map_err(|e| throw_join(&ctx_err, "wait", e))?;

    let (success, exit_code, signal) =
      joined.map_err(|e| throw(&ctx_err, e))?;

    let object = Object::new(ctx_err.clone())?;
    object.set("success", success)?;
    if let Some(exit_code) = exit_code {
      object.set("code", exit_code)?;
    }
    if let Some(signal) = signal {
      object.set("signal", signal)?;
    }
    Ok::<Object<'js>, rquickjs::Error>(object)
  })
}

/// `childProcessReadableStream.text()` -> `Promise<string>`.
pub fn read_text_from_child<'js>(
  ctx: Ctx<'js>,
  rid: i32,
) -> JsResult<Promise<'js>> {
  let kernel = kernel(&ctx)?;
  let resource_table = kernel.borrow().editor.resource_table.clone();
  let rid = ResourceId::from(rid);
  trace!("process.text:{:?}", rid);

  let ctx_err = ctx.clone();
  Promise::wrap_future(&ctx, async move {
    let table = resource_table.clone();
    let tracker = kernel.borrow().blocked_tracker.clone();
    let joined = tracker
      .spawn_blocking(move || -> TheResult<String> {
        use std::io::Read;

        let child = lock!(table).get(&rid).cloned();
        let Some(child) = child else {
          return Err(TheErr::ChildProcessNotFound(rid));
        };
        let mut payload = String::new();
        match child {
          Resource::ChildProcessStdout(resource) => {
            let resource = resource.data();
            let mut resource = lock!(resource);
            resource
              .read_to_string(&mut payload)
              .map_err(|e| TheErr::ReadChildProcessStdioFailed(rid, e))?;
          }
          Resource::ChildProcessStderr(resource) => {
            let resource = resource.data();
            let mut resource = lock!(resource);
            resource
              .read_to_string(&mut payload)
              .map_err(|e| TheErr::ReadChildProcessStdioFailed(rid, e))?;
          }
          _ => unreachable!(),
        }
        Ok(payload)
      })
      .await
      .map_err(|e| throw_join(&ctx_err, "text", e))?;

    let payload = joined.map_err(|e| throw(&ctx_err, e))?;
    Ok::<String, rquickjs::Error>(payload)
  })
}

/// Installs the promise-returning `process_*` built-ins on `internal`.
pub fn install<'js>(ctx: &Ctx<'js>, internal: &Object<'js>) -> JsResult<()> {
  set_function_to(ctx, internal, "process_wait_child", wait_child)?;
  set_function_to(
    ctx,
    internal,
    "process_read_text_from_child",
    read_text_from_child,
  )?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use crate::js::rqjs::binding::install_bindings;
  use crate::js::rqjs::runtime::JsKernel;
  use crate::js::rqjs::runtime::JsRuntimeOptions;
  use crate::js::rqjs::runtime::set_state;
  use crate::tests::js::make_editor;
  use crate::tests::log::init as test_log_init;
  use rquickjs::AsyncContext;
  use rquickjs::AsyncRuntime;
  use std::task::Poll;
  use std::time::Duration;

  /// Builds a context with the bindings installed.
  ///
  /// The driver is polled *outside* the `with` block on purpose: `drive` takes
  /// the same inner lock that `with` holds.
  async fn make_context() -> (AsyncContext, AsyncRuntime) {
    let editor = make_editor();
    let (master_tx, _master_rx) = tokio::sync::mpsc::unbounded_channel();
    let kernel = JsKernel::to_rc(JsKernel {
      options: JsRuntimeOptions::default(),
      master_tx,
      editor,
      blocked_tracker: tokio_util::task::TaskTracker::new(),
    });
    let rt = AsyncRuntime::new().unwrap();
    let ctx = AsyncContext::full(&rt).await.unwrap();
    ctx
      .with(move |ctx| {
        set_state(&ctx, kernel).unwrap();
        install_bindings(&ctx).unwrap();
      })
      .await;
    (ctx, rt)
  }

  async fn pump(
    drive: &mut std::pin::Pin<&mut rquickjs::runtime::DriveFuture>,
  ) {
    futures::future::poll_fn(|cx| {
      let _ = drive.as_mut().poll(cx);
      Poll::Ready(())
    })
    .await;
    tokio::time::sleep(Duration::from_millis(2)).await;
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn waiting_on_an_unknown_child_rejects_the_promise() {
    test_log_init();
    let (ctx, rt) = make_context().await;

    ctx
      .with(|ctx| {
        ctx
          .eval::<(), _>(
            r#"
            globalThis.outcome = "pending";
            __InternalRsvimGlobalObject.process_wait_child(9999).then(
              () => { globalThis.outcome = "resolved"; },
              (e) => { globalThis.outcome = String(e); },
            );
            "#,
          )
          .unwrap();
      })
      .await;

    let drive = rt.drive();
    futures::pin_mut!(drive);

    let mut outcome = String::from("pending");
    for _ in 0..100 {
      pump(&mut drive).await;
      outcome = ctx
        .with(|ctx| ctx.eval("globalThis.outcome").unwrap())
        .await;
      if outcome != "pending" {
        break;
      }
    }

    assert_ne!(outcome, "pending", "the promise must settle");
    assert_ne!(outcome, "resolved", "an unknown child must reject");
    assert!(
      outcome.contains("not found") || outcome.contains("9999"),
      "the rejection must name the missing child, got: {outcome}"
    );
  }
}
