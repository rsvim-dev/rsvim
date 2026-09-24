//! Read Text file APIs.

use crate::is_v8_str;
use crate::js;
use crate::js::JsFuture;
use crate::js::JsRuntime;
use crate::js::binding;
use crate::js::pending;
use crate::prelude::*;

pub fn fs_read_text_file_s(path: &Path) -> TheResult<String> {
  match std::fs::read_to_string(path) {
    Ok(buf) => Ok(buf),
    Err(e) => Err(TheErr::ReadFileByPathFailed(path.to_path_buf(), e)),
  }
}

pub async fn fs_read_text_file_a(path: &Path) -> TheResult<String> {
  match tokio::fs::read_to_string(path).await {
    Ok(buf) => Ok(buf),
    Err(e) => Err(TheErr::ReadFileByPathFailed(path.to_path_buf(), e)),
  }
}

pub struct FsReadTextFileFuture {
  pub promise: v8::Global<v8::PromiseResolver>,
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

impl JsFuture for FsReadTextFileFuture {
  fn run(&mut self, scope: &mut v8::PinScope) {
    trace!("|FsReadTextFileFuture|");

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
    // Deserialize bytes into string.
    let data = postcard::from_bytes::<String>(&result).unwrap();
    let data = v8::String::new(scope, &data).unwrap();

    self
      .promise
      .open(scope)
      .resolve(scope, data.into())
      .unwrap();
  }
}

fn _get_args<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
) -> String {
  debug_assert!(args.length() == 1);
  debug_assert!(is_v8_str!(args.get(0)));
  let filename = args.get(0).to_rust_string_lossy(scope);
  trace!("RsvimFs readTextFile filename:{:?}", filename);
  filename
}

/// `Rsvim.fs.readTextFile` API.
pub fn read_text_file_async<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  let filename = _get_args(scope, args);

  let promise_resolver = v8::PromiseResolver::new(scope).unwrap();
  let promise = promise_resolver.get_promise(scope);

  let state_rc = JsRuntime::state(scope);
  let read_cb = {
    let promise = v8::Global::new(scope, promise_resolver);
    let state_rc = state_rc.clone();
    move |maybe_result: Option<TheResult<Vec<u8>>>| {
      let fut = FsReadTextFileFuture {
        promise: promise.clone(),
        maybe_result,
      };
      let mut state = state_rc.borrow_mut();
      state.pending_futures.push(Box::new(fut));
    }
  };

  let mut state = state_rc.borrow_mut();
  let task_id = js::TaskId::next();
  pending::create_fs_read_text_file(
    &mut state,
    task_id,
    Path::new(&filename),
    Box::new(read_cb),
  );

  rv.set(promise.into());
}

/// `Rsvim.fs.readTextFileSync` API.
pub fn read_text_file_sync<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  let filename = _get_args(scope, args);

  match fs_read_text_file_s(Path::new(&filename)) {
    Ok(data) => {
      let data = v8::String::new(scope, &data).unwrap();

      rv.set(data.into());
    }
    Err(e) => {
      binding::throw_exception(scope, &e);
    }
  }
}
