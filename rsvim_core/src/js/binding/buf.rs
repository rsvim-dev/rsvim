//! APIs for `buf` module.

use crate::buf::BufferId;
use crate::js::binding;
use crate::js::converter::*;
use crate::prelude::*;

/// `buf.current` API.
pub fn current(
  scope: &mut v8::PinScope,
  _args: v8::FunctionCallbackArguments,
  mut rv: v8::ReturnValue,
) {
  debug_assert!(_args.length() == 0);
  let state_rc = crate::js::get_state(scope);
  let tree = state_rc.borrow().editor.tree.clone();
  let tree = lock!(tree);
  match tree.current_window() {
    Some(current_window) => {
      let buf = current_window.buffer().upgrade().unwrap();
      let buf_id = lock!(buf).id();
      trace!("buf.current: {:?}", buf_id);
      rv.set_int32(buf_id.into());
    }
    None => {
      trace!("buf.current: not exist");
      rv.set_undefined();
    }
  }
}

/// `buf.list` API.
pub fn list(
  scope: &mut v8::PinScope,
  _args: v8::FunctionCallbackArguments,
  mut rv: v8::ReturnValue,
) {
  debug_assert!(_args.length() == 0);
  let state_rc = crate::js::get_state(scope);
  let buffer_manager = state_rc.borrow().editor.buffer_manager.clone();
  let buffer_manager = lock!(buffer_manager);
  trace!("buf.list: {:?}", buffer_manager.keys());

  let bufs = buffer_manager
    .keys()
    .collect::<Vec<&BufferId>>()
    .to_v8(scope);

  rv.set(bufs);
}

/// `buf.writeSync` API.
pub fn write_sync<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  debug_assert!(args.length() == 1);
  debug_assert!(args.get(0).is_int32());
  let buf_id = BufferId::from_v8(scope, args.get(0));
  trace!("buf.writeSync: {:?}", buf_id);

  let state_rc = crate::js::get_state(scope);
  let state = state_rc.borrow();
  let buffer_manager = state.editor.buffer_manager.clone();
  let buffer_manager = lock!(buffer_manager);

  match buffer_manager.write_buffer(buf_id) {
    Ok(n) => {
      trace!("write_sync success, bufId:{:?}, bytes:{:?}", buf_id, n);
      rv.set_int32(n as i32);
    }
    Err(e) => {
      trace!("write_sync failed, bufId:{:?}, error:{:?}", buf_id, e);
      binding::throw_exception(scope, &e);
    }
  }
}
