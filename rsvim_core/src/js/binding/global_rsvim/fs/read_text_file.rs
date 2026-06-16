//! Read Text file APIs.

use crate::js::JsFuture;
use crate::js::binding;
use crate::prelude::*;

pub fn fs_read_text_file(path: &Path) -> TheResult<String> {
  match std::fs::read_to_string(path) {
    Ok(buf) => Ok(buf),
    Err(e) => Err(TheErr::ReadFileByPathFailed(path.to_path_buf(), e)),
  }
}

pub async fn async_fs_read_text_file(path: &Path) -> TheResult<String> {
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
