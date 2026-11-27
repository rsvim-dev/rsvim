use crate::cli::CliOptions;
use crate::js::command::attr::CommandNargs;
use crate::prelude::*;
use crate::tests::evloop::*;
use crate::tests::log::init as test_log_init;
use compact_str::ToCompactString;
use std::time::Duration;

#[tokio::test]
#[cfg_attr(miri, ignore)]
// #[should_panic(
//   expected = "cmd.echo: message is required"
// )]
async fn test_echo1_should_panic_with_missing_param() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(30))];

  let src: &str = r#"
    import cmd from "rsvim:cmd";
    cmd.echo();
    "#;

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  // Before running
  {
    let contents = lock!(event_loop.editor.cmdline_text);
    assert!(contents.message().rope().to_string().is_empty());
  }

  event_loop.initialize()?;
  event_loop
    .run_with_mock_events(MockEventReader::new(mocked_events))
    .await?;
  event_loop.shutdown()?;

  // After running
  {
    let contents = lock!(event_loop.editor.cmdline_text);
    let payload = contents.message().rope().to_string();
    let payload = payload.trim();
    assert!(payload.contains("cmd.echo: message is required"));
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
// #[should_panic(
//   expected = "cmd.echo: message is required"
// )]
async fn test_echo2_should_panic_with_null_param() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(30))];

  let src: &str = r#"
    import cmd from "rsvim:cmd";
    cmd.echo(null);
    "#;

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  // Before running
  {
    let contents = lock!(event_loop.editor.cmdline_text);
    assert!(contents.message().rope().to_string().is_empty());
  }

  event_loop.initialize()?;
  event_loop
    .run_with_mock_events(MockEventReader::new(mocked_events))
    .await?;
  event_loop.shutdown()?;

  // After running
  {
    let contents = lock!(event_loop.editor.cmdline_text);
    let payload = contents.message().rope().to_string();
    let payload = payload.trim();
    assert!(payload.contains("cmd.echo: message is required"));
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_echo3() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(30))];

  let src: &str = r#"
    import cmd from "rsvim:cmd";
    cmd.echo("");
    cmd.echo("Test echo");
    cmd.echo(123);
    cmd.echo(true);
    "#;

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  // Before running
  {
    let contents = lock!(event_loop.editor.cmdline_text);
    assert_eq!(contents.message().rope().to_string(), "");
  }

  event_loop.initialize()?;
  event_loop
    .run_with_mock_events(MockEventReader::new(mocked_events))
    .await?;
  event_loop.shutdown()?;

  // After running
  {
    let contents = lock!(event_loop.editor.cmdline_text);
    let actual = contents.message().rope().to_string();
    let actual = actual.trim();
    assert!(
      actual.is_empty()
        || actual == "Test echo"
        || actual == "123"
        || actual == "true"
    );
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_echo4() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(30))];

  let src: &str = r#"
  import cmd from "rsvim:cmd";
  setTimeout(() => {
    cmd.echo("");
    cmd.echo("Test echo");
    cmd.echo(123);
    cmd.echo(true);
  }, 1);
    "#;

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  // Before running
  {
    let text_cmdline = lock!(event_loop.editor.cmdline_text);
    let actual = text_cmdline.message().rope().to_string();
    assert!(actual.is_empty());
  }

  event_loop.initialize()?;
  event_loop
    .run_with_mock_events(MockEventReader::new(mocked_events))
    .await?;
  event_loop.shutdown()?;

  // After running
  {
    let contents = lock!(event_loop.editor.cmdline_text);
    let actual = contents.message().rope().to_string();
    let actual = actual.trim();
    assert_eq!(actual, "true");
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_create1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];

  let src: &str = r#"
import buf from "rsvim:buf";
import cmd from "rsvim:cmd";
function write() {
  try {
    const bufId = cmd.current();
    const n = buf.writeSync(bufId);
    cmd.echo(`Buffer ${bufId} saved, written ${n} bytes`);
  } catch (e) {
    cmd.echo(`Failed to save buffer ${bufId}: ${e}`);
  }
}

