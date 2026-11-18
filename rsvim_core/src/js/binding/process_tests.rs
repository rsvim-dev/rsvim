use crate::cli::CliOptions;
use crate::prelude::*;
use crate::tests::evloop::*;
use crate::tests::log::init as test_log_init;
use regex::Regex;
use std::time::Duration;

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_new_command1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_ops = vec![MockOperation::SleepFor(Duration::from_millis(30))];

  let src: &str = r#"
  import { process } from "rsvim:ext/process";
  const cmd1 = new process.Command("ls");
  if (typeof cmd1 !== "object") {{
    throw new Error("cmd1 is not object");
  }}
  if (cmd1.execPath !== "ls") {{
    throw new Error("cmd1.execPath must be 'ls'");
  }}
  if (typeof cmd1.options !== "object") {{
    throw new Error("cmd1.options is not object");
  }}
  if (!Array.isArray(cmd1.options.args) || cmd1.options.args.length > 0) {{
    throw new Error("cmd1.options.args must be empty array");
  }}
  if (cmd1.options.clearEnv !== false) {{
    throw new Error("cmd1.options.clearEnv must be false");
  }}
  if (cmd1.options.cwd !== null && cmd1.options.cwd !== undefined) {{
    throw new Error("cmd1.options.clearEnv must be null or undefined");
  }}
  if (cmd1.options.detached !== false) {{
    throw new Error("cmd1.options.detached must be false");
  }}
  if (typeof cmd1.options.env !== "object" || Object.keys(cmd1.options.env).length > 0) {{
    throw new Error("cmd1.options.env must be empty object");
  }}
  if (cmd1.options.stdin !== "null") {{
    throw new Error("cmd1.options.stdin is not 'null'");
  }}
  if (cmd1.options.stdout !== "piped") {{
    throw new Error("cmd1.options.stdout is not 'piped'");
  }}
  if (cmd1.options.stderr !== "piped") {{
    throw new Error("cmd1.options.stderr is not 'piped'");
  }}
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
    let contents = lock!(event_loop.cmdline_text);
    let n = contents.message_history().len();
    assert_eq!(n, 0);
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_spawn1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_ops = vec![MockOperation::SleepFor(Duration::from_millis(30000))];

  let src: &str = r#"
  import { cmd } from "rsvim:ext/cmd";
  import { process } from "rsvim:ext/process";
  const cmd1 = new process.Command("ls");
  const child1 = cmd1.spawn();
  cmd.echo(`child1: ${typeof child1}`);
  process.exit();
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
    assert_eq!(actual, "child1: object");
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_spawn2() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_ops = vec![MockOperation::SleepFor(Duration::from_millis(30000))];

  let src: &str = r#"
  import { cmd } from "rsvim:ext/cmd";
  import { process } from "rsvim:ext/process";
  const cmd2 = new process.Command("ls");
  const child2 = cmd2.spawn();
  const stdout2 = child2.stdout;
  const stdout2Text = await stdout2.text();
  cmd.echo(`child2 stdout:${typeof stdout2} text:${stdout2Text}`);
  const stderr2 = child2.stderr;
  const stderr2Text = await stderr2.text();
  cmd.echo(`child2 stderr:${typeof stderr2} text:${stderr2Text}`);
  process.exit();
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
    assert_eq!(n, 2);

    let actual = contents.message_history_mut().pop();
    info!("actual:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();

    let re = Regex::new(r"^child2 stdout:object text:").unwrap();
    assert!(re.is_match(&actual));

    let actual = contents.message_history_mut().pop();
    info!("actual:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();

    let re = Regex::new(r"^child2 stderr:object text:").unwrap();
    assert!(re.is_match(&actual));
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_spawn3() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_ops = vec![MockOperation::SleepFor(Duration::from_millis(30000))];

  let src: &str = r#"
  import { cmd } from "rsvim:ext/cmd";
  import { process } from "rsvim:ext/process";
  const command = new process.Command("ls");
  const child = command.spawn();
  const stdout3 = child.stdout;
  const stdout3Text = await stdout3.text();
  cmd.echo(`child stdout:${typeof stdout3} text:${stdout3Text}`);
  const stderr3 = child.stderr;
  const stderr3Text = await stderr3.text();
  cmd.echo(`child stderr:${typeof stderr3} text:${stderr3Text}`);
  const exitStatus = await child.wait();
  cmd.echo(`child exitStatus:${typeof exitStatus} success:${exitStatus.success} code:${exitStatus.code} signal:${exitStatus.signal}`);
  process.exit();
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
    assert_eq!(n, 3);

    let actual = contents.message_history_mut().pop();
    info!("actual:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();

    let re = Regex::new(r"^child stdout:object text:").unwrap();
    assert!(re.is_match(&actual));

    let actual = contents.message_history_mut().pop();
    info!("actual:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();

    let re = Regex::new(r"^child stderr:object text:").unwrap();
    assert!(re.is_match(&actual));

    let actual = contents.message_history_mut().pop();
    info!("actual:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();

    assert_eq!(
      actual,
      "child exitStatus:object success:true code:0 signal:undefined"
    );
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_spawn4() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_ops = vec![MockOperation::SleepFor(Duration::from_millis(30000))];

  let src: &str = r#"
  import { cmd } from "rsvim:ext/cmd";
  import { process } from "rsvim:ext/process";
  const child = new process.Command("ls").spawn();
  cmd.echo(`disposed1:${child.isDisposed}`);
  const exitStatus1 = await child.wait();
  const exitStatus2 = child.exitStatus;
  cmd.echo(`exitStatus1.code:${exitStatus1.code} exitStatus2.code:${exitStatus2.code}`);
  cmd.echo(`disposed2:${child.isDisposed}`);
  process.exit();
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
    assert_eq!(n, 3);

    let actual = contents.message_history_mut().pop();
    info!("actual:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();

    assert_eq!(actual, "disposed1:false");

    let actual = contents.message_history_mut().pop();
    info!("actual:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();

    assert_eq!(actual, "exitStatus1.code:0 exitStatus2.code:0");

    let actual = contents.message_history_mut().pop();
    info!("actual:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();

    assert_eq!(actual, "disposed2:true");
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_spawn5() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_ops = vec![MockOperation::SleepFor(Duration::from_millis(30000))];

  let src: &str = r#"
  import { cmd } from "rsvim:ext/cmd";
  import { process } from "rsvim:ext/process";
  const child = new process.Command("ls").spawn();
  {
    await using usedChild = child;
    cmd.echo(`disposed1:${usedChild.isDisposed}`);
  }
  cmd.echo(`exitStatus.code:${child.exitStatus.code} disposed2:${child.isDisposed}`);
  process.exit();
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
    assert_eq!(n, 2);

    let actual = contents.message_history_mut().pop();
    info!("actual:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();

    assert_eq!(actual, "disposed1:false");

    let actual = contents.message_history_mut().pop();
    info!("actual:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();

    assert_eq!(actual, "exitStatus.code:0 disposed2:true");
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_exit1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_ops = vec![MockOperation::SleepFor(Duration::from_millis(30))];

  let src: &str = r#"
  import { process } from "rsvim:ext/process";
  setTimeout(() => {
    process.exit();
  }, 1);
    "#;

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  // Before running
  {}

  event_loop.initialize()?;
  let exit_code = event_loop
    .run_with_mock_operations(MockOperationReader::new(mocked_ops))
    .await?;
  event_loop.shutdown()?;

  // After running
  {
    assert_eq!(exit_code, 0);
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_exit2() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_ops = vec![MockOperation::SleepFor(Duration::from_millis(30))];

  let src: &str = r#"
  import { process } from "rsvim:ext/process";
  setTimeout(() => {
    process.exit(-1);
  }, 1);
    "#;

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  // Before running
  {}

  event_loop.initialize()?;
  let exit_code = event_loop
    .run_with_mock_operations(MockOperationReader::new(mocked_ops))
    .await?;
  event_loop.shutdown()?;

  // After running
  {
    assert_eq!(exit_code, -1);
  }

  Ok(())
}
