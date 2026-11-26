//! Engine-neutral `fs` primitives.
//!
//! The `fs` binding is the largest of the five synchronous modules, and almost
//! none of it is engine specific: opening a file, reading a directory entry or
//! stat-ing a path is the same work whatever runs the javascript. What differs
//! is only how a result is turned into a value and how a failure becomes an
//! exception, which stays in each backend.
//!
//! Everything here is therefore plain Rust, and the two backends re-export it
//! into their own namespaces so that existing call sites keep working:
//!
//! - `crate::js::v8::binding::fs::open::fs_open_s`
//! - `crate::js::rqjs::binding::fs::open::fs_open_s`
//!
//! Both resolve to the function below.

use crate::js::resource::Resource;
use crate::js::resource::ResourceId;
use crate::js::resource::ResourceTableArc;
use crate::prelude::*;
use std::fs::Metadata;
use std::time::SystemTime;

/// File metadata, as `fs.stat`/`fs.lstat` report it.
///
/// `serde` matters: the async bindings ship this across the completion channel
/// as `postcard` bytes, so the field order is a wire format. Append new fields,
/// never reorder.
#[derive(
  Debug,
  Copy,
  Clone,
  PartialEq,
  Eq,
  derive_builder::Builder,
  serde::Serialize,
  serde::Deserialize,
)]
#[cfg_attr(feature = "js-v8", derive(rsvim_macro::ToV8, rsvim_macro::FromV8))]
pub struct FsMetadata {
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

/// One entry of a directory listing.
#[derive(
  Debug,
  Clone,
  PartialEq,
  Eq,
  derive_builder::Builder,
  serde::Serialize,
  serde::Deserialize,
)]
#[cfg_attr(feature = "js-v8", derive(rsvim_macro::ToV8, rsvim_macro::FromV8))]
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

/// Options of `fs.open`/`fs.openSync`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, derive_builder::Builder)]
#[cfg_attr(feature = "js-v8", derive(rsvim_macro::ToV8, rsvim_macro::FromV8))]
// See: <https://doc.rust-lang.org/std/fs/struct.OpenOptions.html>.
pub struct FsOpenOptions {
  #[builder(default = false)]
  pub append: bool,

  #[builder(default = false)]
  pub create: bool,

  #[builder(default = false)]
  pub create_new: bool,

  #[builder(default = false)]
  pub read: bool,

  #[builder(default = false)]
  pub truncate: bool,

  #[builder(default = false)]
  pub write: bool,
}

/// Options of `fs.mkdir`/`fs.mkdirSync`.
#[derive(Debug, Copy, Clone, PartialEq, Eq, derive_builder::Builder)]
#[cfg_attr(feature = "js-v8", derive(rsvim_macro::ToV8, rsvim_macro::FromV8))]
pub struct FsMkdirOptions {
  #[builder(default = false)]
  pub recursive: bool,

  #[builder(default = 0o777)]
  pub mode: u32,
}

/// Options of `fs.symlink`/`fs.symlinkSync`.
#[derive(
  Debug,
  Copy,
  Clone,
  PartialEq,
  Eq,
  PartialOrd,
  Ord,
  Hash,
  strum_macros::Display,
  strum_macros::EnumString,
)]
pub enum FsSymlinkOptions {
  #[strum(serialize = "file")]
  File,

  #[strum(serialize = "dir")]
  Dir,

  #[strum(serialize = "junction")]
  Junction,
}

#[cfg(feature = "js-v8")]
impl crate::js::v8::converter::FromV8 for FsSymlinkOptions {
  fn from_v8<'s>(
    scope: &mut v8::PinScope<'s, '_>,
    value: v8::Local<'s, v8::Value>,
  ) -> Self {
    use std::str::FromStr;
    let result = value.to_string(scope).unwrap().to_rust_string_lossy(scope);
    FsSymlinkOptions::from_str(&result).unwrap()
  }
}

#[cfg(feature = "js-v8")]
impl crate::js::v8::converter::ToV8 for FsSymlinkOptions {
  fn to_v8<'s>(
    &self,
    scope: &mut v8::PinScope<'s, '_>,
  ) -> v8::Local<'s, v8::Value> {
    self.to_string().to_v8(scope)
  }
}

