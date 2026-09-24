//! File hard link.

use crate::is_v8_str;
use crate::js;
use crate::js::JsFuture;
use crate::js::JsRuntime;
use crate::js::binding;
use crate::js::pending;
use crate::prelude::*;

pub fn fs_link_s(oldpath: &Path, newpath: &Path) -> TheResult<()> {
  match std::fs::hard_link(oldpath, newpath) {
    Ok(_) => Ok(()),
    Err(e) => Err(TheErr::CreateLinkFailed(
      oldpath.to_path_buf(),
      newpath.to_path_buf(),
      e,
    )),
  }
}

pub async fn fs_link_a(oldpath: &Path, newpath: &Path) -> TheResult<()> {
  match tokio::fs::hard_link(oldpath, newpath).await {
    Ok(_) => Ok(()),
    Err(e) => Err(TheErr::CreateLinkFailed(
      oldpath.to_path_buf(),
      newpath.to_path_buf(),
      e,
    )),
  }
}

pub struct FsLinkFuture {
  pub promise: v8::Global<v8::PromiseResolver>,
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

impl JsFuture for FsLinkFuture {
  fn run(&mut self, scope: &mut v8::PinScope) {
    trace!("|FsLinkFuture|");

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

    // Deserialize bytes into u32 integer.
    let result = postcard::from_bytes::<u32>(&result).unwrap();
    debug_assert_eq!(result, 0);
    let result = v8::undefined(scope);

    self
      .promise
      .open(scope)
      .resolve(scope, result.into())
      .unwrap();
  }
}

fn _get_args<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
) -> (/* oldpath */ String, /* newpath */ String) {
  debug_assert!(args.length() == 2);
  debug_assert!(is_v8_str!(args.get(0)));
  let oldpath = args.get(0).to_rust_string_lossy(scope);
  debug_assert!(is_v8_str!(args.get(1)));
  let newpath = args.get(1).to_rust_string_lossy(scope);
  trace!("RsvimFs link oldpath:{:?},newpath:{:?}", oldpath, newpath);
  (oldpath, newpath)
}

/// `Rsvim.fs.link` API.
pub fn link_async<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  let (oldpath, newpath) = _get_args(scope, args);

  let promise_resolver = v8::PromiseResolver::new(scope).unwrap();
  let promise = promise_resolver.get_promise(scope);

  let state_rc = JsRuntime::state(scope);
  let link_cb = {
    let promise = v8::Global::new(scope, promise_resolver);
    let state_rc = state_rc.clone();
    move |maybe_result: Option<TheResult<Vec<u8>>>| {
      let fut = FsLinkFuture {
        promise: promise.clone(),
        maybe_result,
      };
      let mut state = state_rc.borrow_mut();
      state.pending_futures.push(Box::new(fut));
    }
  };

  let mut state = state_rc.borrow_mut();
  let task_id = js::TaskId::next();
  pending::create_fs_link(
    &mut state,
    task_id,
    Path::new(&oldpath),
    Path::new(&newpath),
    Box::new(link_cb),
  );

  rv.set(promise.into());
}

/// `Rsvim.fs.linkSync` API.
pub fn link_sync<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  let (oldpath, newpath) = _get_args(scope, args);

  match fs_link_s(Path::new(&oldpath), Path::new(&newpath)) {
    Ok(_) => rv.set_undefined(),
    Err(e) => {
      binding::throw_exception(scope, &e);
    }
  }
}
