//! Read directory APIs.

use crate::is_v8_int;
use crate::is_v8_str;
use crate::js;
use crate::js::JsFuture;
use crate::js::JsRuntime;
use crate::js::binding;
use crate::js::converter::*;
use crate::js::pending;
use crate::js::resource::ResourceId;
use crate::js::resource::ResourceTableArc;
use crate::prelude::*;

#[derive(
  Debug,
  Clone,
  PartialEq,
  Eq,
  derive_builder::Builder,
  serde::Serialize,
  serde::Deserialize,
  rsvim_macro::ToV8,
  rsvim_macro::FromV8,
)]
pub struct FsDirEntry {
  #[builder(default = "".to_string())]
  pub name: String,

  #[builder(default = false)]
  pub is_dir: bool,

  #[builder(default = false)]
  pub is_file: bool,

  #[builder(default = false)]
  pub is_symlink: bool,
}

pub fn fs_read_dir_s(
  resource_table: ResourceTableArc,
  path: &Path,
) -> TheResult<ResourceId> {
  match std::fs::read_dir(path) {
    Ok(rd) => {
      let mut resource_table = lock!(resource_table);
      let rid = resource_table.add_read_dir(rd);
      Ok(rid)
    }
    Err(e) => Err(TheErr::ReadDirectoryByPathFailed(path.to_path_buf(), e)),
  }
}

pub struct FsReadDirFuture {
  pub promise: v8::Global<v8::PromiseResolver>,
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

impl JsFuture for FsReadDirFuture {
  fn run(&mut self, scope: &mut v8::PinScope) {
    trace!("|FsReadDirFuture|");

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
    let rid = postcard::from_bytes::<ResourceId>(&result).unwrap();
    let rid = Into::<i32>::into(rid);
    let rid = rid.to_v8(scope);

    self.promise.open(scope).resolve(scope, rid).unwrap();
  }
}

fn _get_args<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
) -> String {
  debug_assert!(args.length() == 1);
  debug_assert!(is_v8_str!(args.get(0)));
  let filename = args.get(0).to_rust_string_lossy(scope);
  trace!("RsvimFs readDir filename:{:?}", filename);
  filename
}

/// `Rsvim.fs.readDir` API.
pub fn read_dir_async<'s>(
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
      let fut = FsReadDirFuture {
        promise: promise.clone(),
        maybe_result,
      };
      let mut state = state_rc.borrow_mut();
      state.pending_futures.push(Box::new(fut));
    }
  };

  let mut state = state_rc.borrow_mut();
  let task_id = js::TaskId::next();
  pending::create_fs_read_dir(
    &mut state,
    task_id,
    Path::new(&filename),
    Box::new(read_cb),
  );

  rv.set(promise.into());
}

/// `Rsvim.fs.readDirSync` API.
pub fn read_dir_sync<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  let filename = _get_args(scope, args);

  let state_rc = JsRuntime::state(scope);
  let resource_table = state_rc.borrow().resource_table.clone();

  match fs_read_dir_s(resource_table, Path::new(&filename)) {
    Ok(rd_rid) => {
      let rd_rid = Into::<i32>::into(rd_rid);
      let rd_rid = rd_rid.to_v8(scope);
      rv.set(rd_rid);
    }
    Err(e) => {
      binding::throw_exception(scope, &e);
    }
  }
}

pub fn fs_read_dir_next_s(
  resource_table: ResourceTableArc,
  rid: ResourceId,
) -> Option<TheResult<FsDirEntry>> {
  let resource_table = lock!(resource_table);
  let res = resource_table.get(&rid).unwrap();
  match res {
    js::resource::Resource::ReadDirResource(rd) => {
      let rd = rd.data();
      let mut rd = lock!(rd);
      match rd.next() {
        Some(Ok(entry)) => match entry.file_type() {
          Ok(entry_ft) => Some(Ok(FsDirEntry {
            name: entry.file_name().to_string_lossy().to_string(),
            is_file: entry_ft.is_file(),
            is_dir: entry_ft.is_dir(),
            is_symlink: entry_ft.is_symlink(),
          })),
          Err(e) => Some(Err(TheErr::ReadDirectoryByRidFailed(rid, e))),
        },
        Some(Err(e)) => Some(Err(TheErr::ReadDirectoryByRidFailed(rid, e))),
        None => None,
      }
    }
    _ => unreachable!(),
  }
}

pub struct FsReadDirNextFuture {
  pub promise: v8::Global<v8::PromiseResolver>,
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

impl JsFuture for FsReadDirNextFuture {
  fn run(&mut self, scope: &mut v8::PinScope) {
    trace!("|FsReadDirNextFuture|");

    let maybe_result = self.maybe_result.take();

    match maybe_result {
      Some(result) => {
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
        let entry = postcard::from_bytes::<FsDirEntry>(&result).unwrap();
        let entry = entry.to_v8(scope);

        self.promise.open(scope).resolve(scope, entry).unwrap();
      }
      None => {
        let undef = v8::undefined(scope);
        self
          .promise
          .open(scope)
          .resolve(scope, undef.into())
          .unwrap();
      }
    }
  }
}

fn _get_next_args<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
) -> ResourceId {
  debug_assert!(args.length() == 1);
  debug_assert!(is_v8_int!(args.get(0)));
  let rid = i32::from_v8(scope, args.get(0));
  let rid = ResourceId::from(rid);
  trace!("RsvimFs readDir.next rid:{:?}", rid);
  rid
}

/// `Rsvim.fs.readDir` API.
pub fn read_dir_next_async<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  let rid = _get_next_args(scope, args);

  let promise_resolver = v8::PromiseResolver::new(scope).unwrap();
  let promise = promise_resolver.get_promise(scope);

  let state_rc = JsRuntime::state(scope);
  let read_cb = {
    let promise = v8::Global::new(scope, promise_resolver);
    let state_rc = state_rc.clone();
    move |maybe_result: Option<TheResult<Vec<u8>>>| {
      let fut = FsReadDirNextFuture {
        promise: promise.clone(),
        maybe_result,
      };
      let mut state = state_rc.borrow_mut();
      state.pending_futures.push(Box::new(fut));
    }
  };

  let mut state = state_rc.borrow_mut();
  let task_id = js::TaskId::next();
  pending::create_fs_read_dir_next(&mut state, task_id, rid, Box::new(read_cb));

  rv.set(promise.into());
}

/// `Rsvim.fs.readDirSync` API.
pub fn read_dir_next_sync<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  let rid = _get_next_args(scope, args);

  let state_rc = JsRuntime::state(scope);
  let resource_table = state_rc.borrow().resource_table.clone();

  match fs_read_dir_next_s(resource_table, rid) {
    Some(Ok(entry)) => {
      let entry = entry.to_v8(scope);
      rv.set(entry);
    }
    Some(Err(e)) => {
      binding::throw_exception(scope, &e);
    }
    None => {
      rv.set_undefined();
    }
  }
}

/// `Rsvim.fs.readDir` and `Rsvim.fs.readDirSync` API.
pub fn read_dir_close<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut _rv: v8::ReturnValue,
) {
  let rid = _get_next_args(scope, args);

  let state_rc = JsRuntime::state(scope);
  let resource_table = state_rc.borrow().resource_table.clone();
  let mut resource_table = lock!(resource_table);
  let mut rd = resource_table.remove(&rid);
  let _ = rd.take();
}
