use crate::cli::CliOptions;
use crate::prelude::*;
use crate::tests::evloop::*;
use crate::tests::log::init as test_log_init;
use assert_fs::prelude::FileTouch;
use assert_fs::prelude::FileWriteStr;
use assert_fs::prelude::PathChild;
use regex::Regex;
use std::time::Duration;

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_open_close1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let tmpfile = assert_fs::NamedTempFile::new("README.md").unwrap();
  tmpfile.touch().unwrap();
  info!("tmpfile:{:?}", tmpfile.path());

  let a = format!("{:?}", tmpfile.path());
  let b = format!("{:?}", tmpfile.path().to_string_lossy().to_string());
  info!("a:{a:?}, b:{b:?}, a.contains(b):{}", a.contains(&b));
  assert!(a.contains(&b));

  let src = format!(
    r#"
  import {{ fs }} from "rsvim:ext/fs";
  const f = await fs.open({:?});
  if (f.isDisposed) {{
    throw new Error("It cannot be closed");
  }}
  f.close();
  if (!f.isDisposed) {{
    throw new Error("It must be closed");
  }}
    "#,
    tmpfile.path()
  );
  info!("src:{:?}", src);

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  event_loop.initialize()?;
  event_loop
    .run_with_mock_events(MockEventReader::new(mocked_events))
    .await?;
  event_loop.shutdown()?;

  // After running
  {
    let contents = lock!(event_loop.cmdline_text);
    let actual = contents.message_history().is_empty();
    assert!(actual);
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_open_close2() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let tmpfile = assert_fs::NamedTempFile::new("README.md").unwrap();
  tmpfile.touch().unwrap();
  info!("tmpfile:{:?}", tmpfile.path());

  let a = format!("{:?}", tmpfile.path());
  let b = format!("{:?}", tmpfile.path().to_string_lossy().to_string());
  info!("a:{a:?}, b:{b:?}, a.contains(b):{}", a.contains(&b));
  assert!(a.contains(&b));

  let src = format!(
    r#"
  import {{ fs }} from "rsvim:ext/fs";
  using f = fs.openSync({:?});
    "#,
    tmpfile.path()
  );
  info!("src:{:?}", src);

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  event_loop.initialize()?;
  event_loop
    .run_with_mock_events(MockEventReader::new(mocked_events))
    .await?;
  event_loop.shutdown()?;

  // After running
  {
    let contents = lock!(event_loop.cmdline_text);
    let actual = contents.message_history().is_empty();
    assert!(actual);
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_open_close3() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let tmpfile = assert_fs::NamedTempFile::new("README.md").unwrap();
  info!("tmpfile:{:?}", tmpfile);

  let src = format!(
    r#"
  import {{ fs }} from "rsvim:ext/fs";
  const f = await fs.open({:?}, {{ create: true, write: true }});
  if (f.isDisposed) {{
    throw new Error("It cannot be closed");
  }}
  f.close();
  if (!f.isDisposed) {{
    throw new Error("It must be closed");
  }}
    "#,
    tmpfile.path()
  );
  info!("src:{:?}", src);

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  event_loop.initialize()?;
  event_loop
    .run_with_mock_events(MockEventReader::new(mocked_events))
    .await?;
  event_loop.shutdown()?;

  // After running
  {
    let contents = lock!(event_loop.cmdline_text);
    let actual = contents.message_history().is_empty();
    assert!(actual);
    assert!(tmpfile.exists());
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_open_close4() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let tmpfile = assert_fs::NamedTempFile::new("README.md").unwrap();
  info!("tmpfile:{:?}", tmpfile);

  let src = format!(
    r#"
  import {{ fs }} from "rsvim:ext/fs";
  const f = fs.openSync({:?}, {{ create: true, write: true }});
  if (f.isDisposed) {{
    throw new Error("It cannot be closed");
  }}
  f.close();
  if (!f.isDisposed) {{
    throw new Error("It must be closed");
  }}
    "#,
    tmpfile.path()
  );
  info!("src:{:?}", src);

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  event_loop.initialize()?;
  event_loop
    .run_with_mock_events(MockEventReader::new(mocked_events))
    .await?;
  event_loop.shutdown()?;

  // After running
  {
    let contents = lock!(event_loop.cmdline_text);
    let actual = contents.message_history().is_empty();
    assert!(actual);
    assert!(tmpfile.exists());
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_open_close_failed1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let tmpfile = assert_fs::NamedTempFile::new("README.md").unwrap();
  info!("tmpfile:{:?}", tmpfile);

  let src = format!(
    r#"
import {{ cmd }} from "rsvim:ext/cmd";
import {{ fs }} from "rsvim:ext/fs";
try {{
  const f = await fs.open({:?});
}} catch (e) {{
  cmd.echo(e);
}}
    "#,
    tmpfile.path()
  );
  info!("src:{:?}", src);

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

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
    assert!(actual.contains("Failed to open file"));
    assert!(!tmpfile.exists());
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_open_close_failed2() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let tmpfile = assert_fs::NamedTempFile::new("README.md").unwrap();
  info!("tmpfile:{:?}", tmpfile);

  let src = format!(
    r#"
import {{ cmd }} from "rsvim:ext/cmd";
import {{ fs }} from "rsvim:ext/fs";
try {{
  const f = fs.openSync({:?});
}} catch (e) {{
  cmd.echo(e);
}}
    "#,
    tmpfile.path()
  );
  info!("src:{:?}", src);

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

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
    assert!(actual.contains("Failed to open file"));
    assert!(!tmpfile.exists());
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_read_write1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let tmpfile = assert_fs::NamedTempFile::new("README.md").unwrap();
  info!("tmpfile:{:?}", tmpfile);
  tmpfile.touch().unwrap();
  tmpfile.write_str("Hello, World!").unwrap();

  let src = format!(
    r#"
  import {{ cmd }} from "rsvim:ext/cmd";
  import {{ fs }} from "rsvim:ext/fs";
  using f = await fs.open({:?});
  const buf1 = new Uint8Array(100);
  const n1 = await f.read(buf1);
  cmd.echo(`n1:${{n1}}`);

  const buf2 = new Uint8Array(100);
  const n2 = await f.read(buf2);
  cmd.echo(`n2:${{n2}}`);
    "#,
    tmpfile.path()
  );
  info!("src:{:?}", src);

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

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
    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "n1:13");
    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "n2:0");
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_read_write2() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let tmpfile = assert_fs::NamedTempFile::new("README.md").unwrap();
  info!("tmpfile:{:?}", tmpfile);
  tmpfile.touch().unwrap();
  tmpfile.write_str("Hello, World!").unwrap();

  let src = format!(
    r#"
  import {{ cmd }} from "rsvim:ext/cmd";
  import {{ fs }} from "rsvim:ext/fs";
  using f = fs.openSync({:?});
  const buf1 = new Uint8Array(0);
  const n1 = f.readSync(buf1);
  cmd.echo(`n1:${{n1}}`);
  const buf2 = new Uint8Array(100);
  const n2 = f.readSync(buf2);
  cmd.echo(`n2:${{n2}}`);
    "#,
    tmpfile.path()
  );
  info!("src:{:?}", src);

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

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
    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "n1:0");
    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "n2:13");
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_read_write3() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let tmpfile = assert_fs::NamedTempFile::new("README.md").unwrap();
  info!("tmpfile:{:?}", tmpfile);

  let src = format!(
    r#"
  import {{ cmd }} from "rsvim:ext/cmd";
  import {{ fs }} from "rsvim:ext/fs";
  using f = await fs.open({:?}, {{create:true,write:true}});
  const buf1 = new TextEncoder().encode("Hello World");
  const n1 = await f.write(buf1);
  cmd.echo(`n1:${{n1}}`);
  const buf2 = new TextEncoder().encode("");
  const n2 = await f.write(buf2);
  cmd.echo(`n2:${{n2}}`);
    "#,
    tmpfile.path()
  );
  info!("src:{:?}", src);

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

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
    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "n1:11");
    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "n2:0");
    assert_eq!(
      std::fs::read_to_string(tmpfile.path()).unwrap(),
      "Hello World".to_string()
    );
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_read_write4() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let tmpfile = assert_fs::NamedTempFile::new("README.md").unwrap();
  info!("tmpfile:{:?}", tmpfile);

  let src = format!(
    r#"
  import {{ cmd }} from "rsvim:ext/cmd";
  import {{ fs }} from "rsvim:ext/fs";
  using f = await fs.open({:?}, {{create:true,write:true}});
  const buf1 = new TextEncoder().encode("Hello World");
  const n1 = f.writeSync(buf1);
  cmd.echo(`n1:${{n1}}`);
  const buf2 = new TextEncoder().encode("");
  const n2 = f.writeSync(buf2);
  cmd.echo(`n2:${{n2}}`);
    "#,
    tmpfile.path()
  );
  info!("src:{:?}", src);

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

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
    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "n1:11");
    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "n2:0");
    assert_eq!(
      std::fs::read_to_string(tmpfile.path()).unwrap(),
      "Hello World".to_string()
    );
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_read_file1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let tmpfile = assert_fs::NamedTempFile::new("README.md").unwrap();
  info!("tmpfile:{:?}", tmpfile);
  tmpfile.touch().unwrap();
  tmpfile.write_str("Hello, World!").unwrap();

  let src = format!(
    r#"
  import {{ cmd }} from "rsvim:ext/cmd";
  import {{ fs }} from "rsvim:ext/fs";
  const data = await fs.readFile({:?});
  cmd.echo(data);
  cmd.echo(data.byteLength);
    "#,
    tmpfile.path()
  );
  info!("src:{:?}", src);

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

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

    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "[object ArrayBuffer]");

    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, format!("{}", "Hello, World!".len()));
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_read_file2() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let tmpfile = assert_fs::NamedTempFile::new("README.md").unwrap();
  info!("tmpfile:{:?}", tmpfile);
  tmpfile.touch().unwrap();
  tmpfile.write_str("Hello, World!").unwrap();

  let src = format!(
    r#"
  import {{ cmd }} from "rsvim:ext/cmd";
  import {{ fs }} from "rsvim:ext/fs";
  const data = fs.readFileSync({:?});
  cmd.echo(data);
  cmd.echo(data.byteLength);
    "#,
    tmpfile.path()
  );
  info!("src:{:?}", src);

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

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

    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "[object ArrayBuffer]");

    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, format!("{}", "Hello, World!".len()));
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_read_text_file1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let tmpfile = assert_fs::NamedTempFile::new("README.md").unwrap();
  info!("tmpfile:{:?}", tmpfile);
  tmpfile.touch().unwrap();
  tmpfile.write_str("Hello, World!").unwrap();

  let src = format!(
    r#"
  import {{ cmd }} from "rsvim:ext/cmd";
  import {{ fs }} from "rsvim:ext/fs";
  const data = await fs.readTextFile({:?});
  cmd.echo(data);
    "#,
    tmpfile.path()
  );
  info!("src:{:?}", src);

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

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
    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "Hello, World!");
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_read_text_file2() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(50))];
  let tmpfile = assert_fs::NamedTempFile::new("README.md").unwrap();
  info!("tmpfile:{:?}", tmpfile);
  tmpfile.touch().unwrap();
  tmpfile.write_str("Hello, World!").unwrap();

  let src = format!(
    r#"
  import {{ cmd }} from "rsvim:ext/cmd";
  import {{ fs }} from "rsvim:ext/fs";
  const data = fs.readTextFileSync({:?});
  cmd.echo(data);
    "#,
    tmpfile.path()
  );
  info!("src:{:?}", src);

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

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
    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "Hello, World!");
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_fs_stat1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(30000))];
  let tmpfile = assert_fs::NamedTempFile::new("README.md").unwrap();
  info!("tmpfile:{:?}", tmpfile);
  tmpfile.touch().unwrap();
  tmpfile.write_str("Hello, World!").unwrap();

  let src = format!(
    r#"
  import {{ cmd }} from "rsvim:ext/cmd";
  import {{ fs }} from "rsvim:ext/fs";
  import {{ process }} from "rsvim:ext/process";
  const fstat = await fs.lstat({:?});
  cmd.echo(`fstat created:${{fstat.created}}, accessed:${{fstat.accessed}}, modified:${{fstat.modified}}, isDir:${{fstat.isDir}}, isFile:${{fstat.isFile}}, isSymlink:${{fstat.isSymlink}}, len:${{fstat.len}}, readOnly:${{fstat.readOnly}}`);
  process.exit();
    "#,
    tmpfile.path()
  );
  info!("src:{:?}", src);

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

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
    let actual = contents.message_history_mut().pop().unwrap();

    let re = Regex::new(r"^fstat created:([^,]+), accessed:([^,]+), modified:([^,]+), isDir:(true|false), isFile:(true|false), isSymlink:(true|false), len:([0-9]+), readOnly:(true|false)$").unwrap();
    assert!(re.is_match(&actual));
  }

  Ok(())
}

