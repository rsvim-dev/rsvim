//! APIs for the `fs` module — the synchronous half.
//!
//! Every `*Sync` function plus `fs.close` and `fs.readDirClose`. The work itself
//! is in [`crate::js::binding::fs::internal`]; what happens here is argument parsing, turning a
//! `Resourcetable` error into a javascript exception, and boxing results into
//! the shapes `fs.min.js` expects.
//!
//! The promise-returning counterparts are in [`super::fs_async`].

use crate::js::binding::fs::internal::FsDirEntry;
use crate::js::binding::fs::internal::FsMetadata;
use crate::js::binding::fs::internal::FsMkdirOptions;
use crate::js::binding::fs::internal::FsMkdirOptionsBuilder;
use crate::js::binding::fs::internal::FsOpenOptions;
use crate::js::binding::fs::internal::FsOpenOptionsBuilder;
use crate::js::binding::fs::internal::FsSymlinkOptions;
use crate::js::binding::fs::internal::fs_close_s;
use crate::js::binding::fs::internal::fs_link_s;
use crate::js::binding::fs::internal::fs_lstat_s;
use crate::js::binding::fs::internal::fs_mkdir_s;
use crate::js::binding::fs::internal::fs_open_s;
use crate::js::binding::fs::internal::fs_read_dir_close_s;
use crate::js::binding::fs::internal::fs_read_dir_next_s;
use crate::js::binding::fs::internal::fs_read_dir_s;
use crate::js::binding::fs::internal::fs_read_file_s;
use crate::js::binding::fs::internal::fs_read_s;
use crate::js::binding::fs::internal::fs_read_text_file_s;
use crate::js::binding::fs::internal::fs_stat_s;
use crate::js::binding::fs::internal::fs_symlink_s;
use crate::js::binding::fs::internal::fs_write_s;
use crate::js::binding::set_function_to;
use crate::js::converter::array_buffer_to_vec;
use crate::js::converter::copy_into_array_buffer;
use crate::js::converter::vec_to_array_buffer;
use crate::js::resource::ResourceId;
use crate::js::resource::ResourceTableArc;
use crate::js::runtime::kernel;
use crate::prelude::*;
use rquickjs::ArrayBuffer;
use rquickjs::Coerced;
use rquickjs::Ctx;
use rquickjs::Exception;
use rquickjs::Object;
use rquickjs::Result as JsResult;
use std::str::FromStr;
pub mod internal;

#[cfg(test)]
mod internal_tests;

/// The editor's resource table.
pub fn resource_table(ctx: &Ctx<'_>) -> JsResult<ResourceTableArc> {
  Ok(kernel(ctx)?.borrow().editor.resource_table.clone())
}

/// Turns a `TheErr` into a javascript exception.
fn throw(ctx: &Ctx<'_>, e: TheErr) -> rquickjs::Error {
  Exception::throw_message(ctx, &e.to_string())
}

/// Parses the options object `fs.open(path, options)` takes.
///
/// `fs.min.js` always passes a fully populated object (it spreads defaults over
/// the caller's argument), so a missing property here only happens when a test
/// calls the internal binding directly — fall back to the defaults then.
pub fn open_options<'js>(
  ctx: &Ctx<'js>,
  options: Option<Object<'js>>,
) -> JsResult<FsOpenOptions> {
  let mut builder = FsOpenOptionsBuilder::default();
  if let Some(object) = options {
    if let Some(v) = object.get::<&str, Option<bool>>("append")? {
      builder.append(v);
    }
    if let Some(v) = object.get::<&str, Option<bool>>("create")? {
      builder.create(v);
    }
    if let Some(v) = object.get::<&str, Option<bool>>("createNew")? {
      builder.create_new(v);
    }
    if let Some(v) = object.get::<&str, Option<bool>>("read")? {
      builder.read(v);
    }
    if let Some(v) = object.get::<&str, Option<bool>>("truncate")? {
      builder.truncate(v);
    }
    if let Some(v) = object.get::<&str, Option<bool>>("write")? {
      builder.write(v);
    }
  }
  builder.build().map_err(|e| {
    Exception::throw_message(ctx, &format!("fs.open: bad options: {}", e))
  })
}

/// Parses the options object `fs.mkdir(path, options)` takes.
pub fn mkdir_options<'js>(
  ctx: &Ctx<'js>,
  options: Option<Object<'js>>,
) -> JsResult<FsMkdirOptions> {
  let mut builder = FsMkdirOptionsBuilder::default();
  if let Some(object) = options {
    if let Some(v) = object.get::<&str, Option<bool>>("recursive")? {
      builder.recursive(v);
    }
    if let Some(v) = object.get::<&str, Option<i32>>("mode")? {
      builder.mode(v as u32);
    }
  }
  builder.build().map_err(|e| {
    Exception::throw_message(ctx, &format!("fs.mkdir: bad options: {}", e))
  })
}

