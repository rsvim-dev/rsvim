//! Close file APIs.

use crate::is_v8_int;
use crate::js::JsRuntime;
use crate::js::converter::*;
use crate::js::resource::ResourceId;
use crate::js::resource::ResourceTableArc;
use crate::prelude::*;

pub fn fs_close_s(resource_table: ResourceTableArc, rid: ResourceId) {
  let mut resource_table = lock!(resource_table);
  let mut handle = resource_table.remove(&rid);
  debug_assert!(handle.is_some());
  // Drop file handle, i.e. close the file
  handle.take();
}

/// `Rsvim.fs.close` API.
pub fn close_sync<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut _rv: v8::ReturnValue,
) {
  debug_assert!(args.length() == 1);
  debug_assert!(is_v8_int!(args.get(0)));
  let file_rid = i32::from_v8(scope, args.get(0));
  trace!("Rsvim.fs.close:{:?}", file_rid);
  let file_rid = ResourceId::from(file_rid);

  let state_rc = JsRuntime::state(scope);
  let resource_table = state_rc.borrow().resource_table.clone();

  fs_close_s(resource_table, file_rid);
}
