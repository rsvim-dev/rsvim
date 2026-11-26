//! APIs for the `fs` module — the promise-returning half.
//!
//! Every function here answers with a promise instead of a value, which is
//! what `fs.min.js` awaits. The work itself is in [`crate::js::fs`]: either a
//! `tokio::fs` coroutine (`fs_*_a`) or a blocking call (`fs_read_s`,
//! `fs_write_s`) that has to run off the event loop thread.
//!
//! Each binding hands its work to [`spawn_op`]: the future runs as a plain tokio
//! task and cannot capture a javascript value, and a `reply` closure builds the
//! javascript result on the main thread once it is done.
//!
//! # Which operations the editor waits for on exit
//!
//! The split is by what the operation does to the outside world, not by how it
//! is implemented. A write is not abortable: by the time `await file.write(buf)`
//! returns to javascript the bytes are already promised to the user, and exiting
//! while that write sits in a queue would truncate the file silently. So
//! everything that *changes* something (`open`, which may create or truncate,
//! `write`, `mkdir`, `symlink`, `link`) is [`OpKind::Blocked`] and
//! `Rsvim::close_trackers` waits for it. Everything that only *looks*
//! (`read`, `readDir`, `readFile`, `stat`, ...) is [`OpKind::Detached`]: nobody
//! would see its result once the loop is gone, so exit does not wait.

use crate::js::binding::fs::dir_entry_to_object;
use crate::js::binding::fs::metadata_to_object;
use crate::js::binding::fs::mkdir_options;
use crate::js::binding::fs::open_options;
use crate::js::binding::fs::resource_table;
use crate::js::binding::fs::symlink_options;
use crate::js::binding::set_function_to;
use crate::js::converter::array_buffer_to_vec;
use crate::js::converter::copy_into_array_buffer;
use crate::js::converter::vec_to_array_buffer;
use crate::js::fs::FsDirEntry;
use crate::js::fs::FsMetadata;
use crate::js::fs::fs_link_a;
use crate::js::fs::fs_lstat_a;
use crate::js::fs::fs_mkdir_a;
use crate::js::fs::fs_open_a;
use crate::js::fs::fs_read_dir_next_s;
use crate::js::fs::fs_read_dir_s;
use crate::js::fs::fs_read_file_a;
use crate::js::fs::fs_read_s;
use crate::js::fs::fs_read_text_file_a;
use crate::js::fs::fs_stat_a;
use crate::js::fs::fs_symlink_a;
use crate::js::fs::fs_write_s;
use crate::js::ops::OpKind;
use crate::js::ops::blocking;
use crate::js::ops::spawn_op;
use crate::js::ops::spawn_simple_op;
use crate::js::resource::ResourceId;
use crate::prelude::*;
use rquickjs::ArrayBuffer;
use rquickjs::Ctx;
use rquickjs::Exception;
use rquickjs::FromJs;
use rquickjs::IntoJs;
use rquickjs::Object;
use rquickjs::Promise;
use rquickjs::Result as JsResult;
use rquickjs::Value;

/// `fs.open(path, options)` -> `Promise<number>`.
///
/// Resolves to the resource id `fs.min.js` wraps in its `File` class.
pub fn open<'js>(
  ctx: Ctx<'js>,
  path: String,
  options: Option<Object<'js>>,
) -> JsResult<Promise<'js>> {
  let table = resource_table(&ctx)?;
  let opts = open_options(&ctx, options)?;
  trace!("fs.open:{:?}", path);
  spawn_simple_op(&ctx, "fs.open", OpKind::Blocked, async move {
    fs_open_a(table, Path::new(&path), opts)
      .await
      .map(i32::from)
  })
}

/// `fs.read(rid, buffer)` -> `Promise<number>`.
///
/// Reads at most `buffer.byteLength` bytes into the caller's array buffer and
/// resolves to the count.
///
/// The buffer is parked in the operation as its `aux` value and written to only
/// after the read, on the main thread, against the length it has *then*: a
/// buffer that was resized while the read was in flight cannot be overrun.
pub fn read<'js>(
  ctx: Ctx<'js>,
  rid: i32,
  buffer: ArrayBuffer<'js>,
) -> JsResult<Promise<'js>> {
  let table = resource_table(&ctx)?;
  let rid = ResourceId::from(rid);
  let size = buffer.len();
  trace!("fs.read rid:{:?}, size:{}", rid, size);
  spawn_op(
    &ctx,
    "fs.read",
    OpKind::Detached,
    Some(buffer.into_value()),
    blocking("fs.read", move || fs_read_s(table, rid, size)),
    |ctx, aux, data: Vec<u8>| {
      let buffer = aux.ok_or_else(|| {
        Exception::throw_message(ctx, "fs.read: buffer is gone")
      })?;
      let buffer = ArrayBuffer::from_js(ctx, buffer)?;
      let n = copy_into_array_buffer(&buffer, &data).ok_or_else(|| {
        Exception::throw_message(ctx, "fs.read: detached buffer")
      })?;
      (n as i32).into_js(ctx)
    },
  )
}

