//! File stat.

// The engine-neutral implementation lives in `crate::js::fs`.
pub use crate::js::fs::{
  FsMetadata, fs_lstat_a, fs_lstat_s, fs_stat_a, fs_stat_s,
};

use crate::is_v8_str;
use crate::js;
use crate::js::JsFuture;
use crate::js::binding;
use crate::js::converter::*;
use crate::js::pending;
use crate::prelude::*;

// lstat doesn't follow symlink

// lstat doesn't follow symlink

// stat follows symlink

// stat follows symlink

pub struct FsStatFuture {
  pub promise: v8::Global<v8::PromiseResolver>,
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

impl JsFuture for FsStatFuture {
  fn run(&mut self, scope: &mut v8::PinScope) {
    trace!("|FsStatFuture|");

    let result = self.maybe_result.take().unwrap();

    // Handle when something goes wrong with it.
    if let Err(e) = result {
      let message = v8::String::new(scope, &e.to_string()).unwrap();
      let exception = v8::Exception::error(scope, message);
      binding::set_exception_code(scope, exception, &e);
      self.promise.open(scope).reject(scope, exception);
      return;
    }

    // Otherwise, get the result and deserialize it.
    let result = result.unwrap();

    // Deserialize bytes into file info.
    let file_info = postcard::from_bytes::<FsMetadata>(&result).unwrap();
    let file_info = file_info.to_v8(scope);

    self.promise.open(scope).resolve(scope, file_info).unwrap();
  }
}

fn _get_args<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
) -> String {
  debug_assert!(args.length() == 1);
  debug_assert!(is_v8_str!(args.get(0)));
  let filename = args.get(0).to_rust_string_lossy(scope);
  trace!("fs lstat/stat filename:{:?}", filename);
  filename
}

/// `fs.lstat` API.
pub fn lstat_async<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  let filename = _get_args(scope, args);

  let promise_resolver = v8::PromiseResolver::new(scope).unwrap();
  let promise = promise_resolver.get_promise(scope);

  let state_rc = crate::js::get_state(scope);
  let stat_cb = {
    let promise = v8::Global::new(scope, promise_resolver);
    let state_rc = state_rc.clone();
    move |maybe_result: Option<TheResult<Vec<u8>>>| {
      let fut = FsStatFuture {
        promise: promise.clone(),
        maybe_result,
      };
      let mut state = state_rc.borrow_mut();
      state.pending_futures.push(Box::new(fut));
    }
  };

  let mut state = state_rc.borrow_mut();
  let task_id = js::TaskId::next();
  pending::create_fs_stat(
    &mut state,
    task_id,
    false,
    Path::new(&filename),
    Box::new(stat_cb),
  );

  rv.set(promise.into());
}

/// `fs.lstatSync` API.
pub fn lstat_sync<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  let filename = _get_args(scope, args);

  match fs_lstat_s(Path::new(&filename)) {
    Ok(info) => {
      let info = info.to_v8(scope);
      rv.set(info);
    }
    Err(e) => {
      binding::throw_exception(scope, &e);
    }
  }
}

/// `fs.stat` API.
pub fn stat_async<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  let filename = _get_args(scope, args);

  let promise_resolver = v8::PromiseResolver::new(scope).unwrap();
  let promise = promise_resolver.get_promise(scope);

  let state_rc = crate::js::get_state(scope);
  let stat_cb = {
    let promise = v8::Global::new(scope, promise_resolver);
    let state_rc = state_rc.clone();
    move |maybe_result: Option<TheResult<Vec<u8>>>| {
      let fut = FsStatFuture {
        promise: promise.clone(),
        maybe_result,
      };
      let mut state = state_rc.borrow_mut();
      state.pending_futures.push(Box::new(fut));
    }
  };

  let mut state = state_rc.borrow_mut();
  let task_id = js::TaskId::next();
  pending::create_fs_stat(
    &mut state,
    task_id,
    true,
    Path::new(&filename),
    Box::new(stat_cb),
  );

  rv.set(promise.into());
}

/// `fs.statSync` API.
pub fn stat_sync<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  let filename = _get_args(scope, args);

  match fs_stat_s(Path::new(&filename)) {
    Ok(info) => {
      let info = info.to_v8(scope);
      rv.set(info);
    }
    Err(e) => {
      binding::throw_exception(scope, &e);
    }
  }
}
