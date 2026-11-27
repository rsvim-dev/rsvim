use crate::cli::CliOptions;
use crate::prelude::*;
use crate::tests::evloop::*;
use crate::tests::log::init as test_log_init;
use std::time::Duration;

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_load1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;

  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(10000))];

  // The runtime path is "./rsvim_core", so we need to prepend ".." for it.
  let src: &str = r#"
  import cmd from "rsvim:cmd";
  import opt from "rsvim:opt";
  import process from "rsvim:process";
  import syn from "rsvim:syn";
  opt.syntaxParserLibPath = ".test-tree-sitter-parsers";
  try {
    const parsers = await syn.loadParser({ grammarPath: "../tests_and_benchmarks/tree-sitter-c" });
    cmd.echo(parsers);
  } catch (e) {
    cmd.echo(e);
  }
  process.exit(0);
"#;

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  event_loop.initialize()?;
  event_loop
    .run_with_mock_events(MockEventReader::new(mocked_events))
    .await?;
  event_loop.shutdown()?;

  // After running
  {
    let mut contents = lock!(event_loop.editor.cmdline_text);
    let n = contents.message_history().len();
    assert_eq!(n, 1);
    let actual = contents.message_history_mut().pop();
    info!("actual:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();
    assert_eq!(actual, "c");
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_load_sync1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;

  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(10000))];

  // The runtime path is "./rsvim_core", so we need to prepend ".." for it.
  let src: &str = r#"
  import cmd from "rsvim:cmd";
  import opt from "rsvim:opt";
  import process from "rsvim:process";
  import syn from "rsvim:syn";
  opt.syntaxParserLibPath = ".test-tree-sitter-parsers";
  try {
    const parsers = syn.loadParserSync({ grammarPath: "../tests_and_benchmarks/tree-sitter-python" });
    cmd.echo(parsers);
  } catch (e) {
    cmd.echo(e);
  }
  process.exit(0);
"#;

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  event_loop.initialize()?;
  event_loop
    .run_with_mock_events(MockEventReader::new(mocked_events))
    .await?;
  event_loop.shutdown()?;

  // After running
  {
    let mut contents = lock!(event_loop.editor.cmdline_text);
    let n = contents.message_history().len();
    assert_eq!(n, 1);
    let actual = contents.message_history_mut().pop();
    info!("actual:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();
    assert_eq!(actual, "python");
  }

  Ok(())
}

#[cfg(not(any(
  all(target_os = "macos", target_arch = "x86_64"),
  all(target_os = "windows", target_arch = "aarch64")
)))]
#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_list1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;

  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(10000))];

  // The runtime path is "./rsvim_core", so we need to prepend ".." for it.
  let src: &str = r#"
  import cmd from "rsvim:cmd";
  import opt from "rsvim:opt";
  import process from "rsvim:process";
  import syn from "rsvim:syn";
  opt.syntaxParserLibPath = ".test-tree-sitter-parsers";
  try {
    const parsers = syn.loadParserSync({ grammarPath: "../tests_and_benchmarks/tree-sitter-python" });
    cmd.echo(parsers);
    const allParsers = syn.listParsers();
    cmd.echo(allParsers);
  } catch (e) {
    cmd.echo(e);
  }
  process.exit(0);
"#;

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  event_loop.initialize()?;
  event_loop
    .run_with_mock_events(MockEventReader::new(mocked_events))
    .await?;
  event_loop.shutdown()?;

  // After running
  {
    let mut contents = lock!(event_loop.editor.cmdline_text);
    let n = contents.message_history().len();
    assert_eq!(n, 2);
    let actual1 = contents.message_history_mut().pop();
    info!("actual1:{:?}", actual1);
    assert!(actual1.is_some());
    let actual1 = actual1.unwrap();
    assert_eq!(actual1, "python");

    let actual2 = contents.message_history_mut().pop();
    info!("actual2:{:?}", actual2);
    assert!(actual2.is_some());
    let actual2 = actual2.unwrap();
    for p in ["toml", "html", "python", "rust", "c", "markdown"].iter() {
      assert!(actual2.contains(p));
    }
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_get_metadata1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;

  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(10000))];

  // The runtime path is "./rsvim_core", so we need to prepend ".." for it.
  let src: &str = r#"
  import cmd from "rsvim:cmd";
  import opt from "rsvim:opt";
  import process from "rsvim:process";
  import syn from "rsvim:syn";
  opt.syntaxParserLibPath = ".test-tree-sitter-parsers";
  try {
    const meta = syn.getParserMetadata("c");
    cmd.echo(meta.name);
    cmd.echo(meta.camelcase);
    cmd.echo(meta.fileTypes);
  } catch (e) {
    cmd.echo(e);
  }
  process.exit(0);