/// Parses the `type` argument of `fs.symlink` / `fs.symlinkSync`; `api` names
/// the caller in the error.
pub fn symlink_options(
  ctx: &Ctx<'_>,
  api: &str,
  options: Option<String>,
) -> JsResult<FsSymlinkOptions> {
  match options {
    Some(o) => FsSymlinkOptions::from_str(&o).map_err(|_| {
      Exception::throw_range(ctx, &format!("{api}: unknown type {:?}", o))
    }),
    None => Ok(FsSymlinkOptions::File),
  }
}

/// `fs.openSync(path, options)`.
pub fn open_sync<'js>(
  ctx: Ctx<'js>,
  path: String,
  options: Option<Object<'js>>,
) -> JsResult<i32> {
  let options = open_options(&ctx, options)?;
  trace!("fs.openSync path:{:?}, options:{:?}", path, options);
  let resource_table = resource_table(&ctx)?;
  let rid = fs_open_s(resource_table, Path::new(&path), options)
    .map_err(|e| throw(&ctx, e))?;
  Ok(rid.into())
}

/// `fs.close(rid)`.
pub fn close<'js>(ctx: Ctx<'js>, rid: Coerced<i32>) -> JsResult<()> {
  let rid = ResourceId::from(rid.0);
  trace!("fs.close:{:?}", rid);
  fs_close_s(resource_table(&ctx)?, rid);
  Ok(())
}

/// `fs.readSync(rid, buffer)`.
///
/// Reads at most `buffer.byteLength` bytes into the caller's array buffer and
/// returns the count.
pub fn read_sync<'js>(
  ctx: Ctx<'js>,
  rid: Coerced<i32>,
  buffer: ArrayBuffer<'js>,
) -> JsResult<i32> {
  let rid = ResourceId::from(rid.0);
  let size = buffer.len();
  trace!("fs.readSync rid:{:?}, size:{}", rid, size);

  let data =
    fs_read_s(resource_table(&ctx)?, rid, size).map_err(|e| throw(&ctx, e))?;

  let n = copy_into_array_buffer(&buffer, &data).ok_or_else(|| {
    Exception::throw_message(&ctx, "fs.readSync: detached buffer")
  })?;
  Ok(n as i32)
}

/// `fs.writeSync(rid, buffer)`.
pub fn write_sync<'js>(
  ctx: Ctx<'js>,
  rid: Coerced<i32>,
  buffer: ArrayBuffer<'js>,
) -> JsResult<i32> {
  let rid = ResourceId::from(rid.0);
  let data = array_buffer_to_vec(&ctx, buffer.into_value())?;
  trace!("fs.writeSync rid:{:?}, len:{}", rid, data.len());
  let n =
    fs_write_s(resource_table(&ctx)?, rid, data).map_err(|e| throw(&ctx, e))?;
  Ok(n as i32)
}

/// `fs.readDirSync(path)`.
pub fn read_dir_sync<'js>(ctx: Ctx<'js>, path: String) -> JsResult<i32> {
  trace!("fs.readDirSync:{:?}", path);
  let rid = fs_read_dir_s(resource_table(&ctx)?, Path::new(&path))
    .map_err(|e| throw(&ctx, e))?;
  Ok(rid.into())
}

/// `fs.readDirNextSync(rid)`.
///
/// Returns `undefined` once the iterator is exhausted, which `fs.min.js` turns
/// into `{done: true}`.
pub fn read_dir_next_sync<'js>(
  ctx: Ctx<'js>,
  rid: Coerced<i32>,
) -> JsResult<Option<Object<'js>>> {
  let rid = ResourceId::from(rid.0);
  match fs_read_dir_next_s(resource_table(&ctx)?, rid) {
    Some(Ok(entry)) => Ok(Some(dir_entry_to_object(&ctx, &entry)?)),
    Some(Err(e)) => Err(throw(&ctx, e)),
    None => Ok(None),
  }
}

/// `fs.readDirClose(rid)`.
pub fn read_dir_close<'js>(ctx: Ctx<'js>, rid: Coerced<i32>) -> JsResult<()> {
  let rid = ResourceId::from(rid.0);
  trace!("fs.readDirClose:{:?}", rid);
  fs_read_dir_close_s(resource_table(&ctx)?, rid);
  Ok(())
}

pub fn metadata_to_object<'js>(
  ctx: &Ctx<'js>,
  meta: &FsMetadata,
) -> JsResult<Object<'js>> {
  let object = Object::new(ctx.clone())?;
  object.set("accessed", meta.accessed)?;
  object.set("created", meta.created)?;
  object.set("modified", meta.modified)?;
  object.set("isDir", meta.is_dir)?;
  object.set("isFile", meta.is_file)?;
  object.set("isSymlink", meta.is_symlink)?;
  object.set("len", meta.len)?;
  object.set("readOnly", meta.read_only)?;
  object.set("fileAttributes", meta.file_attributes)?;
  object.set("creationTime", meta.creation_time)?;
  object.set("lastAccessTime", meta.last_access_time)?;
  object.set("lastWriteTime", meta.last_write_time)?;
  object.set("fileSize", meta.file_size)?;
  object.set("dev", meta.dev)?;
  object.set("ino", meta.ino)?;
  object.set("mode", meta.mode)?;
  object.set("nlink", meta.nlink)?;
  object.set("uid", meta.uid)?;
  object.set("gid", meta.gid)?;
  object.set("rdev", meta.rdev)?;
  object.set("size", meta.size)?;
  object.set("atime", meta.atime)?;
  object.set("atimeNsec", meta.atime_nsec)?;
  object.set("mtime", meta.mtime)?;
  object.set("mtimeNsec", meta.mtime_nsec)?;
  object.set("ctime", meta.ctime)?;
  object.set("ctimeNsec", meta.ctime_nsec)?;
  object.set("blksize", meta.blksize)?;
  object.set("blocks", meta.blocks)?;
  Ok(object)
}

