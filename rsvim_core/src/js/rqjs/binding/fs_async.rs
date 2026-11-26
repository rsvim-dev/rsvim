//! APIs for the `fs` module — the promise-returning half.
//!
//! Every function here answers with a promise instead of a value, which is
//! what `fs.min.js` awaits. The work itself is in [`crate::js::fs`]: either a
//! `tokio::fs` coroutine (`fs_*_a`) or a blocking call (`fs_read_s`,
//! `fs_write_s`) that has to run off the event loop thread.
//!
//! # Why the blocking half runs on `blocked_tracker`
//!
//! A write is not abortable: by the time `await file.write(buf)` returns to
//! javascript the bytes are already promised to the user. Exiting while that
//! write sits in a queue would truncate the file silently. So it goes through
//! `JsKernel::blocked_tracker`, the clone of `Rsvim::blocked_tracker` that
//! `close_trackers` closes and waits on -- the same guarantee the v8 backend
//! gets, through a completely different route.

use crate::js::fs::FsDirEntry;
use crate::js::fs::FsMetadata;
use crate::js::fs::FsMkdirOptions;
use crate::js::fs::FsMkdirOptionsBuilder;
use crate::js::fs::FsOpenOptions;
use crate::js::fs::FsOpenOptionsBuilder;
use crate::js::fs::FsSymlinkOptions;
use crate::js::fs::fs_link_a;
use crate::js::fs::fs_lstat_a;
use crate::js::fs::fs_mkdir_a;
use crate::js::fs::fs_open_a;
use crate::js::fs::fs_read_file_a;
use crate::js::fs::fs_read_s;
use crate::js::fs::fs_read_text_file_a;
use crate::js::fs::fs_stat_a;
use crate::js::fs::fs_symlink_a;
use crate::js::fs::fs_write_s;
use crate::js::resource::ResourceId;
use crate::js::rqjs::binding::set_function_to;
use crate::js::rqjs::converter::array_buffer_to_vec;
use crate::js::rqjs::converter::vec_to_array_buffer;
use crate::js::rqjs::get_state;
use crate::prelude::*;
use rquickjs::ArrayBuffer;
use rquickjs::Ctx;
use rquickjs::Exception;
use rquickjs::Object;
use rquickjs::Promise;
use rquickjs::Result as JsResult;
use std::str::FromStr;

/// Reads the kernel's `resource_table`, or raises if there is no kernel.
macro_rules! resource_table {
  ($ctx:expr) => {{
    let kernel = get_state($ctx).ok_or_else(|| {
      Exception::throw_message($ctx, "js kernel is not initialised")
    })?;
    let table = kernel.borrow().editor.resource_table.clone();
    table
  }};
}

/// Turns a `TheErr` into a javascript exception.
fn throw(ctx: &Ctx<'_>, e: TheErr) -> rquickjs::Error {
  Exception::throw_message(ctx, &e.to_string())
}

/// `fs.open(path, options)` -> `Promise<number>`.
///
/// Resolves to the resource id `fs.min.js` wraps in its `File` class.
pub fn open<'js>(
  ctx: Ctx<'js>,
  path: String,
  options: Option<Object<'js>>,
) -> JsResult<Promise<'js>> {
  let table = resource_table!(&ctx);
  let opts = open_options(&ctx, options)?;
  trace!("fs.open:{:?}", path);
  let ctx_err = ctx.clone();
  Promise::wrap_future(&ctx, async move {
    let rid = fs_open_a(table, Path::new(&path), opts)
      .await
      .map_err(|e| throw(&ctx_err, e))?;
    Ok::<i32, rquickjs::Error>(rid.into())
  })
}

