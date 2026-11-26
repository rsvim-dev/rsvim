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