#[tokio::test]
#[cfg(target_family = "windows")]
#[cfg_attr(miri, ignore)]
async fn test_fs_stat2() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(30000))];
  let tmpfile = assert_fs::NamedTempFile::new("README.md").unwrap();
  info!("tmpfile:{:?}", tmpfile);
  tmpfile.touch().unwrap();
  tmpfile.write_str("Hello, World!").unwrap();

  let src = format!(
    r#"
  import {{ cmd }} from "rsvim:ext/cmd";
  import {{ fs }} from "rsvim:ext/fs";
  import {{ process }} from "rsvim:ext/process";
  const fstat = await fs.lstat({:?});
  cmd.echo(`fstat fileAttributes:${{fstat.fileAttributes}}, creationTime:${{fstat.creationTime}}, lastAccessTime:${{fstat.lastAccessTime}}, lastWriteTime:${{fstat.lastWriteTime}}, fileSize:${{fstat.fileSize}}`);
  cmd.echo(`fstat dev:${{fstat.dev}}`);
  process.exit();
    "#,
    tmpfile.path()
  );
  info!("src:{:?}", src);

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

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

    let actual = contents.message_history_mut().pop().unwrap();
    let re = Regex::new(r"^fstat fileAttributes:([0-9]+), creationTime:([0-9]+), lastAccessTime:([0-9]+), lastWriteTime:([0-9]+), fileSize:([0-9]+)$").unwrap();
    assert!(re.is_match(&actual));

    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "fstat dev:undefined");
  }

  Ok(())
}

