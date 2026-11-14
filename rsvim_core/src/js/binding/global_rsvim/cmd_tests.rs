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
//   expected = "Rsvim.cmd.echo: message is required"
// )]
async fn test_echo1_should_panic_with_missing_param() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(30))];

  let src: &str = r#"
    Rsvim.cmd.echo();
    "#;

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  // Before running
  {
    let contents = lock!(event_loop.cmdline_text);
    assert!(contents.message().rope().to_string().is_empty());
  }

  event_loop.initialize()?;
  event_loop
    .run_with_mock_events(MockEventReader::new(mocked_events))
    .await?;
  event_loop.shutdown()?;

  // After running
  {
    let contents = lock!(event_loop.cmdline_text);
    let payload = contents.message().rope().to_string();
    let payload = payload.trim();
    assert!(payload.contains("Rsvim.cmd.echo: message is required"));
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
// #[should_panic(
//   expected = "Rsvim.cmd.echo: message is required"
// )]
async fn test_echo2_should_panic_with_null_param() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(30))];

  let src: &str = r#"
    Rsvim.cmd.echo(null);
    "#;

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  // Before running
  {
    let contents = lock!(event_loop.cmdline_text);
    assert!(contents.message().rope().to_string().is_empty());
  }

  event_loop.initialize()?;
  event_loop
    .run_with_mock_events(MockEventReader::new(mocked_events))
    .await?;
  event_loop.shutdown()?;

  // After running
  {
    let contents = lock!(event_loop.cmdline_text);
    let payload = contents.message().rope().to_string();
    let payload = payload.trim();
    assert!(payload.contains("Rsvim.cmd.echo: message is required"));
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
    Rsvim.cmd.echo("");
    Rsvim.cmd.echo("Test echo");
    Rsvim.cmd.echo(123);
    Rsvim.cmd.echo(true);
    "#;

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  // Before running
  {
    let contents = lock!(event_loop.cmdline_text);
    assert_eq!(contents.message().rope().to_string(), "");
  }

  event_loop.initialize()?;
  event_loop
    .run_with_mock_events(MockEventReader::new(mocked_events))
    .await?;
  event_loop.shutdown()?;

  // After running
  {
    let contents = lock!(event_loop.cmdline_text);
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
  setTimeout(() => {
    Rsvim.cmd.echo("");
    Rsvim.cmd.echo("Test echo");
    Rsvim.cmd.echo(123);
    Rsvim.cmd.echo(true);
  }, 1);
    "#;

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  // Before running
  {
    let text_cmdline = lock!(event_loop.cmdline_text);
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
    let contents = lock!(event_loop.cmdline_text);
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
function write() {
  try {
    const bufId = Rsvim.cmd.current();
    const n = Rsvim.buf.writeSync(bufId);
    Rsvim.cmd.echo(`Buffer ${bufId} saved, written ${n} bytes`);
  } catch (e) {
    Rsvim.cmd.echo(`Failed to save buffer ${bufId}: ${e}`);
  }
}

const prev = Rsvim.cmd.create("write", write);
Rsvim.cmd.echo(`Previous command:${prev}`);
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
    assert!(actual.is_some());
    let actual = actual.unwrap();
    assert!(actual.contains("Previous command:undefined"));

    let state_rc = event_loop.js_runtime.get_state();
    let state = state_rc.borrow();
    let commands = lock!(state.command_manager);
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
setTimeout(() => {
  const prev1 = Rsvim.cmd.create("write", () => {Rsvim.cmd.echo(1); return 1;});
  Rsvim.cmd.echo(`Previous-1 command:${prev1}`);
  const prev2 = Rsvim.cmd.create("write", () => {Rsvim.cmd.echo(2); return 2;});
  Rsvim.cmd.echo(`Previous-2 command:${typeof prev2}, ${prev2.callback()}`);
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
    let mut contents = lock!(event_loop.cmdline_text);
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

    let state_rc = event_loop.js_runtime.get_state();
    let state = state_rc.borrow();
    let commands = lock!(state.command_manager);
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
setTimeout(() => {
  const prev1 = Rsvim.cmd.create("write", () => {Rsvim.cmd.echo(1); return 1;}, {alias:"w"}, {force:false});
  Rsvim.cmd.echo(`Previous-1 command:${prev1}`);
  const prev2 = Rsvim.cmd.create("writeSync", () => {Rsvim.cmd.echo(2); return 2;}, {alias:"w"}, {force:false});
  Rsvim.cmd.echo(`Previous-2 command:${prev2}`);
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
    let mut contents = lock!(event_loop.cmdline_text);
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

    let state_rc = event_loop.js_runtime.get_state();
    let state = state_rc.borrow();
    let commands = lock!(state.command_manager);
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
setTimeout(() => {
  try {
    const prev1 = Rsvim.cmd.create("write", () => {Rsvim.cmd.echo(1); return 1;}, {}, {force:false, alias:"w"});
    Rsvim.cmd.echo(`Previous-1 command:${prev1}`);
    const prev2 = Rsvim.cmd.create("write", () => {Rsvim.cmd.echo(2); return 2;}, {}, {force:false});
    Rsvim.cmd.echo(`Previous-2 command:${typeof prev2}, ${prev2.callback()}`);
  } catch(e) {
    Rsvim.cmd.echo(`Failed to recreate command ${e}`);
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
    let mut contents = lock!(event_loop.cmdline_text);
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

    let state_rc = event_loop.js_runtime.get_state();
    let state = state_rc.borrow();
    let commands = lock!(state.command_manager);
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
setTimeout(() => {
  try {
    const prev1 = Rsvim.cmd.create("write", () => {Rsvim.cmd.echo(1); return 1;}, {}, {force:false,alias:"w"});
    Rsvim.cmd.echo(`Previous-1 command:${prev1}`);
    const prev2 = Rsvim.cmd.create("writeSync", () => {Rsvim.cmd.echo(2); return 2;}, {}, {force:false, alias:"w"});
    Rsvim.cmd.echo(`Previous-2 command:${typeof prev2}, ${prev2.callback()}`);
  } catch(e) {
    Rsvim.cmd.echo(`Failed to recreate command ${e}`);
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
    let mut contents = lock!(event_loop.cmdline_text);
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

    let state_rc = event_loop.js_runtime.get_state();
    let state = state_rc.borrow();
    let commands = lock!(state.command_manager);
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
Rsvim.cmd.create("write", () => {});
Rsvim.cmd.list().forEach((name) => {
  Rsvim.cmd.echo(`name:${name}`);
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
    let mut contents = lock!(event_loop.cmdline_text);
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

    let state_rc = event_loop.js_runtime.get_state();
    let state = state_rc.borrow();
    let commands = lock!(state.command_manager);
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
Rsvim.cmd.create("write", () => {});
const def = Rsvim.cmd.get("write");
Rsvim.cmd.echo(`name:${def.name}`);
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

    let expects = ["name:write"];

    for i in 0..n {
      let actual = contents.message_history_mut().pop();
      info!("actual{}:{:?}", i, actual);
      assert!(actual.is_some());
      let actual = actual.unwrap();
      assert!(expects.iter().any(|e| *e == actual));
    }

    let state_rc = event_loop.js_runtime.get_state();
    let state = state_rc.borrow();
    let commands = lock!(state.command_manager);
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
Rsvim.cmd.create("write", () => {});
const def = Rsvim.cmd.get("w");
Rsvim.cmd.echo(`name:${def}`);
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

    let expects = ["name:undefined"];

    for i in 0..n {
      let actual = contents.message_history_mut().pop();
      info!("actual{}:{:?}", i, actual);
      assert!(actual.is_some());
      let actual = actual.unwrap();
      assert!(expects.iter().any(|e| *e == actual));
    }

    let state_rc = event_loop.js_runtime.get_state();
    let state = state_rc.borrow();
    let commands = lock!(state.command_manager);
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
Rsvim.cmd.create("write", () => {});
const prev = Rsvim.cmd.remove("write");
Rsvim.cmd.echo(prev.name);
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
    assert_eq!(actual, "write");

    let state_rc = event_loop.js_runtime.get_state();
    let state = state_rc.borrow();
    let commands = lock!(state.command_manager);
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
Rsvim.cmd.create("write", () => {});
const prev = Rsvim.cmd.remove("w");
Rsvim.cmd.echo(`${prev}`);
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
    assert_eq!(actual, "undefined");

    let state_rc = event_loop.js_runtime.get_state();
    let state = state_rc.borrow();
    let commands = lock!(state.command_manager);
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
const prev = Rsvim.cmd.create(1, () => {});
Rsvim.cmd.echo(`Previous command:${prev}`);
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
    assert!(
      actual.contains(r####"Rsvim.cmd.create: name must be a string, got"####)
    );

    let state_rc = event_loop.js_runtime.get_state();
    let state = state_rc.borrow();
    let commands = lock!(state.command_manager);
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
const prev = Rsvim.cmd.create("0", () => {});
Rsvim.cmd.echo(`Previous command:${prev}`);
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
    assert!(actual.contains(r####"Rsvim.cmd.create: name is invalid:"####));

    let state_rc = event_loop.js_runtime.get_state();
    let state = state_rc.borrow();
    let commands = lock!(state.command_manager);
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
const prev = Rsvim.cmd.create("$abc", () => {});
Rsvim.cmd.echo(`Previous command:${prev}`);
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
    assert!(actual.contains(r####"Rsvim.cmd.create: name is invalid:"####));

    let state_rc = event_loop.js_runtime.get_state();
    let state = state_rc.borrow();
    let commands = lock!(state.command_manager);
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
const prev = Rsvim.cmd.create("abc", 123);
Rsvim.cmd.echo(`Previous command:${prev}`);
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
    assert!(
      actual.contains(
        r####"Rsvim.cmd.create: callback must be a function, got"####
      )
    );

    let state_rc = event_loop.js_runtime.get_state();
    let state = state_rc.borrow();
    let commands = lock!(state.command_manager);
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
const prev = Rsvim.cmd.create("abc", () => {}, {bang:1});
Rsvim.cmd.echo(`Previous command:${prev}`);
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
    assert!(actual.contains(
      r####"Rsvim.cmd.create: bang attribute must be a boolean, got"####
    ));

    let state_rc = event_loop.js_runtime.get_state();
    let state = state_rc.borrow();
    let commands = lock!(state.command_manager);
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
const prev = Rsvim.cmd.create("abc", () => {}, {nargs:"a"});
Rsvim.cmd.echo(`Previous command:${prev}`);
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
    assert!(
      actual.contains(r####"Rsvim.cmd.create: nargs attribute is invalid:"####)
    );

    let state_rc = event_loop.js_runtime.get_state();
    let state = state_rc.borrow();
    let commands = lock!(state.command_manager);
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
const prev = Rsvim.cmd.create("abc", () => {}, "a");
Rsvim.cmd.echo(`Previous command:${prev}`);
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
    assert!(actual.contains(
      r####"Rsvim.cmd.create: attributes must be an object, got"####
    ));

    let state_rc = event_loop.js_runtime.get_state();
    let state = state_rc.borrow();
    let commands = lock!(state.command_manager);
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
const prev = Rsvim.cmd.create("abc", () => {}, undefined, 1);
Rsvim.cmd.echo(`Previous command:${prev}`);
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
    assert!(
      actual
        .contains(r####"Rsvim.cmd.create: options must be an object, got"####)
    );

    let state_rc = event_loop.js_runtime.get_state();
    let state = state_rc.borrow();
    let commands = lock!(state.command_manager);
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
const prev = Rsvim.cmd.create("abc", () => {}, undefined, {force:"b"});
Rsvim.cmd.echo(`Previous command:${prev}`);
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
    assert!(actual.contains(
      r####"Rsvim.cmd.create: force option must be a boolean, got"####
    ));

    let state_rc = event_loop.js_runtime.get_state();
    let state = state_rc.borrow();
    let commands = lock!(state.command_manager);
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
const prev = Rsvim.cmd.create("abc", () => {}, undefined, {alias:1});
Rsvim.cmd.echo(`Previous command:${prev}`);
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
    assert!(actual.contains(
      r####"Rsvim.cmd.create: alias option must be a string or nil, got"####
    ));

    let state_rc = event_loop.js_runtime.get_state();
    let state = state_rc.borrow();
    let commands = lock!(state.command_manager);
    assert!(commands.is_empty());
  }

  Ok(())
}
