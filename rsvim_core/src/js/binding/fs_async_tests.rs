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
  let file = assert_fs::NamedTempFile::new("rqjs-fs-async-shrink.txt").unwrap();
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
  let file = assert_fs::NamedTempFile::new("rqjs-fs-async-bytes.txt").unwrap();
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