const prev = cmd.create("write", write);
cmd.echo(`Previous command:${prev}`);
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
    assert!(actual.is_some());
    let actual = actual.unwrap();
    assert!(actual.contains("Previous command:undefined"));

    let state_rc = event_loop.js.kernel().clone();
    let state = state_rc.borrow();
    let commands = lock!(state.editor.command_manager);
    assert_eq!(commands.len(), 1);
    let first_command = commands.first_key_value();
    assert!(first_command.is_some());
    let (command_name, command_def) = first_command.unwrap();
    assert_eq!(command_name, "write");
    assert_eq!(command_def.name, "write");
    assert!(!command_def.attributes.bang);
    assert_eq!(command_def.attributes.nargs, CommandNargs::Zero);
    assert!(command_def.options.force);
    assert_eq!(command_def.options.alias, None);
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_recreate1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];

  let src: &str = r#"
import cmd from "rsvim:cmd";
setTimeout(() => {
  const prev1 = cmd.create("write", () => {cmd.echo(1); return 1;});
  cmd.echo(`Previous-1 command:${prev1}`);
  const prev2 = cmd.create("write", () => {cmd.echo(2); return 2;});
  cmd.echo(`Previous-2 command:${typeof prev2}, ${prev2.callback()}`);
});
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
    let actual = contents.message_history_mut().pop();
    info!("actual1:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();
    assert!(actual.contains("Previous-1 command:undefined"));

    let actual = contents.message_history_mut().pop();
    info!("actual2:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();
    assert_eq!(actual, "1");

    let actual = contents.message_history_mut().pop();
    info!("actual3:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();
    assert!(actual.contains("Previous-2 command:object, 1"));

    let state_rc = event_loop.js.kernel().clone();
    let state = state_rc.borrow();
    let commands = lock!(state.editor.command_manager);
    assert_eq!(commands.len(), 1);
    let first_command = commands.first_key_value();
    assert!(first_command.is_some());
    let (command_name, command_def) = first_command.unwrap();
    assert_eq!(command_name, "write");
    assert_eq!(command_def.name, "write");
    assert!(!command_def.attributes.bang);
    assert_eq!(command_def.attributes.nargs, CommandNargs::Zero);
    assert!(command_def.options.force);
    assert_eq!(command_def.options.alias, None);
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_recreate2() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];

  let src: &str = r#"
import cmd from "rsvim:cmd";
setTimeout(() => {
  const prev1 = cmd.create("write", () => {cmd.echo(1); return 1;}, {alias:"w"}, {force:false});
  cmd.echo(`Previous-1 command:${prev1}`);
  const prev2 = cmd.create("writeSync", () => {cmd.echo(2); return 2;}, {alias:"w"}, {force:false});
  cmd.echo(`Previous-2 command:${prev2}`);
});
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
    let actual = contents.message_history_mut().pop();
    info!("actual1:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();
    assert!(actual.contains("Previous-1 command:undefined"));

    let actual = contents.message_history_mut().pop();
    info!("actual2:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();
    assert!(actual.contains("Previous-2 command:undefined"));

    let state_rc = event_loop.js.kernel().clone();
    let state = state_rc.borrow();
    let commands = lock!(state.editor.command_manager);
    assert_eq!(commands.len(), 2);
    for (name, def) in commands.iter() {
      assert!(name == "write" || name == "writeSync");
      assert_eq!(name, def.name);
      assert!(!def.attributes.bang);
      assert_eq!(def.attributes.nargs, CommandNargs::Zero);
      assert!(!def.options.force);
      assert_eq!(def.options.alias, None);
    }
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_recreate_failed1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];

  let src: &str = r#"
import cmd from "rsvim:cmd";
setTimeout(() => {
  try {
    const prev1 = cmd.create("write", () => {cmd.echo(1); return 1;}, {}, {force:false, alias:"w"});
    cmd.echo(`Previous-1 command:${prev1}`);
    const prev2 = cmd.create("write", () => {cmd.echo(2); return 2;}, {}, {force:false});
    cmd.echo(`Previous-2 command:${typeof prev2}, ${prev2.callback()}`);
  } catch(e) {
    cmd.echo(`Failed to recreate command ${e}`);
  }
});
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
    let actual = contents.message_history_mut().pop();
    info!("actual1:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();
    assert!(actual.contains("Previous-1 command:undefined"));

    let actual = contents.message_history_mut().pop();
    info!("actual3:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();
    assert!(actual.contains("Failed to recreate command"));

    let state_rc = event_loop.js.kernel().clone();
    let state = state_rc.borrow();
    let commands = lock!(state.editor.command_manager);
    assert_eq!(commands.len(), 1);
    let first_command = commands.first_key_value();
    assert!(first_command.is_some());
    let (command_name, command_def) = first_command.unwrap();
    assert_eq!(command_name, "write");
    assert_eq!(command_def.name, "write");
    assert!(!command_def.attributes.bang);
    assert_eq!(command_def.attributes.nargs, CommandNargs::Zero);
    assert!(!command_def.options.force);
    assert_eq!(command_def.options.alias, Some("w".to_compact_string()));
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_recreate_failed2() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];

  let src: &str = r#"
import cmd from "rsvim:cmd";
setTimeout(() => {
  try {
    const prev1 = cmd.create("write", () => {cmd.echo(1); return 1;}, {}, {force:false,alias:"w"});
    cmd.echo(`Previous-1 command:${prev1}`);
    const prev2 = cmd.create("writeSync", () => {cmd.echo(2); return 2;}, {}, {force:false, alias:"w"});
    cmd.echo(`Previous-2 command:${typeof prev2}, ${prev2.callback()}`);
  } catch(e) {
    cmd.echo(`Failed to recreate command ${e}`);
  }
});
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
    let actual = contents.message_history_mut().pop();
    info!("actual1:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();
    assert!(actual.contains("Previous-1 command:undefined"));

    let actual = contents.message_history_mut().pop();
    info!("actual3:{:?}", actual);
    assert!(actual.is_some());
    let actual = actual.unwrap();
    assert!(actual.contains("Failed to recreate command"));

    let state_rc = event_loop.js.kernel().clone();
    let state = state_rc.borrow();
    let commands = lock!(state.editor.command_manager);
    assert_eq!(commands.len(), 1);
    let first_command = commands.first_key_value();
    assert!(first_command.is_some());
    let (command_name, command_def) = first_command.unwrap();
    assert_eq!(command_name, "write");
    assert_eq!(command_def.name, "write");
    assert!(!command_def.attributes.bang);
    assert_eq!(command_def.attributes.nargs, CommandNargs::Zero);
    assert!(!command_def.options.force);
    assert_eq!(command_def.options.alias, Some("w".to_compact_string()));
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_list1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];

  let src: &str = r#"
import cmd from "rsvim:cmd";
cmd.create("write", () => {});
cmd.list().forEach((name) => {
  cmd.echo(`name:${name}`);
});
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

    let expects = ["name:write"];

    for i in 0..n {
      let actual = contents.message_history_mut().pop();
      info!("actual{}:{:?}", i, actual);
      assert!(actual.is_some());
      let actual = actual.unwrap();
      assert!(expects.iter().any(|e| *e == actual));
    }

    let state_rc = event_loop.js.kernel().clone();
    let state = state_rc.borrow();
    let commands = lock!(state.editor.command_manager);
    assert_eq!(commands.len(), 1);
    let first_command = commands.first_key_value();
    assert!(first_command.is_some());
    let (command_name, command_def) = first_command.unwrap();
    assert_eq!(command_name, "write");
    assert_eq!(command_def.name, "write");
    assert!(!command_def.attributes.bang);
    assert_eq!(command_def.attributes.nargs, CommandNargs::Zero);
    assert!(command_def.options.force);
    assert_eq!(command_def.options.alias, None);
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_get1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];

  let src: &str = r#"
import cmd from "rsvim:cmd";
cmd.create("write", () => {});
const def = cmd.get("write");
cmd.echo(`name:${def.name}`);
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

    let expects = ["name:write"];

    for i in 0..n {
      let actual = contents.message_history_mut().pop();
      info!("actual{}:{:?}", i, actual);
      assert!(actual.is_some());
      let actual = actual.unwrap();
      assert!(expects.iter().any(|e| *e == actual));
    }

    let state_rc = event_loop.js.kernel().clone();
    let state = state_rc.borrow();
    let commands = lock!(state.editor.command_manager);
    assert_eq!(commands.len(), 1);
    let first_command = commands.first_key_value();
    assert!(first_command.is_some());
    let (command_name, command_def) = first_command.unwrap();
    assert_eq!(command_name, "write");
    assert_eq!(command_def.name, "write");
    assert!(!command_def.attributes.bang);
    assert_eq!(command_def.attributes.nargs, CommandNargs::Zero);
    assert!(command_def.options.force);
    assert_eq!(command_def.options.alias, None);
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_get_failed1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];

  let src: &str = r#"
import cmd from "rsvim:cmd";
cmd.create("write", () => {});
const def = cmd.get("w");
cmd.echo(`name:${def}`);
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

    let expects = ["name:undefined"];

    for i in 0..n {
      let actual = contents.message_history_mut().pop();
      info!("actual{}:{:?}", i, actual);
      assert!(actual.is_some());
      let actual = actual.unwrap();
      assert!(expects.iter().any(|e| *e == actual));
    }

    let state_rc = event_loop.js.kernel().clone();
    let state = state_rc.borrow();
    let commands = lock!(state.editor.command_manager);
    assert_eq!(commands.len(), 1);
    let first_command = commands.first_key_value();
    assert!(first_command.is_some());
    let (command_name, command_def) = first_command.unwrap();
    assert_eq!(command_name, "write");
    assert_eq!(command_def.name, "write");
    assert!(!command_def.attributes.bang);
    assert_eq!(command_def.attributes.nargs, CommandNargs::Zero);
    assert!(command_def.options.force);
    assert_eq!(command_def.options.alias, None);
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_remove1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];

  let src: &str = r#"
import cmd from "rsvim:cmd";
cmd.create("write", () => {});
const prev = cmd.remove("write");
cmd.echo(prev.name);
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
    assert_eq!(actual, "write");

    let state_rc = event_loop.js.kernel().clone();
    let state = state_rc.borrow();
    let commands = lock!(state.editor.command_manager);
    assert!(commands.is_empty());
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_remove2() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];

  let src: &str = r#"