#[tokio::test]
#[cfg(target_family = "unix")]
#[cfg_attr(miri, ignore)]
async fn test_fs_stat3() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(30000))];
  let tmpfile = assert_fs::NamedTempFile::new("README.md").unwrap();
  info!("tmpfile:{:?}", tmpfile);
  tmpfile.touch().unwrap();
  tmpfile.write_str("Hello, World!").unwrap();

  let src = format!(
    r#"
  import {{ cmd }} from "rsvim:ext/cmd";
  import {{ fs }} from "rsvim:ext/fs";
  import {{ process }} from "rsvim:ext/process";
  const fstat = await fs.lstat({:?});
  cmd.echo(`fstat dev:${{fstat.dev}}, ino:${{fstat.ino}}, ino:${{fstat.ino}}, mode:${{fstat.mode}}, nlink:${{fstat.nlink}}, uid:${{fstat.uid}}, gid:${{fstat.gid}}, rdev:${{fstat.rdev}}, size:${{fstat.size}}, atime:${{fstat.atime}}, atimeNsec:${{fstat.atimeNsec}}, mtime:${{fstat.mtime}}, mtimeNsec:${{fstat.mtimeNsec}}, ctime:${{fstat.ctime}}, ctimeNsec:${{fstat.ctimeNsec}}, blksize:${{fstat.blksize}}, blocks:${{fstat.blocks}}`);
  cmd.echo(`fstat fileAttributes:${{fstat.fileAttributes}}`);
  process.exit();
    "#,
    tmpfile.path()
  );
  info!("src:{:?}", src);

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

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

    let actual = contents.message_history_mut().pop().unwrap();
    let re = Regex::new(r"^fstat dev:([0-9]+), ino:([0-9]+), ino:([0-9]+), mode:([0-9]+), nlink:([0-9]+), uid:([0-9]+), gid:([0-9]+), rdev:([0-9]+), size:([0-9]+), atime:([0-9]+), atimeNsec:([0-9]+), mtime:([0-9]+), mtimeNsec:([0-9]+), ctime:([0-9]+), ctimeNsec:([0-9]+), blksize:([0-9]+), blocks:([0-9]+)$").unwrap();
    assert!(re.is_match(&actual));

    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "fstat fileAttributes:undefined");
  }

  Ok(())
}

