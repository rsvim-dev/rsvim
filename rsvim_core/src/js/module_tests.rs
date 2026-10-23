use super::module::*;
use crate::js::JsRuntime;
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

#[test]
#[cfg_attr(miri, ignore)]
fn fetch_tree3() {
  test_log_init();

  let src1: &str = r#"
  export const PI = 3.14159;
  "#;

  let src2: &str = r#"
  import { PI } from "./pi.js";

  function addPI(a, b) {
    return PI+a+b;
  }

  export { addPI };
  "#;

  let src3: &str = r#"
  import * as pi from "./util/pi.js";
  import addUtil from "./util/add.ts";

  const result = addUtil.addPI(1.0, 2.5);
  "#;

  let fetch1 = "./util/pi.js";
  let fetch2 = "./util/add.ts";
  let fetch3 = "./fetch3.js";

  // Prepare $RSVIM_CONFIG
  // - rsvim.js
  // - fetch3.js
  // - util/pi.js
  // - util/add.ts
  let tp = make_configs(vec![
    (Path::new("rsvim.js"), ""),
    (Path::new(fetch1), src1),
    (Path::new(fetch2), src2),
    (Path::new(fetch3), src3),
  ]);

  let mut jsrt = make_js_runtime();
  let context = jsrt.context();
  v8::scope_with_context!(scope, &mut jsrt.isolate, context);
  let actual1 = fetch_module_tree(
    scope,
    &tp.xdg_config_home.join("rsvim/fetch3.js").to_string_lossy(),
    None,
  );
  assert!(actual1.is_some());
  let actual1 = actual1.unwrap();
  info!(
    "fetch_tree3 actual1:{:?}, script_id:{:?}",
    actual1,
    actual1.script_id()
  );
  assert!(actual1.script_id().is_some());
  assert!(actual1.script_id().unwrap() > 0);

  let state_rc = JsRuntime::state(scope);
  let state = state_rc.borrow();

  let path3 = resolve_import(
    &tp.xdg_config_home.join("rsvim").to_string_lossy(),
    fetch3,
    None,
  );
  assert!(path3.is_ok());
  let path3 = path3.unwrap();
  // NOTE: On macOS, the `tp.xdg_config_home.join("rsvim/fetch3.js")` is `/var/folders/xxx`, while
  // oxc_resolver resolved path is `/private/var/folders/xxx`.
  info!(
    "fetch_tree3 path3:{:?}, module_map:{:?}",
    path3, state.module_map
  );
  assert!(state.module_map.get_by_suffix(&path3).is_some());

  let path1 = resolve_import(
    &tp.xdg_config_home.join("rsvim").to_string_lossy(),
    fetch1,
    None,
  );
  info!(
    "fetch_tree3 path1:{:?}, module_map:{:?}",
    path1, state.module_map
  );
  assert!(path1.is_ok());
  let path1 = path1.unwrap();
  assert!(state.module_map.get(&path1).is_some());

  let fetch2_without_ext = "./util/add";
  info!(
    "fetch_tree3 fetch2:{:?},fetch2.file_stem:{:?},fetch2.without_extension:{:?}",
    fetch2,
    Path::new(fetch2).file_stem(),
    fetch2_without_ext
  );
  let path2 = resolve_import(
    &tp.xdg_config_home.join("rsvim").to_string_lossy(),
    fetch2_without_ext,
    None,
  );
  info!("fetch_tree3 path2:{:?}, fetch2:{:?}", path2, fetch2);
  assert!(path2.is_ok());
}

#[test]
#[cfg_attr(miri, ignore)]
fn fetch_tree4() {
  test_log_init();

  let src1: &str = r#"
  export const PI = 3.14159;
  "#;

  let src2: &str = r#"
  import { PI } from "./index.js";

  function addPI(a, b) {
    return PI+a+b;
  }

  export { addPI };
  "#;

  let src3: &str = r#"
  import * as pi from "./util";
  import addUtil from "./util/add.ts";

  const result = addUtil.addPI(1.0, 2.5);
  "#;

  let fetch1 = "./util/index.js";
  let fetch2 = "./util/add.ts";
  let fetch3 = "./index.js";

  // Prepare $RSVIM_CONFIG
  // - rsvim.js
  // - fetch3.js
  // - util/pi.js
  // - util/add.ts
  let tp = make_configs(vec![
    (Path::new("rsvim.js"), ""),
    (Path::new(fetch1), src1),
    (Path::new(fetch2), src2),
    (Path::new(fetch3), src3),
  ]);

  let mut jsrt = make_js_runtime();
  let context = jsrt.context();
  v8::scope_with_context!(scope, &mut jsrt.isolate, context);
  let actual1 = fetch_module_tree(
    scope,
    &tp.xdg_config_home.join("rsvim/index.js").to_string_lossy(),
    None,
  );
  assert!(actual1.is_some());
  let actual1 = actual1.unwrap();
  info!(
    "fetch_tree4 actual1:{:?}, script_id:{:?}",
    actual1,
    actual1.script_id()
  );
  assert!(actual1.script_id().is_some());
  assert!(actual1.script_id().unwrap() > 0);

  let state = JsRuntime::state(scope);
  let state = state.borrow();

  let path3 = resolve_import(
    &tp.xdg_config_home.join("rsvim").to_string_lossy(),
    fetch3,
    None,
  );
  info!("fetch_tree4 path3:{:?}, fetch3:{:?}", path3, fetch3);
  assert!(path3.is_ok());
  let path3 = path3.unwrap();
  info!(
    "fetch_tree4 path3:{:?}, module_map:{:?}",
    path3, state.module_map
  );
  assert!(state.module_map.get_by_suffix(&path3).is_some());

  let path1 = resolve_import(
    &tp.xdg_config_home.join("rsvim").to_string_lossy(),
    fetch1,
    None,
  );
  info!("fetch_tree4 path1:{:?}, fetch1:{:?}", path1, fetch1);
  assert!(path1.is_ok());
  let path1 = path1.unwrap();
  info!(
    "fetch_tree4 path1:{:?}, module_map:{:?}",
    path1, state.module_map
  );
  assert!(state.module_map.get(&path1).is_some());

  let fetch2_without_ext = "./util/add";
  info!(
    "fetch_tree4 fetch2:{:?},fetch2.without_extension:{:?}",
    fetch2, fetch2_without_ext
  );
  let path2 = resolve_import(
    &tp.xdg_config_home.join("rsvim").to_string_lossy(),
    fetch2_without_ext,
    None,
  );
  info!("fetch_tree4 fetch2_path:{:?}, fetch2:{:?}", path2, fetch2);
  assert!(path2.is_ok());
}
