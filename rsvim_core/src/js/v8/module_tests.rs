use super::module::*;
use crate::prelude::*;
use crate::tests::evloop::*;
use crate::tests::js::make_js_runtime;
use crate::tests::log::init as test_log_init;
use std::io::Write;

#[test]
#[cfg_attr(miri, ignore)]
fn fetch1() {
  test_log_init();

  let tp = TempConfigDir::create();
  let tmpdir = tp.xdg_config_home.to_path_buf();

  let src1: &str = r#"
  export const PI = 3.14159;
  export function Hello(a, b) {
    return a+b;
  }
  function World(a, b) {
    return a-b;
  }
  export { World };
  "#;

  let fetch1 = tmpdir.join("fetch1.js");

  {
    let mut fp = std::fs::File::create(&fetch1).unwrap();
    fp.write_all(src1.as_bytes()).unwrap();
    fp.flush().unwrap();
  }

  let mut jsrt = make_js_runtime();
  let context = jsrt.context();
  v8::scope_with_context!(scope, &mut jsrt.isolate, context);
  let actual1 = fetch_module(scope, &fetch1.to_string_lossy(), None);
  assert!(actual1.is_some());
  let actual1 = actual1.unwrap();
  info!(
    "fetch1 actual1:{:?}, script_id:{:?}",
    actual1,
    actual1.script_id()
  );
  assert!(actual1.script_id().is_some());
  assert!(actual1.script_id().unwrap() > 0);
}

#[test]
#[cfg_attr(miri, ignore)]
fn fetch2() {
  test_log_init();

  let tp = TempConfigDir::create();
  let tmpdir = tp.xdg_config_home.to_path_buf();

  // Actually it's rust code...
  let src2: &str = r#"
  #[test]
  fn fetch2() {
    println!("hello");
  }
  "#;

  let fetch2 = tmpdir.join("fetch2.js");

  {
    let mut fp = std::fs::File::create(&fetch2).unwrap();
    fp.write_all(src2.as_bytes()).unwrap();
    fp.flush().unwrap();
  }

  let mut jsrt = make_js_runtime();
  let context = jsrt.context();
  v8::scope_with_context!(scope, &mut jsrt.isolate, context);
  let actual2 = fetch_module(scope, &fetch2.to_string_lossy(), None);
  assert!(actual2.is_none());
}
