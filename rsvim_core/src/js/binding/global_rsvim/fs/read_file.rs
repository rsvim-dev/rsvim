//! Read file APIs.

use crate::js::JsFuture;
use crate::js::binding;
use crate::prelude::*;

pub fn fs_read_file(path: &Path) -> TheResult<Vec<u8>> {
  match std::fs::read(path) {
    Ok(buf) => {
      trace!("path:{:?},buf.len:{}", path, buf.len());
      Ok(buf)
    }
    Err(e) => Err(TheErr::ReadFileByPathFailed(path.to_path_buf(), e)),
  }
}

pub async fn async_fs_read_file(path: &Path) -> TheResult<Vec<u8>> {
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
