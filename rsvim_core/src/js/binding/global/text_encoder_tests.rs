use crate::cli::CliOptions;
use crate::prelude::*;
use crate::tests::evloop::*;
use crate::tests::log::init as test_log_init;
use std::time::Duration;

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_encode1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;

  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let src: &str = r#"
  const encoder = new TextEncoder();
  const s1 = "This is some data";
  const s2 = "你好，世界！";
  const bytes1 = encoder.encode(s1);
  const bytes2 = encoder.encode(s2);

  if (bytes1.byteLength !== 17) {
    Rsvim.cmd.echo(`bytes1 failed, bytesLen:${bytes1.byteLength}`);
  }

  if (bytes2.byteLength !== 18 ) {
    Rsvim.cmd.echo(`bytes2 failed, bytesLen:${bytes2.byteLength}`);
  }

  Rsvim.cmd.echo(`encoding:${encoder.encoding}`);
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

  // After
  {
    let mut contents = lock!(event_loop.cmdline_text);
    let n = contents.message_history().len();
    assert_eq!(n, 1);
    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "encoding:utf-8");
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_encode2() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;

  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let src: &str = r#"
  const encoder = new TextEncoder();
  const s1 = "This is some data";
  const s2 = "你好，世界！";
  const buf1 = new Uint8Array(s1.length * 4);
  const buf2 = new Uint8Array(s2.length * 4);
  const res1 = encoder.encodeInto(s1, buf1);
  const res2 = encoder.encodeInto(s2, buf2);

  if (res1.read !== s1.length || res1.written !== 17) {
    Rsvim.cmd.echo(`buf1 failed, res1.read:${res1.read}, res1.written:${res1.written}, s1.length:${s1.length}`);
  }

  if (res2.read !== s2.length || res2.written !== 18) {
    Rsvim.cmd.echo(`buf2 failed, res2.read:${res2.read}, res2.written:${res2.written}, s2.length:${s2.length}`);
  }
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

  // After
  {
    let contents = lock!(event_loop.cmdline_text);
    let actual = contents.message_history().is_empty();
    assert!(actual);
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_encode3() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;

  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let src: &str = r#"
  const encoder = new TextEncoder();
  const s1 = "This is some data";
  const s2 = "你好，世界！";
  const buf1 = new Uint8Array(16);
  const buf2 = new Uint8Array(17);
  const res1 = encoder.encodeInto(s1, buf1);
  const res2 = encoder.encodeInto(s2, buf2);

  if (res1.read !== 16 || res1.written !== 16) {
    Rsvim.cmd.echo(`buf1 failed, res1.read:${res1.read}, res1.written:${res1.written}, s1.length:${s1.length}`);
  }

  if (res2.read !== 5 || res2.written !== 15) {
    Rsvim.cmd.echo(`buf2 failed, res2.read:${res2.read}, res2.written:${res2.written}, s2.length:${s2.length}`);
  }
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

  // After
  {
    let contents = lock!(event_loop.cmdline_text);
    let actual = contents.message_history().is_empty();
    assert!(actual);
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_decode1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;

  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let src: &str = r#"
  const s1 = "This is some data";
  const s2 = "你好，世界！";
  const encoder = new TextEncoder();
  const bytes1 = encoder.encode(s1);
  const bytes2 = encoder.encode(s2);

  const decoder3 = new TextDecoder();
  const s3 = decoder3.decode(bytes1);
  const decoder4 = new TextDecoder();
  const s4 = decoder4.decode(bytes2);
  const s5 = decoder4.decode(bytes2);

  if (s1 !== s3) {
    Rsvim.cmd.echo("failed-1");
  }
  if (s2 !== s4) {
    Rsvim.cmd.echo("failed-2");
  }
  if (s2 !== s5) {
    Rsvim.cmd.echo("failed-3");
  }

  Rsvim.cmd.echo(decoder3.encoding);
  Rsvim.cmd.echo(decoder3.fatal);
  Rsvim.cmd.echo(decoder3.ignoreBOM);
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

  // After
  {
    let mut contents = lock!(event_loop.cmdline_text);
    let n = contents.message_history().len();
    assert_eq!(n, 3);
    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "utf-8");
    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "false");
    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "false");
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_decode2() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;

  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let src: &str = r#"
  const s1 = "This is some data";
  const s2 = "你好，世界！";
  const encoder = new TextEncoder();
  const bytes1 = encoder.encode(s1);
  const bytes2 = encoder.encode(s2);

  const decoder3 = new TextDecoder("utf-8", {fatal: true, ignoreBOM: false});
  const s3 = decoder3.decode(bytes1);
  const decoder4 = new TextDecoder("utf-8", {fatal:false, ignoreBOM: true});
  const s4 = decoder4.decode(bytes2);
  const s5 = decoder4.decode(bytes2);

  if (s1 !== s3) {
    Rsvim.cmd.echo("failed-1");
  }
  if (s2 !== s4) {
    Rsvim.cmd.echo("failed-2");
  }
  if (s2 !== s5) {
    Rsvim.cmd.echo("failed-3");
  }

  Rsvim.cmd.echo(decoder3.encoding);
  Rsvim.cmd.echo(decoder3.fatal);
  Rsvim.cmd.echo(decoder3.ignoreBOM);
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

  // After
  {
    let mut contents = lock!(event_loop.cmdline_text);
    let n = contents.message_history().len();
    assert_eq!(n, 3);
    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "utf-8");
    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "true");
    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "false");
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_decode3() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;

  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let src: &str = r#"
  const fixture = new Uint8Array([
    0xf0, 0x9d, 0x93, 0xbd,
    0xf0, 0x9d, 0x93, 0xae,
    0xf0, 0x9d, 0x94, 0x81,
    0xf0, 0x9d, 0x93, 0xbd
  ]);
  const decoder = new TextDecoder();
  if (decoder.decode(fixture) !== "𝓽𝓮𝔁𝓽") {
    Rsvim.cmd.echo("failed");
  }
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

  // After
  {
    let contents = lock!(event_loop.cmdline_text);
    let actual = contents.message_history().is_empty();
    assert!(actual);
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_decode4() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;

  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let src: &str = r#"
  const fixture1 = new Uint8Array([
    0xf0, 0x9d, 0x93, 0xbd,
  ]);
  const fixture2 = new Uint8Array([
    0xf0, 0x9d, 0x93, 0xae,
    0xf0, 0x9d, 0x94, 0x81,
    0xf0, 0x9d, 0x93, 0xbd
  ]);

  const decoder = new TextDecoder();
  let s = "";
  s += decoder.decode(fixture1, {stream:true});
  s += decoder.decode(fixture2, {stream:true});
  s += decoder.decode(undefined, {stream:false});

  if (s !== "𝓽𝓮𝔁𝓽") {
    Rsvim.cmd.echo("failed");
  }
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

  // After
  {
    let contents = lock!(event_loop.cmdline_text);
    let actual = contents.message_history().is_empty();
    assert!(actual);
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_decode5() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;

  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let src: &str = r#"
  const fixture = new Uint8Array([0x89, 0x95, 0x9f, 0xbf]);
  const decoder = new TextDecoder("ascii");
  if (decoder.decode(fixture) !== "‰•Ÿ¿") {
    Rsvim.cmd.echo("failed");
  }
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

  // After
  {
    let contents = lock!(event_loop.cmdline_text);
    let actual = contents.message_history().is_empty();
    assert!(actual);
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_decode_failed1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;

  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let src: &str = r#"
  const decoder = new TextDecoder("FooEncoding");
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

  // After
  {
    let mut contents = lock!(event_loop.cmdline_text);
    let n = contents.message_history().len();
    assert_eq!(n, 1);
    let actual = contents.message_history_mut().pop().unwrap();
    info!("actual:{:?}", actual);
    assert!(actual.contains("unknown encoding FooEncoding"));
  }

  Ok(())
}
