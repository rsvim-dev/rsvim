use crate::cli::CliOptions;
use crate::prelude::*;
use crate::tests::evloop::*;
use crate::tests::log::init as test_log_init;
use crate::ui::widget::window::opt::*;
use std::time::Duration;

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_timeout1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;

  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let src: &str = r#"
  // Set timeout to update global options.
  const timerId = setTimeout(() => {
    Rsvim.opt.wrap = false;
    Rsvim.opt.lineBreak = true;
  }, 1);
"#;

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  // Before evaluating javascript configs
  {
    let tree = lock!(event_loop.tree);
    let global_local_options = tree.global_local_options();
    assert_eq!(global_local_options.wrap(), WRAP);
    assert_eq!(global_local_options.line_break(), LINE_BREAK);
  }

  event_loop.initialize()?;
  event_loop
    .run_with_mock_events(MockEventReader::new(mocked_events))
    .await?;
  event_loop.shutdown()?;

  // After timeout, it changes to new value
  {
    let tree = lock!(event_loop.tree);
    let global_local_options = tree.global_local_options();
    assert!(!global_local_options.wrap());
    assert!(global_local_options.line_break());
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_timeout2() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;

  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(20))];
  let src: &str = r#"
  // Set timeout to update global options.
  const timerId = setTimeout(() => {
    Rsvim.opt.wrap = false;
    Rsvim.opt.lineBreak = true;
  }, 100);

  // Cancel the timeout immediately
  clearTimeout(timerId);
"#;

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  // Before evaluating javascript configs
  {
    let tree = lock!(event_loop.tree);
    let global_local_options = tree.global_local_options();
    assert_eq!(global_local_options.wrap(), WRAP);
    assert_eq!(global_local_options.line_break(), LINE_BREAK);
  }

  event_loop.initialize()?;
  event_loop
    .run_with_mock_events(MockEventReader::new(mocked_events))
    .await?;
  event_loop.shutdown()?;

  // Still remains the same value
  {
    let tree = lock!(event_loop.tree);
    let global_local_options = tree.global_local_options();
    assert_eq!(global_local_options.wrap(), WRAP);
    assert_eq!(global_local_options.line_break(), LINE_BREAK);
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_timeout3() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;

  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let src: &str = r#"
  // Set timeout to update global options.
  const timerId = setTimeout((arg1, arg2, arg3) => {
    Rsvim.cmd.echo(arg1);
    Rsvim.cmd.echo(arg2);
    Rsvim.cmd.echo(arg3);
    Rsvim.opt.wrap = false;
    Rsvim.opt.lineBreak = true;
  }, 1, "Hello", "World", true);
"#;

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  // Before evaluating javascript configs
  {
    let tree = lock!(event_loop.tree);
    let global_local_options = tree.global_local_options();
    assert_eq!(global_local_options.wrap(), WRAP);
    assert_eq!(global_local_options.line_break(), LINE_BREAK);
  }

  event_loop.initialize()?;
  event_loop
    .run_with_mock_events(MockEventReader::new(mocked_events))
    .await?;
  event_loop.shutdown()?;

  // After timeout, it changes to new value
  {
    let tree = lock!(event_loop.tree);
    let global_local_options = tree.global_local_options();
    assert!(!global_local_options.wrap());
    assert!(global_local_options.line_break());

    let mut contents = lock!(event_loop.cmdline_text);
    assert_eq!(3, contents.message_history().len());
    let actual1 = contents.message_history_mut().pop();
    assert!(actual1.is_some());
    let actual1 = actual1.unwrap();
    assert_eq!(actual1, "Hello");

    let actual2 = contents.message_history_mut().pop();
    assert!(actual2.is_some());
    let actual2 = actual2.unwrap();
    assert_eq!(actual2, "World");

    let actual3 = contents.message_history_mut().pop();
    assert!(actual3.is_some());
    let actual3 = actual3.unwrap();
    assert_eq!(actual3, "true");
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_timeout4() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;

  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(20))];
  let src: &str = r#"
  // Set timeout to update global options.
  const timerId = setTimeout((arg1, arg2, arg3) => {
    Rsvim.opt.wrap = false;
    Rsvim.opt.lineBreak = true;
    Rsvim.cmd.echo(arg1);
    Rsvim.cmd.echo(arg2);
    Rsvim.cmd.echo(arg3);
  }, 100, 1,true,"Ok");

  // Cancel the timeout immediately
  clearTimeout(timerId);
