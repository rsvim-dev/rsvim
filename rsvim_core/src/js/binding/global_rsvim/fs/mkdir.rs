//! Make directory.

use crate::js::JsFuture;
use crate::js::binding;
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

pub fn fs_mkdir(path: &Path, options: FsMkdirOptions) -> TheResult<()> {
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
