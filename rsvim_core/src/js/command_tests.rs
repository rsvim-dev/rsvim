// use super::command::*;
use crate::cli::CliOptions;
use crate::prelude::*;
use crate::state::ops::CursorInsertPayload;
use crate::state::ops::Operation;
use crate::tests::evloop::*;
use crate::tests::log::init as test_log_init;
use assert_fs::prelude::FileTouch;
use assert_fs::prelude::FileWriteStr;
use compact_str::ToCompactString;
use std::time::Duration;

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_js_echo1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_ops = vec![
    MockOperation::Operation(Operation::GotoCmdlineExMode),
    MockOperation::Operation(Operation::CursorInsert(
      CursorInsertPayload::Text(
        r#"js import cmd from "rsvim:cmd"; cmd.echo(1);"#.to_compact_string(),
      ),
    )),
    MockOperation::Operation(Operation::ConfirmExCommandAndGotoNormalMode),
    MockOperation::SleepFor(Duration::from_millis(50)),
  ];

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), "")]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  event_loop.initialize()?;
  event_loop
    .run_with_mock_operations(MockOperationReader::new(mocked_ops))
    .await?;
  event_loop.shutdown()?;

  // After running
  {
    let mut contents = lock!(event_loop.cmdline_text);
    let n = contents.message_history().len();
    assert_eq!(n, 1);

    let actual = contents.message_history_mut().pop();
    info!("actual:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();
    assert_eq!(actual, "1");
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_js_throw1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(30))];

  let src: &str = r#"
  throw new Error(1);
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
    let mut contents = lock!(event_loop.cmdline_text);
    let n = contents.message_history().len();
    assert_eq!(n, 1);
    let actual = contents.message_history_mut().pop();
    info!("actual:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();
    assert!(actual.contains("Uncaught Error: 1"));
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_js_invalid1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(30))];

  let src: &str = r#"
  asdf
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
    let mut contents = lock!(event_loop.cmdline_text);
    let n = contents.message_history().len();
    assert_eq!(n, 1);
    let actual = contents.message_history_mut().pop();
    info!("actual:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();
    assert!(actual.contains("Uncaught ReferenceError: asdf is not defined"));
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_buf_write1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_ops = vec![
    MockOperation::Operation(Operation::GotoCmdlineExMode),
    MockOperation::Operation(Operation::CursorInsert(
      CursorInsertPayload::Text("w".to_compact_string()),
    )),
    MockOperation::Operation(Operation::ConfirmExCommandAndGotoNormalMode),
    MockOperation::SleepFor(Duration::from_millis(50)),
  ];

  let src: &str = r#"
import buf from "rsvim:buf";
import cmd from "rsvim:cmd";
function write(ctx) {
  const bufId = buf.current();
  try {
    const n = buf.writeSync(bufId);
    cmd.echo(`Buffer ${bufId} have been saved, ${n} bytes written.`);
  } catch (e) {
    cmd.echo(`Failed to write buffer ${bufId}: ${e}`);
  }
}

cmd.create("write", write, {}, {alias: "w"});
  "#;

  // Prepare $RSVIM_CONFIG/rsvim.js
  let tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let buf_file = tp.xdg_data_home.join("test.txt");
  let cli_opts = CliOptions::new(false, vec![buf_file]);

  let mut event_loop = make_event_loop(terminal_cols, terminal_rows, cli_opts);

  event_loop.initialize()?;
  event_loop
    .run_with_mock_operations(MockOperationReader::new(mocked_ops))
    .await?;
  event_loop.shutdown()?;

  // After running
  {
    let mut contents = lock!(event_loop.cmdline_text);
    let n = contents.message_history().len();
    assert_eq!(n, 1);

    let actual = contents.message_history_mut().pop();
    info!("actual:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();
    assert!(actual.starts_with("Buffer") && actual.ends_with("bytes written."));
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_buf_write2() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_ops = vec![
    MockOperation::Operation(Operation::GotoCmdlineExMode),
    MockOperation::Operation(Operation::CursorInsert(
      CursorInsertPayload::Text("w".to_compact_string()),
    )),
    MockOperation::Operation(Operation::ConfirmExCommandAndGotoNormalMode),
    MockOperation::SleepFor(Duration::from_millis(50)),
  ];

  let src: &str = r#"
import buf from "rsvim:buf";
import cmd from "rsvim:cmd";
function write(ctx) {
  const bufId = ctx.currentBufferId;
  try {
    const n = buf.writeSync(bufId);
    cmd.echo(`Buffer ${bufId} have been saved, ${n} bytes written.`);
  } catch (e) {
    cmd.echo(`Failed to write buffer ${bufId}: ${e}`);
  }
}

cmd.create("write", write, {}, {alias: "w"});
  "#;

  // Prepare $RSVIM_CONFIG/rsvim.js
  let tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let buf_file = tp.xdg_data_home.join("test.txt");
  let cli_opts = CliOptions::new(false, vec![buf_file]);

  let mut event_loop = make_event_loop(terminal_cols, terminal_rows, cli_opts);

  event_loop.initialize()?;
  event_loop
    .run_with_mock_operations(MockOperationReader::new(mocked_ops))
    .await?;
  event_loop.shutdown()?;

  // After running
  {
    let mut contents = lock!(event_loop.cmdline_text);
    let n = contents.message_history().len();
    assert_eq!(n, 1);

    let actual = contents.message_history_mut().pop();
    info!("actual:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();
    assert!(actual.starts_with("Buffer") && actual.ends_with("bytes written."));
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_buf_write_failed1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_ops = vec![
    MockOperation::Operation(Operation::GotoCmdlineExMode),
    MockOperation::Operation(Operation::CursorInsert(
      CursorInsertPayload::Text("w".to_compact_string()),
    )),
    MockOperation::Operation(Operation::ConfirmExCommandAndGotoNormalMode),
    MockOperation::SleepFor(Duration::from_millis(50)),
  ];

  let src: &str = r#"
import buf from "rsvim:buf";
import cmd from "rsvim:cmd";
function write() {
  const bufId = buf.current();
  try {
    const n = buf.writeSync(bufId);
    cmd.echo(`Buffer ${bufId} have been saved, ${n} bytes written.`);
  } catch (e) {
    cmd.echo(`Failed to write buffer ${bufId}: ${e}`);
  }
}

cmd.create("write", write, {}, {alias: "w"});
  "#;

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);
  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  event_loop.initialize()?;
  event_loop
    .run_with_mock_operations(MockOperationReader::new(mocked_ops))
    .await?;
  event_loop.shutdown()?;

  // After running
  {
    let mut contents = lock!(event_loop.cmdline_text);
    let n = contents.message_history().len();
    assert_eq!(n, 1);

    let actual = contents.message_history_mut().pop();
    info!("actual:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();
    assert!(actual.starts_with("Failed to write buffer"));
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_async_command() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_ops = vec![
    MockOperation::Operation(Operation::GotoCmdlineExMode),
    MockOperation::Operation(Operation::CursorInsert(
      CursorInsertPayload::Text("msg".to_compact_string()),
    )),
    MockOperation::Operation(Operation::ConfirmExCommandAndGotoNormalMode),
    MockOperation::SleepFor(Duration::from_millis(50)),
  ];

  let tmpfile = assert_fs::NamedTempFile::new("README.md").unwrap();
  tmpfile.touch().unwrap();
  tmpfile.write_str("Hello, World").unwrap();
  info!("tmpfile:{:?}", tmpfile.path());

  let src = format!(
    r#"
import cmd from "rsvim:cmd";
import fs from "rsvim:fs";
async function msg() {{
  try {{
    const f = await fs.open({:?});
    const bytes = new Uint8Array(100);
    const n = await f.read(bytes);
    cmd.echo(`n:${{n}}`);
  }} catch (e) {{
    cmd.echo(`Failed:${{e}}`);
  }}
}}

cmd.create("msg", msg);
  "#,
    tmpfile.path()
  );

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

  let cli_opts = CliOptions::empty();
  let mut event_loop = make_event_loop(terminal_cols, terminal_rows, cli_opts);

  event_loop.initialize()?;
  event_loop
    .run_with_mock_operations(MockOperationReader::new(mocked_ops))
    .await?;
  event_loop.shutdown()?;

  // After running
  {
    let mut contents = lock!(event_loop.cmdline_text);
    let n = contents.message_history().len();
    assert_eq!(n, 1);

    let actual = contents.message_history_mut().pop();
    info!("actual:{:?}", actual);
    assert_eq!(actual.unwrap(), "n:12");
  }

  Ok(())
}
