//! Read APIs.

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

pub fn fs_read_s(
  resource_table: ResourceTableArc,
  rid: ResourceId,
  bufsize: usize,
) -> TheResult<Vec<u8>> {
  use std::io::Read;

  let res = lock!(resource_table).get(&rid).cloned();
  debug_assert!(res.is_some());
  match res.unwrap() {
    Resource::File(res) => {
      let handle = res.data();
      let mut handle = lock!(handle);
      let mut buf: Vec<u8> = vec![0; bufsize];
      let n = match handle.read(&mut buf) {
        Ok(n) => n,
        Err(e) => {
          return Err(TheErr::ReadFileByRidFailed(rid, e));
        }
      };
      debug_assert!(n <= buf.capacity());
      unsafe {
        buf.set_len(n);
      }
      trace!("|fs_read| bufsize:{},n:{},buf:{:?}", bufsize, n, buf);

      Ok(buf)
    }
    _ => unreachable!(),
  }
}

pub struct FsReadFuture {
  pub promise: v8::Global<v8::PromiseResolver>,
  pub buffer_store: v8::SharedRef<v8::BackingStore>,
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

impl JsFuture for FsReadFuture {
  fn run(&mut self, scope: &mut v8::PinScope) {
    trace!("|FsReadFuture|");

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

    // Copy the slice's bytes into v8's typed-array backing store.
    for (i, b) in data.iter().enumerate() {
      self.buffer_store[i].set(*b);
    }

    let bytes_read = v8::Integer::new(scope, data.len() as i32);

    self
      .promise
      .open(scope)
      .resolve(scope, bytes_read.into())
      .unwrap();
  }
}

fn _get_args<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
) -> (
  /* file rid */ ResourceId,
  /* buf */ v8::Local<'s, v8::ArrayBuffer>,
) {
  debug_assert!(args.length() == 2);
  debug_assert!(is_v8_int!(args.get(0)));
  let file_rid = i32::from_v8(scope, args.get(0));
  let file_rid = ResourceId::from(file_rid);
  debug_assert!(args.get(1).is_array_buffer());
  let buf = args.get(1).cast::<v8::ArrayBuffer>();
  trace!("RsvimFs read file_rid:{:?},buf:{:?}", file_rid, buf);
  (file_rid, buf)
}

/// `File.read` API.
pub fn read_async<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  let (file_rid, buf) = _get_args(scope, args);

  let promise_resolver = v8::PromiseResolver::new(scope).unwrap();
  let promise = promise_resolver.get_promise(scope);

  let state_rc = JsRuntime::state(scope);
  let read_cb = {
    let promise = v8::Global::new(scope, promise_resolver);
    let state_rc = state_rc.clone();
    let buffer_store = buf.get_backing_store().clone();
    move |maybe_result: Option<TheResult<Vec<u8>>>| {
      let fut = FsReadFuture {
        promise: promise.clone(),
        buffer_store: buffer_store.clone(),
        maybe_result,
      };
      let mut state = state_rc.borrow_mut();
      state.pending_futures.push(Box::new(fut));
    }
  };

  let mut state = state_rc.borrow_mut();
  let task_id = js::TaskId::next();
  pending::create_fs_read(
    &mut state,
    task_id,
    file_rid,
    buf.byte_length(),
    Box::new(read_cb),
  );

  rv.set(promise.into());
}

/// `File.readSync` API.
pub fn read_sync<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  let (file_rid, buf) = _get_args(scope, args);

  let state_rc = JsRuntime::state(scope);
  let resource_table = state_rc.borrow().resource_table.clone();

  match fs_read_s(resource_table, file_rid, buf.byte_length()) {
    Ok(data) => {
      let buffer_store = buf.get_backing_store();
      for (i, b) in data.iter().enumerate() {
        buffer_store[i].set(*b);
      }
      rv.set_int32(data.len() as i32);
    }
    Err(e) => binding::throw_exception(scope, &e),
  }
}