"#;

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  event_loop.initialize()?;
  event_loop
    .run_with_mock_events(MockEventReader::new(mocked_events))
    .await?;
  event_loop.shutdown()?;

  // After running
  {
    let mut contents = lock!(event_loop.editor.cmdline_text);
    let n = contents.message_history().len();
    assert_eq!(n, 3);

    let actual1 = contents.message_history_mut().pop();
    info!("actual1:{:?}", actual1);
    assert!(actual1.is_some());
    let actual1 = actual1.unwrap();
    assert_eq!(actual1, "c");

    let actual2 = contents.message_history_mut().pop();
    info!("actual2:{:?}", actual2);
    assert!(actual2.is_some());
    let actual2 = actual2.unwrap();
    assert_eq!(actual2, "C");

    let actual3 = contents.message_history_mut().pop();
    info!("actual3:{:?}", actual3);
    assert!(actual3.is_some());
    let actual3 = actual3.unwrap();
    assert!(actual3.contains("c"));
    assert!(actual3.contains("h"));
  }

  Ok(())
}

use crate::js::runtime;
use crate::tests::js::make_js_host;

/// Runs `f` inside a fully installed host.
fn with_kernel<R>(
  f: impl FnOnce(rquickjs::Ctx<'_>, runtime::JsKernelRc) -> R,
) -> R {
  let mut js = make_js_host();
  let kernel = js.host.kernel().clone();
  js.enter(move |ctx| f(ctx, kernel))
}

#[test]
#[cfg_attr(miri, ignore)]
fn list_parsers_is_an_array() {
  test_log_init();
  with_kernel(|ctx, _kernel| {
    let is_array: bool = ctx
      .eval("Array.isArray(__InternalRsvimGlobalObject.syn_list_parsers())")
      .unwrap();
    assert!(is_array);
  });
}

#[test]
#[cfg_attr(miri, ignore)]
fn unknown_parser_metadata_is_undefined() {
  test_log_init();
  with_kernel(|ctx, _kernel| {
    let kind: String = ctx
      .eval(
        "typeof __InternalRsvimGlobalObject.syn_get_parser_metadata('nope')",
      )
      .unwrap();
    assert_eq!(kind, "undefined");
  });
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn load_parser_returns_a_thenable() {
  test_log_init();
  with_kernel(|ctx, _kernel| {
    // The async variant has to hand javascript something to `await`; it is a
    // promise created by `spawn_op`.
    let is_thenable: bool = ctx
        .eval(
          r#"
          (() => {
            let p;
            try {
              p = __InternalRsvimGlobalObject.syn_load_parser({ grammarPath: "nope" });
            } catch (e) {
              return false;
            }
            return typeof p === "object" && p !== null && typeof p.then === "function";
          })()
          "#,
        )
        .unwrap();
    assert!(is_thenable, "syn_load_parser must return a promise");
  });
}

#[test]
#[cfg_attr(miri, ignore)]
fn load_parser_sync_reports_an_empty_path() {
  test_log_init();
  with_kernel(|ctx, _kernel| {
    // An empty path cannot be absolutized, so the binding has to surface the
    // same `TreeSitterParserNotFound` the v8 backend throws.
    let message: String = ctx
      .eval(
        r#"
          (() => {
            try {
              __InternalRsvimGlobalObject.syn_load_parser_sync({});
              return "no error";
            } catch (e) {
              return String(e);
            }
          })()
          "#,
      )
      .unwrap();
    assert!(
      message.contains("Tree-sitter parser not found"),
      "got: {message}"
    );
  });
}

#[test]
#[cfg_attr(miri, ignore)]
fn load_parser_sync_accepts_a_missing_argument() {
  test_log_init();
  with_kernel(|ctx, _kernel| {
    // `syn.min.js` defaults the options object, so both call shapes reach the
    // binding; neither may blow up with a parameter-count error.
    let kind: String = ctx
      .eval(
        r#"
          (() => {
            try {
              __InternalRsvimGlobalObject.syn_load_parser_sync();
              return "returned";
            } catch (e) {
              return "threw";
            }
          })()
          "#,
      )
      .unwrap();
    assert_eq!(kind, "threw");
  });
}
