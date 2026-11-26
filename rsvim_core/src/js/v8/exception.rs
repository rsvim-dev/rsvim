//! Js exceptions.

use crate::js::binding::set_function_to;

pub type PromiseRejectionEntry =
  (v8::Global<v8::Promise>, v8::Global<v8::Value>);

pub struct ExceptionState {
  /// Holds the current uncaught exception.
  pub exception: Option<v8::Global<v8::Value>>,
  /// Holds uncaught promise rejections.
  pub promise_rejections: Vec<PromiseRejectionEntry>,
  /// Hook to run on an uncaught exception.
  pub uncaught_exception_cb: Option<v8::Global<v8::Function>>,
  /// Hook to run on an uncaught promise rejection.
  pub unhandled_rejection_cb: Option<v8::Global<v8::Function>>,
}

impl ExceptionState {
  /// Creates a new store with given report policy.
  pub fn new() -> Self {
    ExceptionState {
      exception: None,
      promise_rejections: Vec::default(),
      uncaught_exception_cb: None,
      unhandled_rejection_cb: None,
    }
  }

  /// Registers the uncaught exception.
  pub fn capture_exception(&mut self, exception: v8::Global<v8::Value>) {
    if self.exception.is_none() {
      self.exception = Some(exception);
    }
  }

  /// Registers a promise rejection to the store.
  pub fn capture_promise_rejection(
    &mut self,
    promise: v8::Global<v8::Promise>,
    reason: v8::Global<v8::Value>,
  ) {
    self.promise_rejections.push((promise, reason));
  }

  pub fn has_promise_rejection(&self) -> bool {
    !self.promise_rejections.is_empty()
  }

  pub fn remove_promise_rejection(
    &mut self,
    promise: &v8::Global<v8::Promise>,
  ) {
    // Find the correct entry to remove.
    self
      .promise_rejections
      .retain(|(value, _)| value != promise);
  }

  pub fn remove_promise_rejection_entry(
    &mut self,
    exception: &v8::Global<v8::Value>,
  ) {
    // Find the correct entry to remove.
    self
      .promise_rejections
      .retain(|(_, value)| value != exception);
  }

  pub fn set_uncaught_exception_callback(
    &mut self,
    callback: Option<v8::Global<v8::Function>>,
  ) {
    self.uncaught_exception_cb = callback;
  }

  pub fn set_unhandled_rejection_callback(
    &mut self,
    callback: Option<v8::Global<v8::Function>>,
  ) {
    self.unhandled_rejection_cb = callback;
  }
}

pub fn initialize(scope: &mut v8::PinScope) -> v8::Global<v8::Object> {
  // Create local JS object.
  let target = v8::Object::new(scope);

  set_function_to(
    scope,
    target,
    "setUncaughtExceptionCallback",
    set_uncaught_exception_callback,
  );

  set_function_to(
    scope,
    target,
    "setUnhandledRejectionCallback",
    set_unhandled_rejection_callback,
  );

  // Return v8 global handle.
  v8::Global::new(scope, target)
}

/// Setting the `uncaught_exception_callback` from JavaScript.
fn set_uncaught_exception_callback(
  scope: &mut v8::PinScope,
  args: v8::FunctionCallbackArguments,
  _: v8::ReturnValue,
) {
  // Note: Passing `null` from JavaScript essentially will unset the defined callback.
  let callback = match v8::Local::<v8::Function>::try_from(args.get(0)) {
    Ok(callback) => Some(v8::Global::new(scope, callback)),
    Err(_) => None,
  };

  let state_rc = crate::js::get_state(scope);
  let mut state = state_rc.borrow_mut();

  state.exceptions.set_uncaught_exception_callback(callback);
}

/// Setting the `unhandled_rejection_callback` from JavaScript.
fn set_unhandled_rejection_callback(
  scope: &mut v8::PinScope,
  args: v8::FunctionCallbackArguments,
  _: v8::ReturnValue,
) {
  // Note: Passing `null` from JavaScript essentially will unset the defined callback.
  let callback = match v8::Local::<v8::Function>::try_from(args.get(0)) {
    Ok(callback) => Some(v8::Global::new(scope, callback)),
    Err(_) => None,
  };

  let state_rc = crate::js::get_state(scope);
  let mut state = state_rc.borrow_mut();

  state.exceptions.set_unhandled_rejection_callback(callback);
}