import cmd from "rsvim:cmd";
cmd.create("write", () => {});
const prev = cmd.remove("w");
cmd.echo(`${prev}`);
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
    assert_eq!(actual, "undefined");

    let state_rc = event_loop.js.kernel().clone();
    let state = state_rc.borrow();
    let commands = lock!(state.editor.command_manager);
    assert_eq!(commands.len(), 1);
    let first_command = commands.first_key_value();
    assert!(first_command.is_some());
    let (command_name, command_def) = first_command.unwrap();
    assert_eq!(command_name, "write");
    assert_eq!(command_def.name, "write");
    assert!(!command_def.attributes.bang);
    assert_eq!(command_def.attributes.nargs, CommandNargs::Zero);
    assert!(command_def.options.force);
    assert_eq!(command_def.options.alias, None);
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_create_failed1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];

  let src: &str = r#"
import cmd from "rsvim:cmd";
const prev = cmd.create(1, () => {});
cmd.echo(`Previous command:${prev}`);
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
    assert!(actual.contains(r####"cmd.create: name must be a string, got"####));

    let state_rc = event_loop.js.kernel().clone();
    let state = state_rc.borrow();
    let commands = lock!(state.editor.command_manager);
    assert!(commands.is_empty());
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_create_failed2() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];

  let src: &str = r#"
import cmd from "rsvim:cmd";
const prev = cmd.create("0", () => {});
cmd.echo(`Previous command:${prev}`);
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
    assert!(actual.contains(r####"cmd.create: name is invalid:"####));

    let state_rc = event_loop.js.kernel().clone();
    let state = state_rc.borrow();
    let commands = lock!(state.editor.command_manager);
    assert!(commands.is_empty());
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_create_failed3() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];

  let src: &str = r#"
import cmd from "rsvim:cmd";
const prev = cmd.create("$abc", () => {});
cmd.echo(`Previous command:${prev}`);
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
    assert!(actual.contains(r####"cmd.create: name is invalid:"####));

    let state_rc = event_loop.js.kernel().clone();
    let state = state_rc.borrow();
    let commands = lock!(state.editor.command_manager);
    assert!(commands.is_empty());
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_create_failed4() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];

  let src: &str = r#"
import cmd from "rsvim:cmd";
const prev = cmd.create("abc", 123);
cmd.echo(`Previous command:${prev}`);
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
    assert!(
      actual.contains(r####"cmd.create: callback must be a function, got"####)
    );

    let state_rc = event_loop.js.kernel().clone();
    let state = state_rc.borrow();
    let commands = lock!(state.editor.command_manager);
    assert!(commands.is_empty());
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_create_failed5() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];

  let src: &str = r#"
import cmd from "rsvim:cmd";
const prev = cmd.create("abc", () => {}, {bang:1});
cmd.echo(`Previous command:${prev}`);
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
    assert!(
      actual
        .contains(r####"cmd.create: bang attribute must be a boolean, got"####)
    );

    let state_rc = event_loop.js.kernel().clone();
    let state = state_rc.borrow();
    let commands = lock!(state.editor.command_manager);
    assert!(commands.is_empty());
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_create_failed6() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];

  let src: &str = r#"
import cmd from "rsvim:cmd";
const prev = cmd.create("abc", () => {}, {nargs:"a"});
cmd.echo(`Previous command:${prev}`);
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
    assert!(
      actual.contains(r####"cmd.create: nargs attribute is invalid:"####)
    );

    let state_rc = event_loop.js.kernel().clone();
    let state = state_rc.borrow();
    let commands = lock!(state.editor.command_manager);
    assert!(commands.is_empty());
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_create_failed7() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];

  let src: &str = r#"
import cmd from "rsvim:cmd";
const prev = cmd.create("abc", () => {}, "a");
cmd.echo(`Previous command:${prev}`);
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
    assert!(
      actual.contains(r####"cmd.create: attributes must be an object, got"####)
    );

    let state_rc = event_loop.js.kernel().clone();
    let state = state_rc.borrow();
    let commands = lock!(state.editor.command_manager);
    assert!(commands.is_empty());
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_create_failed8() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];

  let src: &str = r#"
import cmd from "rsvim:cmd";
const prev = cmd.create("abc", () => {}, undefined, 1);
cmd.echo(`Previous command:${prev}`);
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
    assert!(
      actual.contains(r####"cmd.create: options must be an object, got"####)
    );

    let state_rc = event_loop.js.kernel().clone();
    let state = state_rc.borrow();
    let commands = lock!(state.editor.command_manager);
    assert!(commands.is_empty());
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_create_failed9() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];

  let src: &str = r#"
import cmd from "rsvim:cmd";
const prev = cmd.create("abc", () => {}, undefined, {force:"b"});
cmd.echo(`Previous command:${prev}`);
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
    assert!(
      actual
        .contains(r####"cmd.create: force option must be a boolean, got"####)
    );

    let state_rc = event_loop.js.kernel().clone();
    let state = state_rc.borrow();
    let commands = lock!(state.editor.command_manager);
    assert!(commands.is_empty());
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_create_failed10() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];

  let src: &str = r#"
import cmd from "rsvim:cmd";
const prev = cmd.create("abc", () => {}, undefined, {alias:1});
cmd.echo(`Previous command:${prev}`);
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
    assert!(actual.contains(
      r####"cmd.create: alias option must be a string or nil, got"####
    ));

    let state_rc = event_loop.js.kernel().clone();
    let state = state_rc.borrow();
    let commands = lock!(state.editor.command_manager);
    assert!(commands.is_empty());
  }

  Ok(())
}

use crate::js::binding::install_bindings;
use crate::tests::js::with_kernel;
use rquickjs::Context;
use rquickjs::Runtime;

#[test]
#[cfg_attr(miri, ignore)]
fn echo_reaches_the_message_history() {
  test_log_init();
  with_kernel(|ctx, kernel| {
    ctx
      .eval::<(), _>("__InternalRsvimGlobalObject.cmd_echo('hello')")
      .unwrap();
    let history = lock!(kernel.borrow().editor.cmdline_text)
      .message_history()
      .iter()
      .cloned()
      .collect::<Vec<String>>();
    assert_eq!(history, vec!["hello".to_string()]);
  });
}

#[test]
#[cfg_attr(miri, ignore)]
fn echo_through_the_javascript_wrapper() {
  test_log_init();
  with_kernel(|ctx, kernel| {
    ctx
      .eval::<(), _>(
        "const cmd = __InternalRsvimGlobalObject; cmd.cmd_echo('wrapped');",
      )
      .unwrap();
    let history = lock!(kernel.borrow().editor.cmdline_text)
      .message_history()
      .iter()
      .cloned()
      .collect::<Vec<String>>();
    assert_eq!(history, vec!["wrapped".to_string()]);
  });
}

#[test]
#[cfg_attr(miri, ignore)]
fn echo_without_a_kernel_is_a_javascript_error() {
  test_log_init();
  let rt = Runtime::new().unwrap();
  let ctx = Context::full(&rt).unwrap();
  ctx.with(|ctx| {
    // Bindings without userdata: the call must raise, not panic.
    install_bindings(&ctx).unwrap();
    let result = ctx.eval::<(), _>("__InternalRsvimGlobalObject.cmd_echo('x')");
    assert!(result.is_err());
  });
}

#[test]
#[cfg_attr(miri, ignore)]
fn create_registers_a_command() {
  test_log_init();
  with_kernel(|ctx, kernel| {
    ctx
      .eval::<(), _>(
        "__InternalRsvimGlobalObject.cmd_create('write', () => 42,
             {bang: true, nargs: '1'}, {force: false, alias: 'w'})",
      )
      .unwrap();
    let manager = kernel.borrow().editor.command_manager.clone();
    let manager = lock!(manager);
    let def = manager
      .get("write")
      .expect("the command must be registered");
    assert_eq!(def.name, "write".to_compact_string());
    assert!(def.attributes.bang);
    assert_eq!(def.attributes.nargs, CommandNargs::One);
    assert!(!def.options.force);
    assert_eq!(def.options.alias, Some("w".to_compact_string()));
  });
}