/// `fs.close` body: dropping the entry closes the handle.
pub fn fs_close_s(resource_table: ResourceTableArc, rid: ResourceId) {
  let mut resource_table = lock!(resource_table);
  let mut handle = resource_table.remove(&rid);
  debug_assert!(handle.is_some());
  // Drop file handle, i.e. close the file
  handle.take();
}

/// `fs.linkSync` body.
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

/// `fs.link` body.
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

/// `fs.mkdirSync` body.
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

/// `fs.mkdir` body.
pub async fn fs_mkdir_a(path: &Path, options: FsMkdirOptions) -> TheResult<()> {
  match tokio::fs::DirBuilder::new()
    .recursive(options.recursive)
    .create(path)
    .await
  {
    Ok(_) => Ok(()),
    Err(e) => Err(TheErr::CreateDirectoryFailed(path.to_path_buf(), e)),
  }
}

/// `fs.openSync` body.
pub fn fs_open_s(
  resource_table: ResourceTableArc,
  path: &Path,
  opts: FsOpenOptions,
) -> TheResult<ResourceId> {
  match std::fs::OpenOptions::new()
    .append(opts.append)
    .create(opts.create)
    .create_new(opts.create_new)
    .read(opts.read)
    .truncate(opts.truncate)
    .write(opts.write)
    .open(path)
  {
    Ok(file) => {
      let mut resource_table = lock!(resource_table);
      Ok(resource_table.add_file(file))
    }
    Err(e) => Err(TheErr::OpenFileFailed(path.to_path_buf(), e)),
  }
}

/// `fs.open` body.
pub async fn fs_open_a(
  resource_table: ResourceTableArc,
  path: &Path,
  opts: FsOpenOptions,
) -> TheResult<ResourceId> {
  match tokio::fs::OpenOptions::new()
    .append(opts.append)
    .create(opts.create)
    .create_new(opts.create_new)
    .read(opts.read)
    .truncate(opts.truncate)
    .write(opts.write)
    .open(path)
    .await
  {
    Ok(file) => {
      let file = file.into_std().await;
      let mut resource_table = lock!(resource_table);
      Ok(resource_table.add_file(file))
    }
    Err(e) => Err(TheErr::OpenFileFailed(path.to_path_buf(), e)),
  }
}

/// `fs.readSync`/`fs.read` body.
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

/// `fs.readDirSync` body.
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