/// `fs.read(rid, buffer)` -> `Promise<number>`.
///
/// Reads at most `buffer.byteLength` bytes into the caller's array buffer and
/// resolves to the count. Writing in place needs the raw backing store because
/// rquickjs has no `as_mut_bytes`.
pub fn read<'js>(
  ctx: Ctx<'js>,
  rid: i32,
  buffer: ArrayBuffer<'js>,
) -> JsResult<Promise<'js>> {
  let table = resource_table!(&ctx);
  let rid = ResourceId::from(rid);
  let size = buffer.len();
  trace!("fs.read rid:{:?}, size:{}", rid, size);
  let tracker = blocked_tracker(&ctx);
  let ctx_err = ctx.clone();
  Promise::wrap_future(&ctx, async move {
    let data = tracker
      .spawn_blocking(move || fs_read_s(table, rid, size))
      .await
      .map_err(|e| {
        Exception::throw_message(&ctx_err, &format!("fs.read task: {}", e))
      })?
      .map_err(|e| throw(&ctx_err, e))?;

    // SAFETY: `as_raw` hands back the backing store of a live array buffer.
    // Nothing else touches this isolate between taking the pointer and
    // finishing the copy, which is the condition the API documents.
    match buffer.as_raw() {
      Some(raw) => {
        let ptr = raw.as_ptr() as *mut u8;
        let dst = unsafe { std::slice::from_raw_parts_mut(ptr, size) };
        let n = data.len().min(size);
        dst[..n].copy_from_slice(&data[..n]);
        Ok::<i32, rquickjs::Error>(data.len() as i32)
      }
      None => Err(Exception::throw_message(
        &ctx_err,
        "fs.read: detached buffer",
      )),
    }
  })
}

/// `fs.write(rid, buffer)` -> `Promise<number>`.
pub fn write<'js>(
  ctx: Ctx<'js>,
  rid: i32,
  buffer: ArrayBuffer<'js>,
) -> JsResult<Promise<'js>> {
  let table = resource_table!(&ctx);
  let rid = ResourceId::from(rid);
  let data = array_buffer_to_vec(&ctx, buffer.into_value())?;
  trace!("fs.write rid:{:?}, len:{}", rid, data.len());
  let tracker = blocked_tracker(&ctx);
  let ctx_err = ctx.clone();
  Promise::wrap_future(&ctx, async move {
    let n = tracker
      .spawn_blocking(move || fs_write_s(table, rid, data))
      .await
      .map_err(|e| {
        Exception::throw_message(&ctx_err, &format!("fs.write task: {}", e))
      })?
      .map_err(|e| throw(&ctx_err, e))?;
    Ok::<i32, rquickjs::Error>(n as i32)
  })
}

/// `fs.readDir(path)` -> `Promise<number>`.
pub fn read_dir<'js>(ctx: Ctx<'js>, path: String) -> JsResult<Promise<'js>> {
  let table = resource_table!(&ctx);
  trace!("fs.readDir:{:?}", path);
  let tracker = blocked_tracker(&ctx);
  let ctx_err = ctx.clone();
  Promise::wrap_future(&ctx, async move {
    let rid = tracker
      .spawn_blocking(move || {
        crate::js::fs::fs_read_dir_s(table, Path::new(&path))
      })
      .await
      .map_err(|e| {
        Exception::throw_message(&ctx_err, &format!("fs.readDir task: {}", e))
      })?
      .map_err(|e| throw(&ctx_err, e))?;
    Ok::<i32, rquickjs::Error>(rid.into())
  })
}

/// `fs.readDirNext(rid)` -> `Promise<object | undefined>`.
pub fn read_dir_next<'js>(ctx: Ctx<'js>, rid: i32) -> JsResult<Promise<'js>> {
  let table = resource_table!(&ctx);
  let rid = ResourceId::from(rid);
  let tracker = blocked_tracker(&ctx);
  let ctx_err = ctx.clone();
  Promise::wrap_future(&ctx, async move {
    let next = tracker
      .spawn_blocking(move || crate::js::fs::fs_read_dir_next_s(table, rid))
      .await
      .map_err(|e| {
        Exception::throw_message(
          &ctx_err,
          &format!("fs.readDirNext task: {}", e),
        )
      })?;
    match next {
      Some(Ok(entry)) => Ok::<Option<Object<'js>>, rquickjs::Error>(Some(
        dir_entry_to_object(&ctx_err, &entry)?,
      )),
      Some(Err(e)) => Err(throw(&ctx_err, e)),
      None => Ok(None),
    }
  })
}

/// `fs.readFile(path)` -> `Promise<ArrayBuffer>`.
pub fn read_file<'js>(ctx: Ctx<'js>, path: String) -> JsResult<Promise<'js>> {
  let ctx_err = ctx.clone();
  Promise::wrap_future(&ctx, async move {
    let data = fs_read_file_a(Path::new(&path))
      .await
      .map_err(|e| throw(&ctx_err, e))?;
    vec_to_array_buffer(ctx_err, data)
  })
}