#[test]
#[cfg_attr(miri, ignore)]
fn create_without_options_uses_the_defaults() {
  test_log_init();
  with_kernel(|ctx, kernel| {
    // The javascript wrapper fills `{force: true}` in; here we call the raw
    // binding to prove the fallback matches.
    ctx
      .eval::<(), _>(
        "__InternalRsvimGlobalObject.cmd_create('bare', () => 0,
             undefined, undefined)",
      )
      .unwrap();
    let manager = kernel.borrow().editor.command_manager.clone();
    let manager = lock!(manager);
    let def = manager.get("bare").expect("the command must be registered");
    assert_eq!(def.attributes.nargs, CommandNargs::Zero);
    assert!(def.options.force);
    assert_eq!(def.options.alias, None);
  });
}

#[test]
#[cfg_attr(miri, ignore)]
fn create_twice_returns_the_previous_definition() {
  test_log_init();
  with_kernel(|ctx, kernel| {
    ctx
      .eval::<(), _>(
        "const g = __InternalRsvimGlobalObject;
           g.cmd_create('twice', () => 'first', undefined, {force: true});
           globalThis.__prev = g.cmd_create('twice', () => 'second',
             undefined, {force: true});
           globalThis.__prevName = __prev.name;
           globalThis.__prevCallable = typeof __prev.callback;",
      )
      .unwrap();
    let name: String = ctx.eval("globalThis.__prevName").unwrap();
    assert_eq!(name, "twice");
    let kind: String = ctx.eval("globalThis.__prevCallable").unwrap();
    assert_eq!(kind, "function");
    // The first callback must still be reachable through the returned object.
    let called: String = ctx.eval("globalThis.__prev.callback()").unwrap();
    assert_eq!(called, "first");
    let manager = kernel.borrow().editor.command_manager.clone();
    assert_eq!(lock!(manager).len(), 1);
  });
}

