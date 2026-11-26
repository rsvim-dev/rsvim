//! Child process.

use crate::js::JsFuture;
use crate::js::binding;
use crate::js::converter::*;
use crate::prelude::*;

pub struct ReadTextFromChildProcessStdioFuture {
  pub promise: v8::Global<v8::PromiseResolver>,
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

impl JsFuture for ReadTextFromChildProcessStdioFuture {
  fn run(&mut self, scope: &mut v8::PinScope) {
    trace!("|ReadTextFromChildProcessStdioFuture|");

    let result = self.maybe_result.take().unwrap();

    // Handle when something goes wrong with it.
    if let Err(e) = result {
      let message = v8::String::new(scope, &e.to_string()).unwrap();
      let exception = v8::Exception::error(scope, message);
      binding::set_exception_code(scope, exception, &e);
      self.promise.open(scope).reject(scope, exception);
      return;
    }

    // Otherwise, resolve the promise passing the result.
    let result = result.unwrap();

    // Deserialize into string
    let payload = postcard::from_bytes::<String>(&result).unwrap();
    let payload = payload.to_v8(scope);

    self.promise.open(scope).resolve(scope, payload).unwrap();
  }
}

pub struct WaitChildFuture {
  pub promise: v8::Global<v8::PromiseResolver>,
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

impl JsFuture for WaitChildFuture {
  fn run(&mut self, scope: &mut v8::PinScope) {
    trace!("|WaitChildFuture|");

    let result = self.maybe_result.take().unwrap();

    // Handle when something goes wrong with it.
    if let Err(e) = result {
      let message = v8::String::new(scope, &e.to_string()).unwrap();
      let exception = v8::Exception::error(scope, message);
      binding::set_exception_code(scope, exception, &e);
      self.promise.open(scope).reject(scope, exception);
      return;
    }

    // Otherwise, resolve the promise passing the result.
    let result = result.unwrap();

    // Deserialize into string
    let (success, exit_code, signal) = postcard::from_bytes::<(
      /* success */ bool,
      /* exit code */ Option<i32>,
      /* signal */ Option<i32>,
    )>(&result)
    .unwrap();

    let result = v8::Object::new(scope);
    let success = success.to_v8(scope);
    binding::set_property_to(scope, result, "success", success);
    if let Some(exit_code) = exit_code {
      let exit_code = exit_code.to_v8(scope);
      binding::set_property_to(scope, result, "code", exit_code);
    }
    if let Some(signal) = signal {
      let signal = signal.to_v8(scope);
      binding::set_property_to(scope, result, "signal", signal);
    }

    self
      .promise
      .open(scope)
      .resolve(scope, result.into())
      .unwrap();
  }
}
