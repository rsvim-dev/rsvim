//! APIs for the `buf` module.

use crate::buf::BufferId;
use crate::js::rqjs::binding::set_function_to;
use crate::js::rqjs::get_state;
use crate::prelude::*;
use rquickjs::Coerced;
use rquickjs::Ctx;
use rquickjs::Exception;
use rquickjs::Object;
use rquickjs::Result as JsResult;

/// `buf.current()`.
///
/// Returns `undefined` when there is no current window yet, which the js side
/// documents as "no buffer".
pub fn current<'js>(ctx: Ctx<'js>) -> JsResult<Option<i32>> {
  let kernel = get_state(&ctx).ok_or_else(|| {
    Exception::throw_message(&ctx, "js kernel is not initialised")
  })?;
  let tree = kernel.borrow().editor.tree.clone();
  let tree = lock!(tree);
  match tree.current_window() {
    Some(current_window) => {
      let buf = current_window.buffer().upgrade().unwrap();
      let buf_id = lock!(buf).id();
      trace!("buf.current: {:?}", buf_id);
      Ok(Some(buf_id.into()))
    }
    None => {
      trace!("buf.current: not exist");
      Ok(None)
    }
  }
}

/// `buf.list()`.
pub fn list<'js>(ctx: Ctx<'js>) -> JsResult<Vec<i32>> {
  let kernel = get_state(&ctx).ok_or_else(|| {
    Exception::throw_message(&ctx, "js kernel is not initialised")
  })?;
  let buffer_manager = kernel.borrow().editor.buffer_manager.clone();
  let buffer_manager = lock!(buffer_manager);
  trace!("buf.list: {:?}", buffer_manager.keys());
  Ok(
    buffer_manager
      .keys()
      .map(|id| (*id).into())
      .collect::<Vec<i32>>(),
  )
}

/// `buf.writeSync(bufId)`.
pub fn write_sync<'js>(ctx: Ctx<'js>, buf_id: Coerced<i32>) -> JsResult<i32> {
  let buf_id = BufferId::from(buf_id.0);
  trace!("buf.writeSync: {:?}", buf_id);

  let kernel = get_state(&ctx).ok_or_else(|| {
    Exception::throw_message(&ctx, "js kernel is not initialised")
  })?;
  let buffer_manager = kernel.borrow().editor.buffer_manager.clone();
  let buffer_manager = lock!(buffer_manager);

  match buffer_manager.write_buffer(buf_id) {
    Ok(n) => {
      trace!("write_sync success, bufId:{:?}, bytes:{:?}", buf_id, n);
      Ok(n as i32)
    }
    Err(e) => {
      trace!("write_sync failed, bufId:{:?}, error:{:?}", buf_id, e);
      Err(Exception::throw_message(&ctx, &e.to_string()))
    }
  }
}

/// Installs the `buf_*` built-ins on `internal`.
pub fn install<'js>(ctx: &Ctx<'js>, internal: &Object<'js>) -> JsResult<()> {
  set_function_to(ctx, internal, "buf_current", current)?;
  set_function_to(ctx, internal, "buf_list", list)?;
  set_function_to(ctx, internal, "buf_write_sync", write_sync)?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use crate::js::rqjs::binding::install_bindings;
  use crate::js::rqjs::runtime;
  use crate::js::rqjs::runtime::JsKernel;
  use crate::js::rqjs::runtime::JsRuntimeOptions;
  use crate::js::rqjs::runtime::set_state;
  use crate::tests::js::make_editor;
  use crate::tests::log::init as test_log_init;
  use rquickjs::AsyncContext;
  use rquickjs::AsyncRuntime;

  fn with_kernel<R>(
    f: impl FnOnce(rquickjs::Ctx<'_>, runtime::JsKernelRc) -> R,
  ) -> R {
    let editor = make_editor();
    let (master_tx, _master_rx) = tokio::sync::mpsc::unbounded_channel();
    let kernel = JsKernel::to_rc(JsKernel {
      options: JsRuntimeOptions::default(),
      master_tx,
      editor,
      blocked_tracker: tokio_util::task::TaskTracker::new(),
    });
    let rt = AsyncRuntime::new().unwrap();
    let ctx = futures::executor::block_on(AsyncContext::full(&rt)).unwrap();
    futures::executor::block_on(ctx.with(move |ctx| {
      set_state(&ctx, kernel.clone()).unwrap();
      install_bindings(&ctx).unwrap();
      f(ctx, kernel)
    }))
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn empty_editor_has_no_current_buffer() {
    test_log_init();
    with_kernel(|ctx, _kernel| {
      // `make_editor` builds no window, so both calls report emptiness rather
      // than raising. The js wrapper turns this into `undefined` / `[]`.
      let current: Option<i32> = ctx
        .eval("__InternalRsvimGlobalObject.buf_current()")
        .unwrap();
      assert_eq!(current, None);

      let list: Vec<i32> =
        ctx.eval("__InternalRsvimGlobalObject.buf_list()").unwrap();
      assert!(list.is_empty());
    });
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn write_sync_on_a_missing_buffer_is_a_javascript_error() {
    test_log_init();
    with_kernel(|ctx, _kernel| {
      // Buffer ids start at 1, so 99 cannot exist in a fresh editor.
      let result =
        ctx.eval::<i32, _>("__InternalRsvimGlobalObject.buf_write_sync(99)");
      assert!(result.is_err());
    });
  }
}
