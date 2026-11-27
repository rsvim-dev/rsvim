//! APIs for the `buf` module.

use crate::buf::BufferId;
use crate::js::binding::set_function_to;
use crate::js::runtime::kernel;
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
  let kernel = kernel(&ctx)?;
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
  let kernel = kernel(&ctx)?;
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

  let kernel = kernel(&ctx)?;
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