/// `fs.write(rid, buffer)` -> `Promise<number>`.
pub fn write<'js>(
  ctx: Ctx<'js>,
  rid: i32,
  buffer: ArrayBuffer<'js>,
) -> JsResult<Promise<'js>> {
  let table = resource_table(&ctx)?;
  let rid = ResourceId::from(rid);
  let data = array_buffer_to_vec(&ctx, buffer.into_value())?;
  trace!("fs.write rid:{:?}, len:{}", rid, data.len());
  spawn_simple_op(
    &ctx,
    "fs.write",
    OpKind::Blocked,
    blocking("fs.write", move || {
      fs_write_s(table, rid, data).map(|n| n as i32)
    }),
  )
}

/// `fs.readDir(path)` -> `Promise<number>`.
pub fn read_dir<'js>(ctx: Ctx<'js>, path: String) -> JsResult<Promise<'js>> {
  let table = resource_table(&ctx)?;
  trace!("fs.readDir:{:?}", path);
  spawn_simple_op(
    &ctx,
    "fs.readDir",
    OpKind::Detached,
    blocking("fs.readDir", move || {
      fs_read_dir_s(table, Path::new(&path)).map(i32::from)
    }),
  )
}

/// `fs.readDirNext(rid)` -> `Promise<object | undefined>`.
pub fn read_dir_next<'js>(ctx: Ctx<'js>, rid: i32) -> JsResult<Promise<'js>> {
  let table = resource_table(&ctx)?;
  let rid = ResourceId::from(rid);
  spawn_op(
    &ctx,
    "fs.readDirNext",
    OpKind::Detached,
    None,
    blocking("fs.readDirNext", move || {
      fs_read_dir_next_s(table, rid).transpose()
    }),
    |ctx, _aux, entry: Option<FsDirEntry>| match entry {
      Some(entry) => Ok(dir_entry_to_object(ctx, &entry)?.into_value()),
      None => Ok(Value::new_undefined(ctx.clone())),
    },
  )
}

/// `fs.readFile(path)` -> `Promise<ArrayBuffer>`.
pub fn read_file<'js>(ctx: Ctx<'js>, path: String) -> JsResult<Promise<'js>> {
  spawn_op(
    &ctx,
    "fs.readFile",
    OpKind::Detached,
    None,
    async move { fs_read_file_a(Path::new(&path)).await },
    |ctx, _aux, data: Vec<u8>| {
      Ok(vec_to_array_buffer(ctx.clone(), data)?.into_value())
    },
  )
}

/// `fs.readTextFile(path)` -> `Promise<string>`.
pub fn read_text_file<'js>(
  ctx: Ctx<'js>,
  path: String,
) -> JsResult<Promise<'js>> {
  spawn_simple_op(&ctx, "fs.readTextFile", OpKind::Detached, async move {
    fs_read_text_file_a(Path::new(&path)).await
  })
}

/// `fs.lstat(path)` -> `Promise<object>`.
pub fn lstat<'js>(ctx: Ctx<'js>, path: String) -> JsResult<Promise<'js>> {
  spawn_op(
    &ctx,
    "fs.lstat",
    OpKind::Detached,
    None,
    async move { fs_lstat_a(Path::new(&path)).await },
    |ctx, _aux, meta: FsMetadata| {
      Ok(metadata_to_object(ctx, &meta)?.into_value())
    },
  )
}

/// `fs.stat(path)` -> `Promise<object>`.
pub fn stat<'js>(ctx: Ctx<'js>, path: String) -> JsResult<Promise<'js>> {
  spawn_op(
    &ctx,
    "fs.stat",
    OpKind::Detached,
    None,
    async move { fs_stat_a(Path::new(&path)).await },
    |ctx, _aux, meta: FsMetadata| {
      Ok(metadata_to_object(ctx, &meta)?.into_value())
    },
  )
}

