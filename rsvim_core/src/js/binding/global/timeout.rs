//! Timeout APIs.

use crate::is_v8_bool;
use crate::is_v8_func;
use crate::is_v8_int;
use crate::js;
use crate::js::JsFuture;
use crate::js::JsRuntime;
use crate::js::TimerId;
use crate::js::converter::*;
use crate::js::pending;
use crate::prelude::*;
use std::rc::Rc;

struct TimeoutFuture {
  cb: Rc<v8::Global<v8::Function>>,
  params: Rc<Vec<v8::Global<v8::Value>>>,
}

impl JsFuture for TimeoutFuture {
  fn run(&mut self, scope: &mut v8::PinScope) {
    trace!("|TimeoutFuture|");
    let undefined = v8::undefined(scope).into();
    let callback = v8::Local::new(scope, (*self.cb).clone());
    let args: Vec<v8::Local<v8::Value>> = self
      .params
      .iter()
      .map(|arg| v8::Local::new(scope, arg))
      .collect();

    v8::tc_scope!(let tc_scope, scope);

    callback.call(tc_scope, undefined, &args);

    // Report if callback threw an exception.
    if tc_scope.has_caught() {
      let exception = tc_scope.exception().unwrap();
      let exception = v8::Global::new(tc_scope, exception);
      let state_rc = JsRuntime::state(tc_scope);
      state_rc
        .borrow_mut()
        .exceptions
        .capture_exception(exception);
    }
  }
}

/// Javascript `setTimeout`/`setInterval` API.
pub fn create_timer<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  debug_assert!(args.length() == 3);

  // Get timer's callback.
  debug_assert!(is_v8_func!(args.get(0)));
  let callback = v8::Local::<v8::Function>::try_from(args.get(0)).unwrap();
  let callback = Rc::new(v8::Global::new(scope, callback));

  // Get timer's delay time in millis.
  debug_assert!(is_v8_int!(args.get(1)));
  let delay = u32::from_v8(scope, args.get(1));
  // Get timer's repeated.
  debug_assert!(is_v8_bool!(args.get(2)));
  let repeated = bool::from_v8(scope, args.get(2));

  // NOTE: Don't delete this part of code, it shows how to convert function
  // arguments into an array of values.
  //
  // Convert params argument (Array<Local<Value>>) to Rust vector.
  // let params = match v8::Local::<v8::Array>::try_from(args.get(3)) {
  //   Ok(params) => (0..params.length()).fold(
  //     Vec::<v8::Global<v8::Value>>::new(),
  //     |mut acc, i| {
  //       let param = params.get_index(scope, i).unwrap();
  //       acc.push(v8::Global::new(scope, param));
  //       acc
  //     },
  //   ),
  //   Err(_) => vec![],
  // };

  // NOTE: Since in javascript side, we don't pass any extra parameters to
  // timers, thus it is always empty array. But, we leave this code here as a
  // reference.
  let params = vec![];
  let params = Rc::new(params);

  let state_rc = JsRuntime::state(scope);
  let timer_cb = {
    let state_rc = state_rc.clone();
    move || {
      let fut = TimeoutFuture {
        cb: Rc::clone(&callback),
        params: Rc::clone(&params),
      };
      let mut state = state_rc.borrow_mut();
      state.pending_futures.push(Box::new(fut));
    }
  };

  let mut state = state_rc.borrow_mut();
  let timer_id = js::TimerId::next();
  pending::create_timer(
    &mut state,
    timer_id,
    delay,
    repeated,
    Box::new(timer_cb),
  );
  rv.set_int32(timer_id.into());
  trace!(
    "|create_timer| timer_id:{:?}, delay:{:?}, repeated:{:?}",
    timer_id, delay, repeated
  );
}

/// Javascript `clearTimeout`/`clearInterval` API.
pub fn clear_timer<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  _: v8::ReturnValue,
) {
  debug_assert!(args.length() == 1);
  // Get timer ID, and remove it.
  debug_assert!(is_v8_int!(args.get(0)));
  let timer_id = TimerId::from_v8(scope, args.get(0));
  let state_rc = JsRuntime::state(scope);

  let mut state = state_rc.borrow_mut();
  pending::remove_timer(&mut state, timer_id);
  trace!("|clear_timer| timer_id:{:?}", timer_id);
}
