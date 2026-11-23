//! Microtask Queue APIs.

use crate::js;
use crate::js::JsRuntime;
use crate::js::err::report_js_error;
use crate::prelude::*;

pub fn report_error(
  scope: &mut v8::PinScope,
  args: v8::FunctionCallbackArguments,
  _: v8::ReturnValue,
) {
  let exception = v8::Global::new(scope, args.get(0));
  let state_rc = JsRuntime::state(scope);
  let mut state = state_rc.borrow_mut();

  state.exceptions.capture_exception(exception);
  drop(state);

  if let Some(error) = js::check_exceptions(scope) {
    let state = state_rc.borrow();
    report_js_error(&state, TheErr::JsError(Box::new(error)));
  }
}

// This method queues a microtask to invoke callback.
pub fn queue_microtask(
  scope: &mut v8::PinScope,
  args: v8::FunctionCallbackArguments,
  _: v8::ReturnValue,
) {
  let callback = v8::Local::<v8::Function>::try_from(args.get(0)).unwrap();
  let state_rc = JsRuntime::state(scope);
  let state = state_rc.borrow();
  let ctx = state.context.open(scope);

  ctx.get_microtask_queue().enqueue_microtask(scope, callback);
}