/// `fs.symlink(oldpath, newpath, options)` -> `Promise<void>`.
pub fn symlink<'js>(
  ctx: Ctx<'js>,
  oldpath: String,
  newpath: String,
  options: Option<String>,
) -> JsResult<Promise<'js>> {
  let options = symlink_options(&ctx, "fs.symlink", options)?;
  spawn_simple_op(&ctx, "fs.symlink", OpKind::Blocked, async move {
    fs_symlink_a(Path::new(&oldpath), Path::new(&newpath), options).await
  })
}

/// `fs.link(oldpath, newpath)` -> `Promise<void>`.
pub fn link<'js>(
  ctx: Ctx<'js>,
  oldpath: String,
  newpath: String,
) -> JsResult<Promise<'js>> {
  spawn_simple_op(&ctx, "fs.link", OpKind::Blocked, async move {
    fs_link_a(Path::new(&oldpath), Path::new(&newpath)).await
  })
}

/// `fs.mkdir(path, options)` -> `Promise<void>`.
pub fn mkdir<'js>(
  ctx: Ctx<'js>,
  path: String,
  options: Option<Object<'js>>,
) -> JsResult<Promise<'js>> {
  let opts = mkdir_options(&ctx, options)?;
  spawn_simple_op(&ctx, "fs.mkdir", OpKind::Blocked, async move {
    fs_mkdir_a(Path::new(&path), opts).await
  })
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
  use crate::tests::js::make_js_host;
  use crate::tests::log::init as test_log_init;

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn open_write_read_close_roundtrip() {
    test_log_init();
    let file = assert_fs::NamedTempFile::new("rqjs-fs-async.txt").unwrap();
    let path = file.path().to_string_lossy().to_string();
    let mut js = make_js_host();

    js.settle(&format!(
      "__InternalRsvimGlobalObject.fs_open({path:?}, {{create:true,read:true,write:true,truncate:true}})"
    ))
    .await
    .unwrap();
    let rid: i32 = js.eval("globalThis.__value");

    js.settle(&format!(
      "__InternalRsvimGlobalObject.fs_write({rid}, new Uint8Array([72, 101, 108, 108, 111]).buffer)"
    ))
    .await
    .unwrap();
    let written: i32 = js.eval("globalThis.__value");
    js.run(&format!("__InternalRsvimGlobalObject.fs_close({rid})"));

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
    let mut js = make_js_host();

    js.settle(&format!(
      "__InternalRsvimGlobalObject.fs_open({path:?}, {{read:true}})"
    ))
    .await
    .unwrap();
    let rid: i32 = js.eval("globalThis.__value");

    js.run("globalThis.buf = new Uint8Array(4);");
    js.settle(&format!(
      "__InternalRsvimGlobalObject.fs_read({rid}, buf.buffer)"
    ))
    .await
    .unwrap();
    let n: i32 = js.eval("globalThis.__value");
    let text: String = js.eval("String.fromCharCode(...buf)");
    js.run(&format!("__InternalRsvimGlobalObject.fs_close({rid})"));

    assert_eq!(n, 4);
    assert_eq!(text, "abcd");
  }

  /// The copy happens after the read, against the buffer's length at that
  /// moment, so a buffer that shrank while the read was in flight is not
  /// overrun. (`ArrayBuffer.prototype.resize` is what shrinks it.)
  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn read_into_a_buffer_that_shrank_does_not_overrun() {
    test_log_init();
    let file =
      assert_fs::NamedTempFile::new("rqjs-fs-async-shrink.txt").unwrap();
    std::fs::write(file.path(), "abcdef").unwrap();
    let path = file.path().to_string_lossy().to_string();
    let mut js = make_js_host();

    js.settle(&format!(
      "__InternalRsvimGlobalObject.fs_open({path:?}, {{read:true}})"
    ))
    .await
    .unwrap();
    let rid: i32 = js.eval("globalThis.__value");

    // Start the read with an 8 byte buffer and shrink it before the completion
    // is delivered.
    js.run(&format!(
      "globalThis.rab = new ArrayBuffer(8, {{ maxByteLength: 8 }});
       globalThis.__state = 0;
       __InternalRsvimGlobalObject.fs_read({rid}, rab).then(
         (n) => {{ globalThis.__value = n; globalThis.__state = 1; }},
         (e) => {{ globalThis.__value = String(e); globalThis.__state = 2; }});
       rab.resize(2);"
    ));
    let done = js
      .run_until(std::time::Duration::from_secs(5), |js| {
        js.eval::<i32>("globalThis.__state") != 0
      })
      .await;
    assert!(done);
    let state: i32 = js.eval("globalThis.__state");
    assert_eq!(state, 1, "the read must still resolve");
    let n: i32 = js.eval("globalThis.__value");
    assert_eq!(n, 2, "only the 2 bytes that still fit are copied");
    js.run(&format!("__InternalRsvimGlobalObject.fs_close({rid})"));
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn read_text_file_resolves_to_the_contents() {
    test_log_init();
    let file = assert_fs::NamedTempFile::new("rqjs-fs-async-text.txt").unwrap();
    std::fs::write(file.path(), "hello world").unwrap();
    let path = file.path().to_string_lossy().to_string();
    let mut js = make_js_host();

    js.settle(&format!(
      "__InternalRsvimGlobalObject.fs_read_text_file({path:?})"
    ))
    .await
    .unwrap();
    let text: String = js.eval("globalThis.__value");
    assert_eq!(text, "hello world");
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn read_file_resolves_to_an_array_buffer() {
    test_log_init();
    let file =
      assert_fs::NamedTempFile::new("rqjs-fs-async-bytes.txt").unwrap();
    std::fs::write(file.path(), "xyz").unwrap();
    let path = file.path().to_string_lossy().to_string();
    let mut js = make_js_host();

    js.settle(&format!(
      "__InternalRsvimGlobalObject.fs_read_file({path:?})"
    ))
    .await
    .unwrap();
    let len: i32 = js.eval("globalThis.__value.byteLength");
    assert_eq!(len, 3);
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn stat_resolves_to_a_metadata_object() {
    test_log_init();
    let file = assert_fs::NamedTempFile::new("rqjs-fs-async-stat.txt").unwrap();
    std::fs::write(file.path(), "12345").unwrap();
    let path = file.path().to_string_lossy().to_string();
    let mut js = make_js_host();

    js.settle(&format!("__InternalRsvimGlobalObject.fs_stat({path:?})"))
      .await
      .unwrap();
    let is_file: bool = js.eval("globalThis.__value.isFile");
    let len: i64 = js.eval("globalThis.__value.len");
    assert!(is_file);
    assert_eq!(len, 5);
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn missing_file_rejects_the_promise() {
    test_log_init();
    let mut js = make_js_host();
    let got = js
      .settle(
        "__InternalRsvimGlobalObject.fs_read_text_file('/nonexistent/rsvim')",
      )
      .await;
    assert!(got.is_err(), "a missing file must reject, not resolve");
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn mkdir_and_read_dir_roundtrip() {
    test_log_init();
    let dir = assert_fs::TempDir::new().unwrap();
    let target = dir.path().join("nested");
    let target = target.to_string_lossy().to_string();
    let mut js = make_js_host();

    js.settle(&format!(
      "__InternalRsvimGlobalObject.fs_mkdir({target:?}, {{recursive:true}})"
    ))
    .await
    .unwrap();
    std::fs::write(format!("{}/one.txt", target), "").unwrap();

    js.settle(&format!(
      "__InternalRsvimGlobalObject.fs_read_dir({target:?})"
    ))
    .await
    .unwrap();
    let rid: i32 = js.eval("globalThis.__value");

    js.settle(&format!(
      "(async () => {{
        globalThis.__names = [];
        for (;;) {{
          const e = await __InternalRsvimGlobalObject.fs_read_dir_next({rid});
          if (e === undefined || e === null) break;
          globalThis.__names.push(e.name);
        }}
      }})()"
    ))
    .await
    .unwrap();
    js.run(&format!(
      "__InternalRsvimGlobalObject.fs_read_dir_close({rid})"
    ));
    let names: Vec<String> = js.eval("globalThis.__names");
    assert_eq!(names, vec!["one.txt".to_string()]);
  }

  /// Changes to the outside world are waited for on exit, lookups are not.
  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn only_mutations_are_tracked_as_blocked() {
    test_log_init();
    let dir = assert_fs::TempDir::new().unwrap();
    let target = dir.path().join("made");
    let target = target.to_string_lossy().to_string();
    let mut js = make_js_host();

    js.run(&format!(
      "__InternalRsvimGlobalObject.fs_mkdir({target:?}, {{recursive:true}});"
    ));
    assert_eq!(js.host.ops().pending_len(), 1);
    js.run_until(std::time::Duration::from_secs(5), |js| {
      js.host.ops().pending_len() == 0
    })
    .await;
    assert!(std::path::Path::new(&target).is_dir());
  }
}