pub fn dir_entry_to_object<'js>(
  ctx: &Ctx<'js>,
  entry: &FsDirEntry,
) -> JsResult<Object<'js>> {
  let object = Object::new(ctx.clone())?;
  object.set("name", entry.name.clone())?;
  object.set("isDir", entry.is_dir)?;
  object.set("isFile", entry.is_file)?;
  object.set("isSymlink", entry.is_symlink)?;
  Ok(object)
}

/// `fs.readFileSync(path)`.
pub fn read_file_sync<'js>(
  ctx: Ctx<'js>,
  path: String,
) -> JsResult<ArrayBuffer<'js>> {
  let data = fs_read_file_s(Path::new(&path)).map_err(|e| throw(&ctx, e))?;
  vec_to_array_buffer(ctx.clone(), data)
}

/// `fs.readTextFileSync(path)`.
pub fn read_text_file_sync<'js>(
  ctx: Ctx<'js>,
  path: String,
) -> JsResult<String> {
  fs_read_text_file_s(Path::new(&path)).map_err(|e| throw(&ctx, e))
}

/// `fs.lstatSync(path)`.
pub fn lstat_sync<'js>(ctx: Ctx<'js>, path: String) -> JsResult<Object<'js>> {
  let meta = fs_lstat_s(Path::new(&path)).map_err(|e| throw(&ctx, e))?;
  metadata_to_object(&ctx, &meta)
}

/// `fs.statSync(path)`.
pub fn stat_sync<'js>(ctx: Ctx<'js>, path: String) -> JsResult<Object<'js>> {
  let meta = fs_stat_s(Path::new(&path)).map_err(|e| throw(&ctx, e))?;
  metadata_to_object(&ctx, &meta)
}

/// `fs.symlinkSync(oldpath, newpath, options)`.
pub fn symlink_sync<'js>(
  ctx: Ctx<'js>,
  oldpath: String,
  newpath: String,
  options: Option<String>,
) -> JsResult<()> {
  let options = symlink_options(&ctx, "fs.symlinkSync", options)?;
  fs_symlink_s(Path::new(&oldpath), Path::new(&newpath), options)
    .map_err(|e| throw(&ctx, e))
}

/// `fs.linkSync(oldpath, newpath)`.
pub fn link_sync<'js>(
  ctx: Ctx<'js>,
  oldpath: String,
  newpath: String,
) -> JsResult<()> {
  fs_link_s(Path::new(&oldpath), Path::new(&newpath))
    .map_err(|e| throw(&ctx, e))
}

/// `fs.mkdirSync(path, options)`.
pub fn mkdir_sync<'js>(
  ctx: Ctx<'js>,
  path: String,
  options: Option<Object<'js>>,
) -> JsResult<()> {
  let options = mkdir_options(&ctx, options)?;
  fs_mkdir_s(Path::new(&path), options).map_err(|e| throw(&ctx, e))
}

/// Installs the `fs_*` built-ins on `internal`.
pub fn install<'js>(ctx: &Ctx<'js>, internal: &Object<'js>) -> JsResult<()> {
  set_function_to(ctx, internal, "fs_open_sync", open_sync)?;
  set_function_to(ctx, internal, "fs_close", close)?;
  set_function_to(ctx, internal, "fs_read_sync", read_sync)?;
  set_function_to(ctx, internal, "fs_write_sync", write_sync)?;
  set_function_to(ctx, internal, "fs_read_dir_sync", read_dir_sync)?;
  set_function_to(ctx, internal, "fs_read_dir_next_sync", read_dir_next_sync)?;
  set_function_to(ctx, internal, "fs_read_dir_close", read_dir_close)?;
  set_function_to(ctx, internal, "fs_read_file_sync", read_file_sync)?;
  set_function_to(
    ctx,
    internal,
    "fs_read_text_file_sync",
    read_text_file_sync,
  )?;
  set_function_to(ctx, internal, "fs_lstat_sync", lstat_sync)?;
  set_function_to(ctx, internal, "fs_stat_sync", stat_sync)?;
  set_function_to(ctx, internal, "fs_symlink_sync", symlink_sync)?;
  set_function_to(ctx, internal, "fs_link_sync", link_sync)?;
  set_function_to(ctx, internal, "fs_mkdir_sync", mkdir_sync)?;
  Ok(())
}