/// `fs.readTextFile(path)` -> `Promise<string>`.
pub fn read_text_file<'js>(
  ctx: Ctx<'js>,
  path: String,
) -> JsResult<Promise<'js>> {
  let ctx_err = ctx.clone();
  Promise::wrap_future(&ctx, async move {
    fs_read_text_file_a(Path::new(&path))
      .await
      .map_err(|e| throw(&ctx_err, e))
  })
}

/// `fs.lstat(path)` -> `Promise<object>`.
pub fn lstat<'js>(ctx: Ctx<'js>, path: String) -> JsResult<Promise<'js>> {
  let ctx_err = ctx.clone();
  Promise::wrap_future(&ctx, async move {
    let meta = fs_lstat_a(Path::new(&path))
      .await
      .map_err(|e| throw(&ctx_err, e))?;
    metadata_to_object(&ctx_err, &meta)
  })
}

/// `fs.stat(path)` -> `Promise<object>`.
pub fn stat<'js>(ctx: Ctx<'js>, path: String) -> JsResult<Promise<'js>> {
  let ctx_err = ctx.clone();
  Promise::wrap_future(&ctx, async move {
    let meta = fs_stat_a(Path::new(&path))
      .await
      .map_err(|e| throw(&ctx_err, e))?;
    metadata_to_object(&ctx_err, &meta)
  })
}

/// `fs.symlink(oldpath, newpath, options)` -> `Promise<void>`.
pub fn symlink<'js>(
  ctx: Ctx<'js>,
  oldpath: String,
  newpath: String,
  options: Option<String>,
) -> JsResult<Promise<'js>> {
  let options = match options {
    Some(o) => FsSymlinkOptions::from_str(&o).map_err(|_| {
      Exception::throw_range(&ctx, &format!("fs.symlink: unknown type {:?}", o))
    })?,
    None => FsSymlinkOptions::File,
  };
  let ctx_err = ctx.clone();
  Promise::wrap_future(&ctx, async move {
    fs_symlink_a(Path::new(&oldpath), Path::new(&newpath), options)
      .await
      .map_err(|e| throw(&ctx_err, e))
  })
}

/// `fs.link(oldpath, newpath)` -> `Promise<void>`.
pub fn link<'js>(
  ctx: Ctx<'js>,
  oldpath: String,
  newpath: String,
) -> JsResult<Promise<'js>> {
  let ctx_err = ctx.clone();
  Promise::wrap_future(&ctx, async move {
    fs_link_a(Path::new(&oldpath), Path::new(&newpath))
      .await
      .map_err(|e| throw(&ctx_err, e))
  })
}

/// `fs.mkdir(path, options)` -> `Promise<void>`.
pub fn mkdir<'js>(
  ctx: Ctx<'js>,
  path: String,
  options: Option<Object<'js>>,
) -> JsResult<Promise<'js>> {
  let opts = mkdir_options(&ctx, options)?;
  let ctx_err = ctx.clone();
  Promise::wrap_future(&ctx, async move {
    fs_mkdir_a(Path::new(&path), opts)
      .await
      .map_err(|e| throw(&ctx_err, e))
  })
}

/// Clones the tracker that outlives the event loop, or raises if the kernel is
/// missing.
fn blocked_tracker(ctx: &Ctx<'_>) -> tokio_util::task::TaskTracker {
  match get_state(ctx) {
    Some(kernel) => kernel.borrow().blocked_tracker.clone(),
    None => tokio_util::task::TaskTracker::new(),
  }
}

fn open_options<'js>(
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

fn mkdir_options<'js>(
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

fn metadata_to_object<'js>(
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

fn dir_entry_to_object<'js>(
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

