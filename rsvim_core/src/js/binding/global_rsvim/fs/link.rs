//! File symbolic link.

use crate::js::JsFuture;
use crate::js::binding;
use crate::prelude::*;
use compact_str::ToCompactString;

pub fn fs_link(oldpath: &Path, newpath: &Path) -> TheResult<()> {
  match std::fs::hard_link(oldpath, newpath) {
    Ok(_) => Ok(()),
    Err(e) => Err(TheErr::CreateSymlinkFailed(
      oldpath.to_string_lossy().to_compact_string(),
      newpath.to_string_lossy().to_compact_string(),
      e,
    )),
  }
}

pub async fn async_fs_link(oldpath: &Path, newpath: &Path) -> TheResult<()> {
  match tokio::fs::hard_link(oldpath, newpath).await {
    Ok(_) => Ok(()),
    Err(e) => Err(TheErr::CreateSymlinkFailed(
      oldpath.to_string_lossy().to_compact_string(),
      newpath.to_string_lossy().to_compact_string(),
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