"#;

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  // Before evaluating javascript configs
  {
    let tree = lock!(event_loop.tree);
    let global_local_options = tree.global_local_options();
    assert_eq!(global_local_options.wrap(), WRAP);
    assert_eq!(global_local_options.line_break(), LINE_BREAK);
  }

  event_loop.initialize()?;
  event_loop
    .run_with_mock_events(MockEventReader::new(mocked_events))
    .await?;
  event_loop.shutdown()?;

  // Still remains the same value
  {
    let tree = lock!(event_loop.tree);
    let global_local_options = tree.global_local_options();
    assert_eq!(global_local_options.wrap(), WRAP);
    assert_eq!(global_local_options.line_break(), LINE_BREAK);

    let contents = lock!(event_loop.cmdline_text);
    assert_eq!(0, contents.message_history().len());
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_timeout5() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;

  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let src: &str = r#"
  // Set timeout to update global options.
  const timerId = setTimeout(() => {
    Rsvim.opt.wrap = false;
    Rsvim.opt.lineBreak = true;
  }); // by default `delay` is 1
"#;

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  // Before evaluating javascript configs
  {
    let tree = lock!(event_loop.tree);
    let global_local_options = tree.global_local_options();
    assert_eq!(global_local_options.wrap(), WRAP);
    assert_eq!(global_local_options.line_break(), LINE_BREAK);
  }

  event_loop.initialize()?;
  event_loop
    .run_with_mock_events(MockEventReader::new(mocked_events))
    .await?;
  event_loop.shutdown()?;

  // After timeout, it changes to new value
  {
    let tree = lock!(event_loop.tree);
    let global_local_options = tree.global_local_options();
    assert!(!global_local_options.wrap());
    assert!(global_local_options.line_break());
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_interval1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;

  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let src: &str = r#"
  var a = 1;
  var timerId;
  timerId = setInterval(() => {
    Rsvim.cmd.echo(a);
    a += 1;
    if (a > 3) {
      clearInterval(timerId);
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

  // After timeout
  {
    let mut contents = lock!(event_loop.cmdline_text);
    assert_eq!(3, contents.message_history().len());
    for i in 0..3 {
      let actual = contents.message_history_mut().pop();
      assert!(actual.is_some());
      let actual = actual.unwrap();
      assert_eq!(actual, (i + 1).to_string());
    }
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_interval2() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;

  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let src: &str = r#"
  var n = 0;
  const timerId = setInterval(() => {
    Rsvim.cmd.echo("a");
    n += 1;
  }, 3);

  setTimeout(() => {
    clearInterval(timerId);
    Rsvim.cmd.echo(n);
  }, 20);
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

  // After timeout
  {
    let mut contents = lock!(event_loop.cmdline_text);
    let n = contents.message_history().len();
    info!("n:{}", n);
    assert!(n >= 2);
    for i in 0..n {
      let actual = contents.message_history_mut().pop();
      info!("actual-{}:{:?}", i, actual);
      assert!(actual.is_some());
      let actual = actual.unwrap();
      if i < n - 1 {
        assert_eq!(actual, "a");
      } else {
        assert_eq!(actual, i.to_string());
      }
    }
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_interval3() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;

  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let src: &str = r#"
  var n = 0;
  const timerId = setInterval(() => {
    Rsvim.cmd.echo("a");
    n += 1;
  }, 3);

  setTimeout(() => {
    clearTimeout(timerId);
    Rsvim.cmd.echo(n);
  }, 20);
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

  // After timeout
  {
    let mut contents = lock!(event_loop.cmdline_text);
    let n = contents.message_history().len();
    info!("n:{}", n);
    assert!(n >= 2);
    for i in 0..n {
      let actual = contents.message_history_mut().pop();
      info!("actual-{}:{:?}", i, actual);
      assert!(actual.is_some());
      let actual = actual.unwrap();
      if i < n - 1 {
        assert_eq!(actual, "a");
      } else {
        assert_eq!(actual, i.to_string());
      }
    }
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_timeout6() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;

  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let src: &str = r#"
  // A fractional delay must be truncated into an integer, otherwise the rust
  // side hits a `debug_assert!(is_v8_int!(...))` and panics in debug builds.
  setTimeout(() => {
    Rsvim.cmd.echo("fractional");
  }, 1.5);

  // A `NaN` delay falls back to the lower bound, i.e. 1.
  setTimeout(() => {
    Rsvim.cmd.echo("nan");
  }, NaN);

  // An out-of-range delay is clamped into `[1, 2^31 - 1]`.
  setTimeout(() => {
    Rsvim.cmd.echo("huge");
  }, 1e10);
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

  {
    let mut contents = lock!(event_loop.cmdline_text);
    // The `1e10` delay is clamped to `2^31 - 1` millis, which is far beyond the
    // mocked sleep, so only the first two timers fire.
    assert_eq!(2, contents.message_history().len());
    let actual1 = contents.message_history_mut().pop();
    assert!(actual1.is_some());
    assert_eq!(actual1.unwrap(), "fractional");
    let actual2 = contents.message_history_mut().pop();
    assert!(actual2.is_some());
    assert_eq!(actual2.unwrap(), "nan");
  }

  Ok(())
}
