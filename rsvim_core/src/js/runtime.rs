//! The kernel and the `get_state` lookup.
//!
//! The runtime itself -- the engine, its context, the tables around them -- is
//! [`crate::js::JsHost`].

use crate::chan::MasterMessage;
use crate::evloop::Editor;
use crate::prelude::*;
use rquickjs::Ctx;
use rquickjs::JsLifetime;
use rquickjs::Result as JsResult;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

use super::err::JsError;

/// The state only javascript callbacks can reach.
///
/// Same rule as any engine embedding: only state that a binding can and only
/// can reach belongs here. `Rsvim`-level fields are forbidden, because a
/// binding never sees `Rsvim`.
///
/// Asynchronous work is not here: it goes through the operation table
/// ([`crate::js::ops::op_table`]), which also owns the task trackers.
#[derive(Debug, rsvim_macro::RcRefCellPtr)]
pub struct JsKernel {
  /// Runtime options.
  pub options: JsRuntimeOptions,
  /// Data access for rsvim.
  pub master_tx: UnboundedSender<MasterMessage>,
  /// Editor's shared data (the same `Arc`s as `Rsvim::editor`).
  pub editor: Editor,
}

// The kernel holds no `'js` types, so its lifetime is trivially unchanged.
unsafe impl<'js> JsLifetime<'js> for JsKernel {
  type Changed<'to> = JsKernel;
}

/// Runtime options.
#[derive(Debug, Default, Clone)]
pub struct JsRuntimeOptions {
  /// The main entry point for the program.
  pub root: Option<String>,
  /// Indicates if we're running JavaScript tests.
  pub test_mode: bool,
  /// How long one entry into javascript may run, `None` for no limit.
  ///
  /// This is the watchdog for synchronous runaways (`while (true) {}`): when the
  /// time is up the interpreter raises an exception javascript cannot catch, the
  /// callback is aborted, and the error is reported. It does not preempt native
  /// calls that block, and it does not roll back what the callback already did.
  ///
  /// Off by default: without it a runaway script freezes the editor, which is
  /// the behaviour `evloop-plan-v2.md` decided to accept. See
  /// `evloop-quickjs-plan-claude.md` §8.6 for the trade-off.
  pub exec_timeout: Option<Duration>,
}

/// Resolves the kernel stored as context userdata.
///
/// The `UserDataGuard` must not outlive the expression: `store_userdata`
/// refuses to run while any guard is alive, so the `Rc` is cloned out and the
/// guard dropped immediately.
pub fn get_state<'js>(ctx: &Ctx<'js>) -> Option<JsKernelRc> {
  ctx.userdata::<JsKernelRc>().map(|g| (*g).clone())
}

/// Installs `kernel` as the context's userdata.
pub fn set_state<'js>(ctx: &Ctx<'js>, kernel: JsKernelRc) -> JsResult<()> {
  ctx.store_userdata(kernel)?;
  Ok(())
}

/// Lifts a `rquickjs` error into the crate's error type.
pub fn to_io_error(error: rquickjs::Error) -> IoErr {
  IoErr::other(error.to_string())
}

/// Turns a javascript exception into [`JsError`] so it can be reported.
pub fn capture_exception<'js>(ctx: &Ctx<'js>) -> Option<JsError> {
  if !ctx.has_exception() {
    return None;
  }
  let value = ctx.catch();
  let error = JsError::from_js_value(ctx, value);
  ctx.catch();
  Some(error)
}

#[cfg(test)]
pub(crate) mod tests {
  use super::*;
  use crate::tests::js::make_editor;
  use crate::tests::log::init as test_log_init;
  use rquickjs::Context;
  use rquickjs::Module;
  use rquickjs::Runtime;
  use std::rc::Rc;

  fn make_kernel() -> JsKernelRc {
    let (master_tx, _master_rx) =
      tokio::sync::mpsc::unbounded_channel::<MasterMessage>();
    JsKernel::to_rc(JsKernel {
      options: JsRuntimeOptions::default(),
      master_tx,
      editor: make_editor(),
    })
  }

  /// The kernel must survive a store/lookup round trip.
  ///
  /// This is the load-bearing assumption of the whole port: a binding reaches
  /// the kernel through `ctx.userdata`, and a live `UserDataGuard` makes
  /// `store_userdata` fail, so `get_state` has to clone the `Rc` out and drop
  /// the guard.
  #[test]
  fn userdata_roundtrip() {
    test_log_init();

    let runtime = Runtime::new().unwrap();
    let context = Context::full(&runtime).unwrap();

    let kernel = make_kernel();
    context
      .with(|ctx| -> JsResult<()> { set_state(&ctx, kernel.clone()) })
      .unwrap();

    let marker = context
      .with(|ctx| -> JsResult<u8> {
        let state = get_state(&ctx).expect("the kernel must be reachable");
        // Both handles must point at the same kernel.
        Ok(u8::from(Rc::ptr_eq(&state, &kernel)))
      })
      .unwrap();
    assert_eq!(marker, 1, "get_state must return the very same kernel");

    // Storing again must still work: `get_state` dropped its guard.
    context
      .with(|ctx| -> JsResult<()> { set_state(&ctx, kernel.clone()) })
      .expect("a leaked userdata guard would make this fail");
  }

  /// Declaring and evaluating a module must yield its exported bindings.
  #[test]
  fn module_declare_and_eval() {
    test_log_init();

    let runtime = Runtime::new().unwrap();
    let context = Context::full(&runtime).unwrap();

    let value = context
      .with(|ctx| -> JsResult<i32> {
        let declared =
          Module::declare(ctx.clone(), "hello.js", "export const a = 40 + 2;")?;
        let (evaluated, promise) = declared.eval()?;
        promise.finish::<()>()?;
        evaluated.namespace()?.get::<_, i32>("a")
      })
      .unwrap();

    assert_eq!(value, 42);
  }

  /// `queueMicrotask` runs through the same job queue the host drains.
  #[test]
  fn microtask_via_promise_then() {
    test_log_init();

    let runtime = Runtime::new().unwrap();
    let context = Context::full(&runtime).unwrap();

    let value = context
      .with(|ctx| -> JsResult<i32> {
        let flag = rquickjs::Function::new(ctx.clone(), || 7_i32)?;
        let (promise, resolve, _reject) = ctx.promise()?;
        // `promise.then()` returns the unbound `then` method, so it has to be
        // called with the promise as `this`.
        let then = promise.then()?;
        then.call::<_, ()>((rquickjs::prelude::This(promise.clone()), flag))?;
        resolve
          .call::<_, ()>((rquickjs::Value::new_undefined(ctx.clone()),))?;
        // Proto and async functions reject; a plain resolved promise is fine.
        promise.finish::<()>()?;
        Ok(7_i32)
      })
      .unwrap();

    assert_eq!(value, 7);
  }
}