/// Installs the promise-returning `fs_*` built-ins on `internal`.
pub fn install<'js>(ctx: &Ctx<'js>, internal: &Object<'js>) -> JsResult<()> {
  set_function_to(ctx, internal, "fs_open", open)?;
  set_function_to(ctx, internal, "fs_read", read)?;
  set_function_to(ctx, internal, "fs_write", write)?;
  set_function_to(ctx, internal, "fs_read_dir", read_dir)?;
  set_function_to(ctx, internal, "fs_read_dir_next", read_dir_next)?;
  set_function_to(ctx, internal, "fs_read_file", read_file)?;
  set_function_to(ctx, internal, "fs_read_text_file", read_text_file)?;
  set_function_to(ctx, internal, "fs_lstat", lstat)?;
  set_function_to(ctx, internal, "fs_stat", stat)?;
  set_function_to(ctx, internal, "fs_symlink", symlink)?;
  set_function_to(ctx, internal, "fs_link", link)?;
  set_function_to(ctx, internal, "fs_mkdir", mkdir)?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use crate::js::rqjs::binding::install_bindings;
  use crate::js::rqjs::binding::install_web_globals;
  use crate::js::rqjs::runtime::JsKernel;
  use crate::js::rqjs::runtime::JsRuntimeOptions;
  use crate::js::rqjs::runtime::set_state;
  use crate::tests::js::make_editor;
  use crate::tests::log::init as test_log_init;
  use rquickjs::AsyncContext;
  use rquickjs::AsyncRuntime;
  use rquickjs::Ctx;
  use rquickjs::Promise;

  /// Runs `f` inside an async context that already has the kernel, the
  /// bindings and the web globals installed.
  ///
  /// The closure has to be async: the promise-returning built-ins only make
  /// progress while the context is being driven, and `with` does not drive.
  async fn with_kernel<F, R>(f: F) -> R
  where
    F: for<'js> AsyncFnOnce(Ctx<'js>) -> R,
    R: 'static,
  {
    let editor = make_editor();
    let (master_tx, _master_rx) = tokio::sync::mpsc::unbounded_channel();
    let kernel = JsKernel::to_rc(JsKernel {
      options: JsRuntimeOptions::default(),
      master_tx,
      editor,
      blocked_tracker: tokio_util::task::TaskTracker::new(),
    });
    let rt = AsyncRuntime::new().unwrap();
    let ctx = AsyncContext::full(&rt).await.unwrap();
    ctx
      .async_with(async move |ctx| {
        set_state(&ctx, kernel).unwrap();
        install_bindings(&ctx).unwrap();
        install_web_globals(&ctx).unwrap();
        f(ctx).await
      })
      .await
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn open_write_read_close_roundtrip() {
    test_log_init();
    let file = assert_fs::NamedTempFile::new("rqjs-fs-async.txt").unwrap();
    let path = file.path().to_string_lossy().to_string();
    let written = with_kernel(async |ctx| {
      let src = format!(
        "__InternalRsvimGlobalObject.fs_open({path:?}, {{create:true,read:true,write:true,truncate:true}})"
      );
      let promise: Promise = ctx.eval(src).unwrap();
      let rid: i32 = promise.into_future().await.unwrap();

      let src = format!(
        "const buf = new Uint8Array([72, 101, 108, 108, 111]).buffer;
        __InternalRsvimGlobalObject.fs_write({rid}, buf)"
      );
      let promise: Promise = ctx.eval(src).unwrap();
      let n: i32 = promise.into_future().await.unwrap();

      let src =
        format!("__InternalRsvimGlobalObject.fs_close({rid})");
      ctx.eval::<(), _>(src).unwrap();
      n
    })
    .await;
    assert_eq!(written, 5);
    assert_eq!(std::fs::read_to_string(file.path()).unwrap(), "Hello");
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn read_fills_the_caller_buffer() {
    test_log_init();
    let file = assert_fs::NamedTempFile::new("rqjs-fs-async-read.txt").unwrap();
    std::fs::write(file.path(), "abcdef").unwrap();
    let path = file.path().to_string_lossy().to_string();
    let got = with_kernel(async |ctx| {
      let src =
        format!("__InternalRsvimGlobalObject.fs_open({path:?}, {{read:true}})");
      let promise: Promise = ctx.eval(src).unwrap();
      let rid: i32 = promise.into_future().await.unwrap();

      let src = format!(
        "const buf = new Uint8Array(4);
        __InternalRsvimGlobalObject.fs_read({rid}, buf.buffer).then((n) => {{
          globalThis.__read = n;
          globalThis.__text = String.fromCharCode(...buf);
        }})"
      );
      let promise: Promise = ctx.eval(src).unwrap();
      promise.into_future::<()>().await.unwrap();
      ctx
        .eval::<(), _>(format!("__InternalRsvimGlobalObject.fs_close({rid})"))
        .unwrap();
      let n: i32 = ctx.eval("globalThis.__read").unwrap();
      let text: String = ctx.eval("globalThis.__text").unwrap();
      (n, text)
    })
    .await;
    assert_eq!(got.0, 4);
    assert_eq!(got.1, "abcd");
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn read_text_file_resolves_to_the_contents() {
    test_log_init();
    let file = assert_fs::NamedTempFile::new("rqjs-fs-async-text.txt").unwrap();
    std::fs::write(file.path(), "hello world").unwrap();
    let path = file.path().to_string_lossy().to_string();
    let text = with_kernel(async |ctx| {
      let src =
        format!("__InternalRsvimGlobalObject.fs_read_text_file({path:?})");
      let promise: Promise = ctx.eval(src).unwrap();
      promise.into_future::<String>().await.unwrap()
    })
    .await;
    assert_eq!(text, "hello world");
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn stat_resolves_to_a_metadata_object() {
    test_log_init();
    let file = assert_fs::NamedTempFile::new("rqjs-fs-async-stat.txt").unwrap();
    std::fs::write(file.path(), "12345").unwrap();
    let path = file.path().to_string_lossy().to_string();
    let (is_file, len) = with_kernel(async |ctx| {
      let src = format!("__InternalRsvimGlobalObject.fs_stat({path:?})");
      let promise: Promise = ctx.eval(src).unwrap();
      promise.into_future::<rquickjs::Object>().await.unwrap();
      let promise: Promise = ctx.eval(format!(
        "__InternalRsvimGlobalObject.fs_stat({path:?}).then((m) => {{ globalThis.__meta = m; }})"
      )).unwrap();
      promise.into_future::<()>().await.unwrap();
      let is_file: bool = ctx.eval("globalThis.__meta.isFile").unwrap();
      let len: i64 = ctx.eval("globalThis.__meta.len").unwrap();
      (is_file, len)
    })
    .await;
    assert!(is_file);
    assert_eq!(len, 5);
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn missing_file_rejects_the_promise() {
    test_log_init();
    let got = with_kernel(async |ctx| {
      let promise: Promise = ctx
        .eval(
          "__InternalRsvimGlobalObject.fs_read_text_file('/nonexistent/rsvim')",
        )
        .unwrap();
      promise.into_future::<String>().await.is_err()
    })
    .await;
    assert!(got, "a missing file must reject, not resolve");
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn mkdir_and_read_dir_roundtrip() {
    test_log_init();
    let dir = assert_fs::TempDir::new().unwrap();
    let target = dir.path().join("nested");
    let target = target.to_string_lossy().to_string();
    let names = with_kernel(async |ctx| {
      let src = format!(
        "__InternalRsvimGlobalObject.fs_mkdir({target:?}, {{recursive:true}})"
      );
      let promise: Promise = ctx.eval(src).unwrap();
      promise.into_future::<()>().await.unwrap();

      std::fs::write(format!("{}/one.txt", target), "").unwrap();

      let src = format!("__InternalRsvimGlobalObject.fs_read_dir({target:?})");
      let promise: Promise = ctx.eval(src).unwrap();
      let rid: i32 = promise.into_future().await.unwrap();

      let src = format!(
        "globalThis.__names = [];
        (async () => {{
          for (;;) {{
            const e = await __InternalRsvimGlobalObject.fs_read_dir_next({rid});
            if (e === undefined || e === null) break;
            globalThis.__names.push(e.name);
          }}
        }})()"
      );
      let promise: Promise = ctx.eval(src).unwrap();
      promise.into_future::<()>().await.unwrap();
      ctx
        .eval::<(), _>(format!(
          "__InternalRsvimGlobalObject.fs_read_dir_close({rid})"
        ))
        .unwrap();
      ctx.eval::<Vec<String>, _>("globalThis.__names").unwrap()
    })
    .await;
    assert_eq!(names, vec!["one.txt".to_string()]);
  }
}