#[tokio::test]
#[cfg(target_family = "unix")]
#[cfg_attr(miri, ignore)]
async fn test_fs_symlink1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(30000))];
  let tmp = assert_fs::TempDir::new().unwrap();
  let original = tmp.child("hello.txt");
  original.touch().unwrap();
  original.write_str("Hello, World!").unwrap();
  let link1 = tmp.child("link1.txt");
  let link2 = tmp.child("link2.txt");

  let src = format!(
    r#"
  import {{ cmd }} from "rsvim:ext/cmd";
  import {{ fs }} from "rsvim:ext/fs";
  import {{ process }} from "rsvim:ext/process";
  await fs.symlink({:?}, {:?});
  const payload1 = await fs.readTextFile({:?});
  cmd.echo(payload1);

  fs.symlinkSync({:?}, {:?});
  const payload2 = await fs.readTextFile({:?});
  cmd.echo(payload2);
  process.exit();
    "#,
    original.path(),
    link1.path(),
    link1.path(),
    original.path(),
    link2.path(),
    link2.path(),
  );
  info!("src:{:?}", src);

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

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

    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "Hello, World!");

    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "Hello, World!");
  }

  Ok(())
}

#[tokio::test]
#[cfg(target_family = "unix")]
#[cfg_attr(miri, ignore)]
async fn test_fs_symlink2() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(30000))];
  let tmp = assert_fs::TempDir::new().unwrap();
  let original = tmp.child("hello.txt");
  original.touch().unwrap();
  original.write_str("Hello, World!").unwrap();
  let link1 = tmp.child("link1.txt");
  let link2 = tmp.child("link2.txt");

  let src = format!(
    r#"
  import {{ cmd }} from "rsvim:ext/cmd";
  import {{ fs }} from "rsvim:ext/fs";
  import {{ process }} from "rsvim:ext/process";
  await fs.symlink({:?}, {:?}, "file");
  const payload1 = await fs.readTextFile({:?});
  cmd.echo(payload1);

  fs.symlinkSync({:?}, {:?}, "file");
  const payload2 = await fs.readTextFile({:?});
  cmd.echo(payload2);
  process.exit();
    "#,
    original.path(),
    link1.path(),
    link1.path(),
    original.path(),
    link2.path(),
    link2.path(),
  );
  info!("src:{:?}", src);

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

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

    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "Hello, World!");

    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "Hello, World!");
  }

  Ok(())
}

