//! APIs for the `fs` module — the synchronous half.
//!
//! Every `*Sync` function plus `fs.close` and `fs.readDirClose`. The work itself
//! is in [`crate::js::fs`]; what happens here is argument parsing, turning a
//! `Resourcetable` error into a javascript exception, and boxing results into
//! the shapes `fs.min.js` expects.
//!
//! The promise-returning counterparts are in [`super::fs_async`].

use crate::js::binding::set_function_to;
use crate::js::converter::array_buffer_to_vec;
use crate::js::converter::copy_into_array_buffer;
use crate::js::converter::vec_to_array_buffer;
use crate::js::fs::FsDirEntry;
use crate::js::fs::FsMetadata;
use crate::js::fs::FsMkdirOptions;
use crate::js::fs::FsMkdirOptionsBuilder;
use crate::js::fs::FsOpenOptions;
use crate::js::fs::FsOpenOptionsBuilder;
use crate::js::fs::FsSymlinkOptions;
use crate::js::fs::fs_close_s;
use crate::js::fs::fs_link_s;
use crate::js::fs::fs_lstat_s;
use crate::js::fs::fs_mkdir_s;
use crate::js::fs::fs_open_s;
use crate::js::fs::fs_read_dir_close_s;
use crate::js::fs::fs_read_dir_next_s;
use crate::js::fs::fs_read_dir_s;
use crate::js::fs::fs_read_file_s;
use crate::js::fs::fs_read_s;
use crate::js::fs::fs_read_text_file_s;
use crate::js::fs::fs_stat_s;
use crate::js::fs::fs_symlink_s;
use crate::js::fs::fs_write_s;
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

#[cfg(test)]
mod tests {
  use crate::tests::js::with_kernel;
  use crate::tests::log::init as test_log_init;

  #[test]
  #[cfg_attr(miri, ignore)]
  fn open_write_read_close_roundtrip() {
    test_log_init();
    let file = assert_fs::NamedTempFile::new("rqjs-fs.txt").unwrap();
    let path = file.path().to_string_lossy().to_string();
    with_kernel(|ctx, _kernel| {
      let src = format!(
        "const rid = __InternalRsvimGlobalObject.fs_open_sync({path:?}, {{create:true,read:true,write:true,truncate:true}});
        const buf = new Uint8Array([72, 101, 108, 108, 111]).buffer;
        const n = __InternalRsvimGlobalObject.fs_write_sync(rid, buf);
        __InternalRsvimGlobalObject.fs_close(rid);
        n;"
      );
      let n: i32 = ctx.eval(src).unwrap();
      assert_eq!(n, 5);
    });
    assert_eq!(std::fs::read_to_string(file.path()).unwrap(), "Hello");
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn read_sync_fills_the_caller_buffer() {
    test_log_init();
    let file = assert_fs::NamedTempFile::new("rqjs-fs-read.txt").unwrap();
    std::fs::write(file.path(), "abcdef").unwrap();
    let path = file.path().to_string_lossy().to_string();
    with_kernel(|ctx, _kernel| {
      let src = format!(
        "const rid = __InternalRsvimGlobalObject.fs_open_sync({path:?}, {{read:true}});
        const buf = new ArrayBuffer(6);
        const n = __InternalRsvimGlobalObject.fs_read_sync(rid, buf);
        __InternalRsvimGlobalObject.fs_close(rid);
        const view = new Uint8Array(buf);
        globalThis.__n = n;
        globalThis.__text = Array.from(view).map(c => String.fromCharCode(c)).join('');"
      );
      ctx.eval::<(), _>(src).unwrap();
      let n: i32 = ctx.eval("globalThis.__n").unwrap();
      let text: String = ctx.eval("globalThis.__text").unwrap();
      assert_eq!(n, 6);
      assert_eq!(text, "abcdef");
    });
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn stat_sync_reports_a_file() {
    test_log_init();
    let file = assert_fs::NamedTempFile::new("rqjs-fs-stat.txt").unwrap();
    std::fs::write(file.path(), "1234").unwrap();
    let path = file.path().to_string_lossy().to_string();
    with_kernel(|ctx, _kernel| {
      let src = format!(
        "const meta = __InternalRsvimGlobalObject.fs_stat_sync({path:?});
        globalThis.__meta = meta;"
      );
      ctx.eval::<(), _>(src).unwrap();
      let is_file: bool = ctx.eval("globalThis.__meta.isFile").unwrap();
      let is_dir: bool = ctx.eval("globalThis.__meta.isDir").unwrap();
      let len: u64 = ctx.eval("globalThis.__meta.len").unwrap();
      assert!(is_file);
      assert!(!is_dir);
      assert_eq!(len, 4);
    });
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn read_dir_sync_lists_entries() {
    test_log_init();
    let dir = assert_fs::TempDir::new().unwrap();
    std::fs::write(dir.path().join("a.txt"), "").unwrap();
    let path = dir.path().to_string_lossy().to_string();
    with_kernel(|ctx, _kernel| {
      let src = format!(
        "const rid = __InternalRsvimGlobalObject.fs_read_dir_sync({path:?});
        const names = [];
        for (;;) {{
          const entry = __InternalRsvimGlobalObject.fs_read_dir_next_sync(rid);
          if (entry === undefined) {{ break; }}
          names.push(entry.name);
        }}
        __InternalRsvimGlobalObject.fs_read_dir_close(rid);
        names;"
      );
      let names: Vec<String> = ctx.eval(src).unwrap();
      assert_eq!(names, vec!["a.txt".to_string()]);
    });
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn missing_file_is_a_javascript_error() {
    test_log_init();
    with_kernel(|ctx, _kernel| {
      let result = ctx.eval::<rquickjs::Object, _>(
        "__InternalRsvimGlobalObject.fs_stat_sync('/nonexistent/rsvim/fs')",
      );
      assert!(result.is_err());
    });
  }
}
