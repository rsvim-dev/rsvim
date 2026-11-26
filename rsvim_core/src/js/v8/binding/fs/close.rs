//! Close file APIs.

// The engine-neutral implementation lives in `crate::js::fs`.
pub use crate::js::fs::fs_close_s;

use crate::is_v8_int;
use crate::js::converter::*;
use crate::js::resource::ResourceId;
use crate::prelude::*;

/// `fs.close` API.
pub fn close_sync<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut _rv: v8::ReturnValue,
) {
  debug_assert!(args.length() == 1);
  debug_assert!(is_v8_int!(args.get(0)));
  let file_rid = i32::from_v8(scope, args.get(0));
  trace!("fs.close:{:?}", file_rid);
  let file_rid = ResourceId::from(file_rid);

  let state_rc = crate::js::get_state(scope);
  let resource_table = state_rc.borrow().editor.resource_table.clone();

  fs_close_s(resource_table, file_rid);
}