/// `fs.readDirNextSync` body.
pub fn fs_read_dir_next_s(
  resource_table: ResourceTableArc,
  rid: ResourceId,
) -> Option<TheResult<FsDirEntry>> {
  let resource_table = lock!(resource_table);
  let res = resource_table.get(&rid).unwrap();
  match res {
    Resource::ReadDirResource(rd) => {
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

/// Drops a directory handle, which stops the underlying `ReadDir` iterator.
pub fn fs_read_dir_close_s(resource_table: ResourceTableArc, rid: ResourceId) {
  let mut resource_table = lock!(resource_table);
  let mut rd = resource_table.remove(&rid);
  let _ = rd.take();
}

/// `fs.readFileSync` body.
pub fn fs_read_file_s(path: &Path) -> TheResult<Vec<u8>> {
  match std::fs::read(path) {
    Ok(buf) => {
      trace!("path:{:?},buf.len:{}", path, buf.len());
      Ok(buf)
    }
    Err(e) => Err(TheErr::ReadFileByPathFailed(path.to_path_buf(), e)),
  }
}

/// `fs.readFile` body.
pub async fn fs_read_file_a(path: &Path) -> TheResult<Vec<u8>> {
  match tokio::fs::read(path).await {
    Ok(buf) => {
      trace!("path:{:?},buf.len:{}", path, buf.len());
      Ok(buf)
    }
    Err(e) => Err(TheErr::ReadFileByPathFailed(path.to_path_buf(), e)),
  }
}

/// `fs.readTextFileSync` body.
pub fn fs_read_text_file_s(path: &Path) -> TheResult<String> {
  match std::fs::read_to_string(path) {
    Ok(buf) => Ok(buf),
    Err(e) => Err(TheErr::ReadFileByPathFailed(path.to_path_buf(), e)),
  }
}

/// `fs.readTextFile` body.
pub async fn fs_read_text_file_a(path: &Path) -> TheResult<String> {
  match tokio::fs::read_to_string(path).await {
    Ok(buf) => Ok(buf),
    Err(e) => Err(TheErr::ReadFileByPathFailed(path.to_path_buf(), e)),
  }
}

/// `fs.writeSync`/`fs.write` body.
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

/// `lstat` does not follow a symlink.
pub fn fs_lstat_s(path: &Path) -> TheResult<FsMetadata> {
  match std::fs::symlink_metadata(path) {
    Ok(meta) => Ok(convert_metadata(meta)),
    Err(e) => Err(TheErr::ReadFileByPathFailed(path.to_path_buf(), e)),
  }
}

/// `lstat` does not follow a symlink.
pub async fn fs_lstat_a(path: &Path) -> TheResult<FsMetadata> {
  match tokio::fs::symlink_metadata(path).await {
    Ok(meta) => Ok(convert_metadata(meta)),
    Err(e) => Err(TheErr::ReadFileByPathFailed(path.to_path_buf(), e)),
  }
}

/// `stat` follows a symlink.
pub fn fs_stat_s(path: &Path) -> TheResult<FsMetadata> {
  match std::fs::metadata(path) {
    Ok(meta) => Ok(convert_metadata(meta)),
    Err(e) => Err(TheErr::ReadFileByPathFailed(path.to_path_buf(), e)),
  }
}

/// `stat` follows a symlink.
pub async fn fs_stat_a(path: &Path) -> TheResult<FsMetadata> {
  match tokio::fs::metadata(path).await {
    Ok(meta) => Ok(convert_metadata(meta)),
    Err(e) => Err(TheErr::ReadFileByPathFailed(path.to_path_buf(), e)),
  }
}

/// Fills `FsMetadata` from a `std::fs::Metadata`.
pub fn convert_metadata(meta: Metadata) -> FsMetadata {
  let mut builder = FsMetadataBuilder::default();
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

/// Creates the directory a symlink points at.
#[cfg(target_family = "unix")]
pub fn fs_symlink_s(
  oldpath: &Path,
  newpath: &Path,
  _options: FsSymlinkOptions,
) -> TheResult<()> {
  match std::os::unix::fs::symlink(oldpath, newpath) {
    Ok(_) => Ok(()),
    Err(e) => Err(TheErr::CreateSymlinkFailed(
      oldpath.to_path_buf(),
      newpath.to_path_buf(),
      e,
    )),
  }
}

/// Creates a symlink; on Windows the flavour comes from `options`.
#[cfg(target_family = "windows")]
pub fn fs_symlink_s(
  oldpath: &Path,
  newpath: &Path,
  options: FsSymlinkOptions,
) -> TheResult<()> {
  match options {
    FsSymlinkOptions::File => {
      match std::os::windows::fs::symlink_file(oldpath, newpath) {
        Ok(_) => Ok(()),
        Err(e) => Err(TheErr::CreateSymlinkFailed(
          oldpath.to_path_buf(),
          newpath.to_path_buf(),
          e,
        )),
      }
    }
    FsSymlinkOptions::Dir => {
      match std::os::windows::fs::symlink_dir(oldpath, newpath) {
        Ok(_) => Ok(()),
        Err(e) => Err(TheErr::CreateSymlinkFailed(
          oldpath.to_path_buf(),
          newpath.to_path_buf(),
          e,
        )),
      }
    }
    FsSymlinkOptions::Junction => match junction::create(oldpath, newpath) {
      Ok(_) => Ok(()),
      Err(e) => Err(TheErr::CreateSymlinkFailed(
        oldpath.to_path_buf(),
        newpath.to_path_buf(),
        e,
      )),
    },
  }
}

/// Creates a symlink.
pub async fn fs_symlink_a(
  oldpath: &Path,
  newpath: &Path,
  options: FsSymlinkOptions,
) -> TheResult<()> {
  #[cfg(target_family = "unix")]
  {
    let _ = options;
    fs_symlink_s(oldpath, newpath, FsSymlinkOptions::File)
  }
  #[cfg(target_family = "windows")]
  {
    // `tokio::fs::symlink*` picks the flavour from the target's metadata, which
    // is exactly what the option means here.
    let result = match options {
      FsSymlinkOptions::File => tokio::fs::symlink_file(oldpath, newpath).await,
      FsSymlinkOptions::Dir | FsSymlinkOptions::Junction => {
        tokio::fs::symlink_dir(oldpath, newpath).await
      }
    };
    match result {
      Ok(_) => Ok(()),
      Err(e) => Err(TheErr::CreateSymlinkFailed(
        oldpath.to_path_buf(),
        newpath.to_path_buf(),
        e,
      )),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::js::resource::ResourceTable;
  use assert_fs::prelude::*;

  #[test]
  #[cfg_attr(miri, ignore)]
  fn open_read_write_close_round_trip() {
    crate::tests::log::init();
    let tmpfile = assert_fs::NamedTempFile::new("fs-neutral.txt").unwrap();
    let resource_table = ResourceTable::to_arc(ResourceTable::new());

    let rid = fs_open_s(
      resource_table.clone(),
      tmpfile.path(),
      FsOpenOptionsBuilder::default()
        .create(true)
        .write(true)
        .read(true)
        .build()
        .unwrap(),
    )
    .unwrap();

    let written =
      fs_write_s(resource_table.clone(), rid, b"hello".to_vec()).unwrap();
    assert_eq!(written, 5);

    // Re-open for reading: the handle above is positioned at the end.
    fs_close_s(resource_table.clone(), rid);
    assert_eq!(fs_read_file_s(tmpfile.path()).unwrap(), b"hello".to_vec());
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn read_dir_lists_the_temporary_directory() {
    crate::tests::log::init();
    let dir = assert_fs::TempDir::new().unwrap();
    dir.child("a.txt").touch().unwrap();
    let resource_table = ResourceTable::to_arc(ResourceTable::new());

    let rid = fs_read_dir_s(resource_table.clone(), dir.path()).unwrap();
    let name = fs_read_dir_next_s(resource_table.clone(), rid)
      .expect("one entry")
      .unwrap();
    assert_eq!(name.name, "a.txt");
    assert!(name.is_file);

    // The iterator is exhausted after the single entry.
    assert!(fs_read_dir_next_s(resource_table.clone(), rid).is_none());
    fs_read_dir_close_s(resource_table, rid);
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn stat_reports_a_directory() {
    crate::tests::log::init();
    let dir = assert_fs::TempDir::new().unwrap();
    let meta = fs_stat_s(dir.path()).unwrap();
    assert!(meta.is_dir);
    assert!(!meta.is_file);
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn lstat_on_a_missing_path_is_an_error() {
    crate::tests::log::init();
    let result = fs_lstat_s(Path::new("/nonexistent/rsvim-neutral-test"));
    assert!(result.is_err());
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn mkdir_then_remove() {
    crate::tests::log::init();
    let dir = assert_fs::TempDir::new().unwrap();
    let nested = dir.path().join("a").join("b");
    fs_mkdir_s(
      &nested,
      FsMkdirOptionsBuilder::default()
        .recursive(true)
        .build()
        .unwrap(),
    )
    .unwrap();
    assert!(nested.is_dir());
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn link_and_symlink() {
    crate::tests::log::init();
    let dir = assert_fs::TempDir::new().unwrap();
    let target = dir.child("target.txt");
    target.write_str("x").unwrap();
    let hard = dir.path().join("hard.txt");
    fs_link_s(target.path(), &hard).unwrap();
    assert!(hard.is_file());

    let soft = dir.path().join("soft.txt");
    fs_symlink_s(target.path(), &soft, FsSymlinkOptions::File).unwrap();
    assert!(soft.symlink_metadata().unwrap().is_symlink());
  }
}
