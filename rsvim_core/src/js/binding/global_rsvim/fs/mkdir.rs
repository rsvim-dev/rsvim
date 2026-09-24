//! Make directory.

use crate::is_v8_str;
use crate::js;
use crate::js::JsFuture;
use crate::js::JsRuntime;
use crate::js::binding;
use crate::js::converter::*;
use crate::js::pending;
use crate::prelude::*;

#[derive(
  Debug,
  Copy,
  Clone,
  PartialEq,
  Eq,
  derive_builder::Builder,
  rsvim_macro::ToV8,
  rsvim_macro::FromV8,
)]
pub struct FsMkdirOptions {
  #[builder(default = false)]
  pub recursive: bool,

  #[builder(default = 0o777)]
  pub mode: u32,
}

pub fn fs_mkdir_s(path: &Path, options: FsMkdirOptions) -> TheResult<()> {
  let mut builder = std::fs::DirBuilder::new();

  builder.recursive(options.recursive);

  #[cfg(target_family = "unix")]
  {
    use std::os::unix::fs::DirBuilderExt;
    builder.mode(options.mode);
  }

  match builder.create(path) {
    Ok(_) => Ok(()),
    Err(e) => Err(TheErr::CreateDirectoryFailed(path.to_path_buf(), e)),
  }
}

pub struct FsMkdirFuture {
  pub promise: v8::Global<v8::PromiseResolver>,
  pub maybe_result: Option<TheResult<Vec<u8>>>,
}

impl JsFuture for FsMkdirFuture {
  fn run(&mut self, scope: &mut v8::PinScope) {
    trace!("|FsMkdirFuture|");

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
) -> (/* path */ String, /* options */ FsMkdirOptions) {
  debug_assert!(args.length() == 2);
  debug_assert!(is_v8_str!(args.get(0)));
  let path = args.get(0).to_rust_string_lossy(scope);
  debug_assert!(args.get(1).is_object());
  let options = FsMkdirOptions::from_v8(scope, args.get(1));
  trace!("RsvimFs mkdir path:{:?},options:{:?}", path, options);
  (path, options)
}

/// `Rsvim.fs.mkdir` API.
pub fn mkdir_async<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  let (path, options) = _get_args(scope, args);

  let promise_resolver = v8::PromiseResolver::new(scope).unwrap();
  let promise = promise_resolver.get_promise(scope);

  let state_rc = JsRuntime::state(scope);
  let link_cb = {
    let promise = v8::Global::new(scope, promise_resolver);
    let state_rc = state_rc.clone();
    move |maybe_result: Option<TheResult<Vec<u8>>>| {
      let fut = FsMkdirFuture {
        promise: promise.clone(),
        maybe_result,
      };
      let mut state = state_rc.borrow_mut();
      state.pending_futures.push(Box::new(fut));
    }
  };

  let mut state = state_rc.borrow_mut();
  let task_id = js::TaskId::next();
  pending::create_fs_mkdir(
    &mut state,
    task_id,
    Path::new(&path),
    options,
    Box::new(link_cb),
  );

  rv.set(promise.into());
}

/// `Rsvim.fs.mkdirSync` API.
pub fn mkdir_sync<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  args: v8::FunctionCallbackArguments<'s>,
  mut rv: v8::ReturnValue,
) {
  let (path, options) = _get_args(scope, args);

  match fs_mkdir_s(Path::new(&path), options) {
    Ok(_) => rv.set_undefined(),
    Err(e) => {
      binding::throw_exception(scope, &e);
    }
  }
}
