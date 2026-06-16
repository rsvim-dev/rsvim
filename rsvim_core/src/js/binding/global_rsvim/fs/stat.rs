//! Get file path metadata.

use crate::js::JsFuture;
use crate::js::binding;
use crate::js::converter::*;
use crate::prelude::*;
use std::fs::Metadata;
use std::time::SystemTime;

#[derive(
  Debug,
  Copy,
  Clone,
  PartialEq,
  Eq,
  derive_builder::Builder,
  serde::Serialize,
  serde::Deserialize,
  rsvim_macro::ToV8,
  rsvim_macro::FromV8,
)]
pub struct FsFileInfo {
  #[builder(default = None)]
  pub accessed: Option<SystemTime>,

  #[builder(default = None)]
  pub created: Option<SystemTime>,

  #[builder(default = None)]
  pub modified: Option<SystemTime>,

  #[builder(default = false)]
  pub is_dir: bool,

  #[builder(default = false)]
  pub is_file: bool,

  #[builder(default = false)]
  pub is_symlink: bool,

  #[builder(default = 0_u64)]
  pub len: u64,

  #[builder(default = false)]
  pub read_only: bool,

  // Windows only {{{
  #[builder(default = None)]
  pub file_attributes: Option<u32>,

  #[builder(default = None)]
  pub creation_time: Option<u64>,

  #[builder(default = None)]
  pub last_access_time: Option<u64>,

  #[builder(default = None)]
  pub last_write_time: Option<u64>,

  #[builder(default = None)]
  pub file_size: Option<u64>,
  // Windows only }}}

  // Unix only {{{
  #[builder(default = None)]
  pub dev: Option<u64>,

  #[builder(default = None)]
  pub ino: Option<u64>,

  #[builder(default = None)]
  pub mode: Option<u32>,

  #[builder(default = None)]
  pub nlink: Option<u64>,

  #[builder(default = None)]
  pub uid: Option<u32>,

  #[builder(default = None)]
  pub gid: Option<u32>,

  #[builder(default = None)]
  pub rdev: Option<u64>,

  #[builder(default = None)]
  pub size: Option<u64>,

  #[builder(default = None)]
  pub atime: Option<i64>,

  #[builder(default = None)]
  pub atime_nsec: Option<i64>,

  #[builder(default = None)]
  pub mtime: Option<i64>,

  #[builder(default = None)]
  pub mtime_nsec: Option<i64>,

  #[builder(default = None)]
  pub ctime: Option<i64>,

  #[builder(default = None)]
  pub ctime_nsec: Option<i64>,

  #[builder(default = None)]
  pub blksize: Option<u64>,

  #[builder(default = None)]
  pub blocks: Option<u64>,
  // Unix only }}}
}

fn convert_metadata_to_fileinfo(meta: Metadata) -> FsFileInfo {
  let mut builder = FsFileInfoBuilder::default();
  builder.accessed(meta.accessed().ok());
  builder.created(meta.created().ok());
  builder.modified(meta.modified().ok());
  builder.is_dir(meta.is_dir());
  builder.is_file(meta.is_file());
  builder.is_symlink(meta.is_symlink());
  builder.len(meta.len());
  builder.read_only(meta.permissions().readonly());

  #[cfg(target_family = "windows")]
  {
    use std::os::windows::fs::MetadataExt;
    builder.file_attributes(Some(meta.file_attributes()));
    builder.creation_time(Some(meta.creation_time()));
    builder.last_access_time(Some(meta.last_access_time()));
    builder.last_write_time(Some(meta.last_write_time()));
    builder.file_size(Some(meta.file_size()));
  }

  #[cfg(target_family = "unix")]
  {
    use std::os::unix::fs::MetadataExt;
    builder.dev(Some(meta.dev()));
    builder.ino(Some(meta.ino()));
    builder.mode(Some(meta.mode()));
    builder.nlink(Some(meta.nlink()));
    builder.uid(Some(meta.uid()));
    builder.gid(Some(meta.gid()));
    builder.rdev(Some(meta.rdev()));
    builder.size(Some(meta.size()));
    builder.atime(Some(meta.atime()));
    builder.atime_nsec(Some(meta.atime_nsec()));
    builder.mtime(Some(meta.mtime()));
    builder.mtime_nsec(Some(meta.mtime_nsec()));
    builder.ctime(Some(meta.ctime()));
    builder.ctime_nsec(Some(meta.ctime_nsec()));
    builder.blksize(Some(meta.blksize()));
    builder.blocks(Some(meta.blocks()));
  }

  builder.build().unwrap()
}

// lstat doesn't follow symlink
pub fn fs_lstat(path: &Path) -> TheResult<FsFileInfo> {
  match std::fs::symlink_metadata(path) {
    Ok(meta) => Ok(convert_metadata_to_fileinfo(meta)),
    Err(e) => Err(TheErr::ReadFileByPathFailed(path.to_path_buf(), e)),
  }
}

// lstat doesn't follow symlink
pub async fn async_fs_lstat(path: &Path) -> TheResult<FsFileInfo> {
  match tokio::fs::symlink_metadata(path).await {
    Ok(meta) => Ok(convert_metadata_to_fileinfo(meta)),
    Err(e) => Err(TheErr::ReadFileByPathFailed(path.to_path_buf(), e)),
  }
}

// stat follows symlink
pub fn fs_stat(path: &Path) -> TheResult<FsFileInfo> {
  match std::fs::metadata(path) {
    Ok(meta) => Ok(convert_metadata_to_fileinfo(meta)),
    Err(e) => Err(TheErr::ReadFileByPathFailed(path.to_path_buf(), e)),
  }
}

// stat follows symlink
pub async fn async_fs_stat(path: &Path) -> TheResult<FsFileInfo> {
  match tokio::fs::metadata(path).await {
    Ok(meta) => Ok(convert_metadata_to_fileinfo(meta)),
    Err(e) => Err(TheErr::ReadFileByPathFailed(path.to_path_buf(), e)),
  }
}

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
    let file_info = postcard::from_bytes::<FsFileInfo>(&result).unwrap();
    let file_info = file_info.to_v8(scope);

    self.promise.open(scope).resolve(scope, file_info).unwrap();
  }
}