#[tokio::test]
#[cfg(target_family = "windows")]
#[cfg_attr(miri, ignore)]
async fn test_fs_symlink3() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(30000))];
  let tmp = assert_fs::TempDir::new().unwrap();
  let original = tmp.child("hello.txt");
  original.touch().unwrap();
  original.write_str("Hello, World!").unwrap();
  let link1 = tmp.child("link1.txt");
  let link2 = tmp.child("link2.txt");

  let src = format!(
    r#"
  import {{ fs }} from "rsvim:ext/fs";
  import {{ process }} from "rsvim:ext/process";
  await fs.symlink({:?}, {:?});
  fs.symlinkSync({:?}, {:?});
  process.exit();
    "#,
    original.path(),
    link1.path(),
    original.path(),
    link2.path(),
  );
  info!("src:{:?}", src);

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  event_loop.initialize()?;
  event_loop
    .run_with_mock_events(MockEventReader::new(mocked_events))
    .await?;
  event_loop.shutdown()?;

  // After running
  {
    let contents = lock!(event_loop.cmdline_text);
    let n = contents.message_history().len();
    assert_eq!(n, 0);

    // let actual = contents.message_history_mut().pop().unwrap();
    // assert_eq!(actual, "Hello, World!");
    //
    // let actual = contents.message_history_mut().pop().unwrap();
    // assert_eq!(actual, "Hello, World!");
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_fs_link1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(30000))];
  let tmp = assert_fs::TempDir::new().unwrap();
  let original = tmp.child("hello.txt");
  original.touch().unwrap();
  original.write_str("Hello, World!").unwrap();
  let link1 = tmp.child("link1.txt");
  let link2 = tmp.child("link2.txt");

  let src = format!(
    r#"
  import {{ cmd }} from "rsvim:ext/cmd";
  import {{ fs }} from "rsvim:ext/fs";
  import {{ process }} from "rsvim:ext/process";
  await fs.link({:?}, {:?});
  const payload1 = await fs.readTextFile({:?});
  cmd.echo(payload1);

  fs.linkSync({:?}, {:?});
  const payload2 = await fs.readTextFile({:?});
  cmd.echo(payload2);
  process.exit();
    "#,
    original.path(),
    link1.path(),
    link1.path(),
    original.path(),
    link2.path(),
    link2.path(),
  );
  info!("src:{:?}", src);

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

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

    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "Hello, World!");

    let actual = contents.message_history_mut().pop().unwrap();
    assert_eq!(actual, "Hello, World!");
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_fs_mkdir1() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(30000))];
  let tmp = assert_fs::TempDir::new().unwrap();
  let target1 = tmp.child("hello");
  let target2 = tmp.child("world");

  let src = format!(
    r#"
  import {{ fs }} from "rsvim:ext/fs";
  import {{ process }} from "rsvim:ext/process";
  await fs.mkdir({:?});
  fs.mkdirSync({:?});
  process.exit();
    "#,
    target1.path(),
    target2.path(),
  );
  info!("src:{:?}", src);

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  event_loop.initialize()?;
  event_loop
    .run_with_mock_events(MockEventReader::new(mocked_events))
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
async fn test_fs_mkdir2() -> IoResult<()> {
  test_log_init();

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(30000))];
  let tmp = assert_fs::TempDir::new().unwrap();
  let target1 = tmp.child("hello").child("world");
  let target2 = tmp.child("good").child("bye");

  let src = format!(
    r#"
  import {{ fs }} from "rsvim:ext/fs";
  import {{ process }} from "rsvim:ext/process";
  await fs.mkdir({:?}, {{recursive: true, mode: 0o766}});
  fs.mkdirSync({:?}, {{recursive: true, mode: 0o766}});
  process.exit();
    "#,
    target1.path(),
    target2.path(),
  );
  info!("src:{:?}", src);

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

  let mut event_loop =
    make_event_loop(terminal_cols, terminal_rows, CliOptions::empty());

  event_loop.initialize()?;
  event_loop
    .run_with_mock_events(MockEventReader::new(mocked_events))
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
async fn test_read_dir0() -> IoResult<()> {
  test_log_init();

  let target = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../tests_and_benchmarks/tree-sitter-python"
  );
  info!("target:{:?}", target);

  // case-1: pure rust implementation
  {
    match std::fs::read_dir(target) {
      Ok(rd) => {
        // for-in automatically handles `None` returned from `rd`
        for it in rd {
          match it {
            Ok(entry) => {
              info!("case-1 next: {:?}", entry);
            }
            Err(e) => {
              info!("case-1 next failed: {:?}", e);
            }
          }
        }
      }
      Err(e) => {
        info!("case-1 read_dir failed: {:?}", e);
      }
    }
  }

  // case-2: resource table implementation
  {
    match std::fs::read_dir(target) {
      Ok(rd) => {
        let rd_obj = std::sync::Arc::new(std::sync::Mutex::new(rd));
        loop {
          let mut rd_obj2 = rd_obj.lock().unwrap();
          let it = rd_obj2.next();
          match it {
            Some(it2) => match it2 {
              Ok(entry) => {
                info!("case-2 next: {:?}", entry);
              }
              Err(e) => {
                info!("case-2 next failed: {:?}", e);
              }
            },
            None => {
              info!("case-2 next: None");
              break;
            }
          }
        }
      }
      Err(e) => {
        info!("case-2 read_dir failed: {:?}", e);
      }
    }
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_read_dir1() -> IoResult<()> {
  test_log_init();

  let target = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../tests_and_benchmarks/tree-sitter-python"
  );
  info!("target:{:?}", target);

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(100))];

  let src = format!(
    r###"
  import {{ cmd }} from "rsvim:ext/cmd";
  import {{ fs }} from "rsvim:ext/fs";
  for (const entry of fs.readDirSync({:?})) {{
    cmd.echo(`name:${{entry.name}},file:${{entry.isFile}},dir:${{entry.isDir}},symlink:${{entry.isSymlink}}`);
  }}
"###,
    target
  );

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

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
    assert_eq!(n, 28);

    let msg_re = Regex::new(r"^name:.*,file:(?:true|false),dir:(?:true|false),symlink:(?:true|false)$").unwrap();
    for i in 0..n {
      let msg = contents.message_history_mut().pop();
      assert!(msg.is_some());
      let msg = msg.unwrap();
      info!("{}:{:?}", i, msg);
      assert!(msg_re.is_match(&msg));
    }
  }

  Ok(())
}

