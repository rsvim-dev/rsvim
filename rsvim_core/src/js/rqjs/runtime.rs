//! Runtime, kernel and the `get_state` lookup.

use crate::chan::MasterMessage;
use crate::evloop::Editor;
use rquickjs::Ctx;
use rquickjs::JsLifetime;
use rquickjs::Result as JsResult;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::task::TaskTracker;

/// The QuickJS-ng counterpart of the v8 kernel.
///
/// Same rule as the v8 one: only state that js callbacks can and only can reach
/// belongs here. `Rsvim`-level fields are forbidden.
#[derive(Debug, rsvim_macro::RcRefCellPtr)]
pub struct JsKernel {
  /// Runtime options.
  pub options: JsRuntimeOptions,
  /// Data access for rsvim.
  pub master_tx: UnboundedSender<MasterMessage>,
  /// Editor's shared data (the same `Arc`s as `Rsvim::editor`).
  pub editor: Editor,
  /// The tracker for work whose result was already promised to the user.
  ///
  /// This is a clone of `Rsvim::blocked_tracker`, not a separate tracker: the
  /// two handles share one counter, so `Rsvim::close_trackers` waiting on its
  /// copy also waits for every blocking job a js callback started here.
  ///
  /// It has to live in the kernel because a promise-producing callback cannot
  /// reach `Rsvim` at all -- rquickjs futures are `'js`, and `Rsvim` is not
  /// reachable from an isolate. `TaskTracker` is `Clone + Send + Sync`, so
  /// sharing it costs one clone.
  pub blocked_tracker: TaskTracker,
}

// The kernel holds no `'js` types, so its lifetime is trivially unchanged.
unsafe impl<'js> JsLifetime<'js> for JsKernel {
  type Changed<'to> = JsKernel;
}

/// Runtime options, shared with the v8 backend so `evloop` does not care which
/// engine is compiled in.
#[derive(Debug, Default, Clone)]
pub struct JsRuntimeOptions {
  /// The main entry point for the program.
  pub root: Option<String>,
  /// Indicates if we're running JavaScript tests.
  pub test_mode: bool,
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

#[cfg(test)]
pub(crate) mod tests {
  use super::*;
  use crate::tests::js::make_editor;
  use crate::tests::log::init as test_log_init;
  use rquickjs::AsyncContext;
  use rquickjs::AsyncRuntime;
  use std::rc::Rc;

  fn make_kernel() -> JsKernelRc {
    let (master_tx, _master_rx) =
      tokio::sync::mpsc::unbounded_channel::<MasterMessage>();
    JsKernel::to_rc(JsKernel {
      options: JsRuntimeOptions::default(),
      master_tx,
      editor: make_editor(),
      blocked_tracker: TaskTracker::new(),
    })
  }

  /// The kernel must survive a store/lookup round trip.
  ///
  /// This is the load-bearing assumption of the whole port: v8 gives callbacks
  /// the kernel through an isolate slot, and `ctx.userdata` is the only
  /// equivalent. It also pins the guard lifetime rule: a live
  /// `UserDataGuard` makes `store_userdata` fail, so `get_state` has to clone
  /// the `Rc` out and drop the guard.
  #[test]
  fn userdata_roundtrip() {
    test_log_init();

    let runtime = AsyncRuntime::new().unwrap();
    let context =
      futures::executor::block_on(AsyncContext::full(&runtime)).unwrap();

    let kernel = make_kernel();
    futures::executor::block_on(
      context.with(|ctx| -> JsResult<()> { set_state(&ctx, kernel.clone()) }),
    )
    .unwrap();

    let marker =
      futures::executor::block_on(context.with(|ctx| -> JsResult<u8> {
        let state = get_state(&ctx).expect("the kernel must be reachable");
        // Both handles must point at the same kernel.
        Ok(u8::from(Rc::ptr_eq(&state, &kernel)))
      }))
      .unwrap();
    assert_eq!(marker, 1, "get_state must return the very same kernel");

    // Storing again must still work: `get_state` dropped its guard.
    futures::executor::block_on(
      context.with(|ctx| -> JsResult<()> { set_state(&ctx, kernel.clone()) }),
    )
    .expect("a leaked userdata guard would make this fail");
  }

  /// A module must be declarable, evaluable, and readable back.
  #[test]
  fn module_declare_and_eval() {
    test_log_init();

    let runtime = AsyncRuntime::new().unwrap();
    let context =
      futures::executor::block_on(AsyncContext::full(&runtime)).unwrap();

    let value = futures::executor::block_on(context.with(|ctx| {
      let declared = rquickjs::Module::declare(
        ctx.clone(),
        "hello.js",
        "export const a = 40 + 2;",
      )?;
      let (evaluated, _promise) = declared.eval()?;
      let ns = evaluated.namespace()?;
      let a: i32 = ns.get("a")?;
      Ok::<_, rquickjs::Error>(a)
    }))
    .unwrap();

    assert_eq!(value, 42);
  }

  /// `ctx.promise()` + `then` + manual resolve is the replacement for v8's
  /// `enqueue_microtask`.
  ///
  /// The `then` returned by rquickjs is an *unbound* method, so the promise has
  /// to be passed back in as its receiver; calling `then(cb)` throws. That is
  /// easy to get wrong and impossible to notice without running it.
  #[test]
  fn microtask_via_promise_then() {
    test_log_init();

    let runtime = AsyncRuntime::new().unwrap();
    let context =
      futures::executor::block_on(AsyncContext::full(&runtime)).unwrap();

    let ran = futures::executor::block_on(context.with(|ctx| {
      let flag: rquickjs::Function = ctx.eval(
        "(function () { globalThis.__ran = false; \
         return () => { globalThis.__ran = true; }; })()",
      )?;
      let (promise, resolve, _reject) = ctx.promise()?;
      let then: rquickjs::Function = promise.then()?;
      then.call::<_, ()>((rquickjs::prelude::This(promise.clone()), flag))?;
      resolve.call::<_, ()>((rquickjs::Value::new_undefined(ctx.clone()),))?;
      Ok::<_, rquickjs::Error>(())
    }));
    ran.unwrap();
  }
}
