//! Write file APIs.

use crate::is_v8_int;
use crate::js;
use crate::js::JsFuture;
use crate::js::JsRuntime;
use crate::js::binding;
use crate::js::converter::*;
use crate::js::pending;
use crate::js::resource::Resource;
use crate::js::resource::ResourceId;
use crate::js::resource::ResourceTableArc;
use crate::prelude::*;
use itertools::Itertools;

pub fn fs_write_s(
  resource_table: ResourceTableArc,
  rid: ResourceId,
  buf: Vec<u8>,
) -> TheResult<usize> {
  use std::io::Write;

  let res = lock!(resource_table).get(&rid).cloned();
  debug_assert!(res.is_some());
  match res.unwrap() {
    Resource::File(res) => {
      let handle = res.data();
      let mut handle = lock!(handle);
      let n = match handle.write(&buf) {
        Ok(n) => n,
        Err(e) => return Err(TheErr::WriteFileByRidFailed(rid, e)),
      };
      debug_assert!(n <= buf.len());
      trace!("|fs_write| n:{},buf:{:?}", n, buf);

      Ok(n)
    }
    _ => unreachable!(),
  }
}

pub struct FsWriteFuture {
  pub promise: v8::Global<v8::PromiseResolver>,
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

impl JsFuture for FsWriteFuture {
  fn run(&mut self, scope: &mut v8::PinScope) {
    trace!("|FsWriteFuture|");

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
    let bytes_written = postcard::from_bytes::<usize>(&data).unwrap();

    let bytes_written = v8::Integer::new(scope, bytes_written as i32);

    self
      .promise
      .open(scope)
      .resolve(scope, bytes_written.into())
      .unwrap();
  }
}

fn _get_args<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
) -> (/* file rid */ ResourceId, /* buf */ Vec<u8>) {
  debug_assert!(args.length() == 2);
  debug_assert!(is_v8_int!(args.get(0)));
  let file_rid = i32::from_v8(scope, args.get(0));
  let file_rid = ResourceId::from(file_rid);
  debug_assert!(args.get(1).is_array_buffer());
  let buf = args.get(1).cast::<v8::ArrayBuffer>();
  let buf = buf
    .get_backing_store()
    .iter()
    .map(|b| b.get())
    .collect_vec();
  trace!("RsvimFs write file_rid:{:?},buf:{:?}", file_rid, buf);
  (file_rid, buf)
}

/// `File.write` API.
pub fn write_async<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  let (file_rid, buf) = _get_args(scope, args);

  let promise_resolver = v8::PromiseResolver::new(scope).unwrap();
  let promise = promise_resolver.get_promise(scope);

  let state_rc = JsRuntime::state(scope);
  let write_cb = {
    let promise = v8::Global::new(scope, promise_resolver);
    let state_rc = state_rc.clone();
    move |maybe_result: Option<TheResult<Vec<u8>>>| {
      let fut = FsWriteFuture {
        promise: promise.clone(),
        maybe_result,
      };
      let mut state = state_rc.borrow_mut();
      state.pending_futures.push(Box::new(fut));
    }
  };

  let mut state = state_rc.borrow_mut();
  let task_id = js::TaskId::next();
  pending::create_fs_write(
    &mut state,
    task_id,
    file_rid,
    buf,
    Box::new(write_cb),
  );

  rv.set(promise.into());
}

/// `File.writeSync` API.
pub fn write_sync<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  let (file_rid, buf) = _get_args(scope, args);

  let state_rc = JsRuntime::state(scope);
  let resource_table = state_rc.borrow().resource_table.clone();

  match fs_write_s(resource_table, file_rid, buf) {
    Ok(bytes_written) => {
      rv.set_int32(bytes_written as i32);
    }
    Err(e) => binding::throw_exception(scope, &e),
  }
}
