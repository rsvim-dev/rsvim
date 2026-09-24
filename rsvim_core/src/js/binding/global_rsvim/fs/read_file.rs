//! Read file APIs.

use crate::is_v8_str;
use crate::js;
use crate::js::JsFuture;
use crate::js::JsRuntime;
use crate::js::binding;
use crate::js::pending;
use crate::prelude::*;

pub fn fs_read_file_s(path: &Path) -> TheResult<Vec<u8>> {
  match std::fs::read(path) {
    Ok(buf) => {
      trace!("path:{:?},buf.len:{}", path, buf.len());
      Ok(buf)
    }
    Err(e) => Err(TheErr::ReadFileByPathFailed(path.to_path_buf(), e)),
  }
}

pub async fn fs_read_file_a(path: &Path) -> TheResult<Vec<u8>> {
  match tokio::fs::read(path).await {
    Ok(buf) => {
      trace!("path:{:?},buf.len:{}", path, buf.len());
      Ok(buf)
    }
    Err(e) => Err(TheErr::ReadFileByPathFailed(path.to_path_buf(), e)),
  }
}

pub struct FsReadFileFuture {
  pub promise: v8::Global<v8::PromiseResolver>,
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

impl JsFuture for FsReadFileFuture {
  fn run(&mut self, scope: &mut v8::PinScope) {
    trace!("|FsReadFileFuture|");

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
    let data = result.unwrap();
    trace!("FsReadFileFuture data.len:{}, data:{:?}", data.len(), data);
    let buf = v8::ArrayBuffer::new(scope, data.len());
    let buffer_store = buf.get_backing_store();

    // Copy the slice's bytes into v8's typed-array backing store.
    for (i, b) in data.iter().enumerate() {
      buffer_store[i].set(*b);
    }

    self.promise.open(scope).resolve(scope, buf.into()).unwrap();
  }
}

fn _get_args<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
) -> String {
  debug_assert!(args.length() == 1);
  debug_assert!(is_v8_str!(args.get(0)));
  let filename = args.get(0).to_rust_string_lossy(scope);
  trace!("RsvimFs readFile filename:{:?}", filename);
  filename
}

/// `Rsvim.fs.readFile` API.
pub fn read_file_async<'s>(
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
      let fut = FsReadFileFuture {
        promise: promise.clone(),
        maybe_result,
      };
      let mut state = state_rc.borrow_mut();
      state.pending_futures.push(Box::new(fut));
    }
  };

  let mut state = state_rc.borrow_mut();
  let task_id = js::TaskId::next();
  pending::create_fs_read_file(
    &mut state,
    task_id,
    Path::new(&filename),
    Box::new(read_cb),
  );

  rv.set(promise.into());
}

/// `Rsvim.fs.readFileSync` API.
pub fn read_file_sync<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  let filename = _get_args(scope, args);

  match fs_read_file_s(Path::new(&filename)) {
    Ok(data) => {
      let buf = v8::ArrayBuffer::new(scope, data.len());
      let buffer_store = buf.get_backing_store();

      // Copy the slice's bytes into v8's typed-array backing store.
      for (i, b) in data.iter().enumerate() {
        buffer_store[i].set(*b);
      }

      rv.set(buf.into());
    }
    Err(e) => {
      binding::throw_exception(scope, &e);
    }
  }
}