#[tokio::test]
#[cfg_attr(miri, ignore)]
async fn test_read_dir2() -> IoResult<()> {
  test_log_init();

  let target = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../tests_and_benchmarks/tree-sitter-python"
  );
  info!("target:{:?}", target);

  let terminal_cols = 10_u16;
  let terminal_rows = 10_u16;
  let mocked_events = vec![MockEvent::SleepFor(Duration::from_millis(1000))];

  let src = format!(
    r###"
  import {{ cmd }} from "rsvim:ext/cmd";
  import {{ fs }} from "rsvim:ext/fs";
  for await (const entry of fs.readDir({:?})) {{
    cmd.echo(`name:${{entry.name}},file:${{entry.isFile}},dir:${{entry.isDir}},symlink:${{entry.isSymlink}}`);
  }}
"###,
    target
  );

  // Prepare $RSVIM_CONFIG/rsvim.js
  let _tp = make_configs(vec![(Path::new("rsvim.js"), &src)]);

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
    assert_eq!(n, 28);

    let msg_re = Regex::new(r"^name:.*,file:(?:true|false),dir:(?:true|false),symlink:(?:true|false)$").unwrap();
    for i in 0..n {
      let msg = contents.message_history_mut().pop();
      assert!(msg.is_some());
      let msg = msg.unwrap();
      info!("{}:{:?}", i, msg);
      assert!(msg_re.is_match(&msg));
    }
  }

  Ok(())
}