#[test]
#[cfg_attr(miri, ignore)]
fn create_without_force_rejects_a_duplicate() {
  test_log_init();
  with_kernel(|ctx, _kernel| {
    let result = ctx.eval::<(), _>(
      "const g = __InternalRsvimGlobalObject;
         g.cmd_create('dup', () => 0, undefined, {force: false});
         g.cmd_create('dup', () => 0, undefined, {force: false});",
    );
    assert!(result.is_err(), "the second registration must throw");
  });
}

#[test]
#[cfg_attr(miri, ignore)]
fn list_get_and_remove_round_trip() {
  test_log_init();
  with_kernel(|ctx, kernel| {
    ctx
      .eval::<(), _>(
        "const g = __InternalRsvimGlobalObject;
           g.cmd_create('alpha', () => 'a');
           g.cmd_create('beta', () => 'b');
           globalThis.__names = g.cmd_list().join(',');
           globalThis.__beta = g.cmd_get('beta').name;
           globalThis.__missing = g.cmd_get('nope');
           globalThis.__removed = g.cmd_remove('alpha').name;
           globalThis.__after = g.cmd_list().join(',');",
      )
      .unwrap();
    let names: String = ctx.eval("globalThis.__names").unwrap();
    assert_eq!(names, "alpha,beta");
    let beta: String = ctx.eval("globalThis.__beta").unwrap();
    assert_eq!(beta, "beta");
    let missing: String = ctx.eval("String(globalThis.__missing)").unwrap();
    assert_eq!(missing, "undefined");
    let removed: String = ctx.eval("globalThis.__removed").unwrap();
    assert_eq!(removed, "alpha");
    let after: String = ctx.eval("globalThis.__after").unwrap();
    assert_eq!(after, "beta");
    let manager = kernel.borrow().editor.command_manager.clone();
    assert_eq!(lock!(manager).len(), 1);
  });
}
