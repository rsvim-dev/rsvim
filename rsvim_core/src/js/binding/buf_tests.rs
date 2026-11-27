use crate::cli::CliOptions;
use crate::prelude::*;
use crate::state::ops::CursorInsertPayload;
use crate::state::ops::GotoInsertModeVariant;
use crate::state::ops::Operation;
use crate::tests::evloop::*;
use crate::tests::js::with_kernel;
use crate::tests::log::init as test_log_init;
use compact_str::ToCompactString;
use regex::Regex;
use std::time::Duration;

#[cfg(test)]
mod tests_current1 {
  use super::*;

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn undefined1() -> IoResult<()> {
    test_log_init();

    let terminal_cols = 10_u16;
    let terminal_rows = 10_u16;
    let mocked_ops = vec![MockOperation::SleepFor(Duration::from_millis(30))];

    let src: &str = r#"
    import buf from "rsvim:buf";
    const bufId = buf.current();
    if (bufId !== undefined) {
        throw new Error("Current buffer ID is not undefined!");
    }
    const bufs = buf.list();
    if (!Array.isArray(bufs)) {
        throw new Error("Buffers is not an array!");
    }
    if (bufs.length > 0) {
        throw new Error("Buffers list is not empty!");
    }
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
      let contents = lock!(event_loop.editor.cmdline_text);
      let payload = contents.message().rope().to_string();
      info!("After payload:{payload:?}");
      let payload = payload.trim();
      assert!(payload.is_empty());
    }

    Ok(())
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn valid1() -> IoResult<()> {
    test_log_init();

    let terminal_cols = 10_u16;
    let terminal_rows = 10_u16;
    let mocked_ops = vec![MockOperation::SleepFor(Duration::from_millis(30))];

    let src: &str = r#"
  import buf from "rsvim:buf";
  setTimeout(() => {
    const buf1 = buf.current();
    if (buf1 === null || buf1 === undefined) {
      throw new Error("Current buffer ID1 is null or undefined!");
    }
    if (typeof buf1 !== "number") {
      throw new Error(`Current buffer ID1 ${buf1} (${typeof buf1}) is not a number!`);
    }
    if (buf1 <= 0) {
      throw new Error(`Current buffer ID1 ${buf1} (${typeof buf1}) <= 0`);
    }
    const bufs = buf.list();
    if (!Array.isArray(bufs)) {
        throw new Error("Buffers is not an array!");
    }
    if (bufs.length !== 1) {
        throw new Error("Buffers list size is not 1!");
    }
    const buf2 = bufs[0];
    if (buf2 === null || buf2 === undefined) {
      throw new Error("Current buffer ID2 is null or undefined!");
    }
    if (typeof buf2 !== "number") {
      throw new Error(`Current buffer ID2 ${buf2} (${typeof buf2}) is not a number!`);
    }
    if (buf2 != buf1) {
      throw new Error(`Current buffer ID2 ${buf2} (${typeof buf2}) != ID1 ${buf1} (${typeof buf1})`);
    }
  }, 1);
      "#;

    // Prepare $RSVIM_CONFIG/rsvim.js
    let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

    let mut event_loop = make_event_loop(
      terminal_cols,
      terminal_rows,
      CliOptions::new(false, vec![Path::new("README.md").to_path_buf()]),
    );

    event_loop.initialize()?;
    event_loop
      .run_with_mock_operations(MockOperationReader::new(mocked_ops))
      .await?;
    event_loop.shutdown()?;

    // After running
    {
      let contents = lock!(event_loop.editor.cmdline_text);
      let payload = contents.message().rope().to_string();
      info!("After payload:{payload:?}");
      let payload = payload.trim();
      assert!(payload.is_empty());
    }

    Ok(())
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn write_sync1() -> IoResult<()> {
    test_log_init();

    let terminal_cols = 10_u16;
    let terminal_rows = 10_u16;

    let f1 = assert_fs::NamedTempFile::new("write_sync1.txt").unwrap();

    let src: &str = r#"
  import buf from "rsvim:buf";
  import cmd from "rsvim:cmd";
  setTimeout(() => {
    const buf1 = buf.current();
    if (typeof buf1 !== "number" || buf1 <= 0) {
      throw new Error(`Current buffer ID ${buf1} (${typeof buf1}) is invalid!`);
    }
    try {
      const n = buf.writeSync(buf1);
      cmd.echo(`Buffer ${buf1} has been saved, ${n} bytes written`);
    } catch (e) {
      cmd.echo(`Failed to save buffer ${buf1}, exception: ${e}`);
    }
  }, 500);
      "#;

    // Prepare $RSVIM_CONFIG/rsvim.js
    let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

    let mocked_ops = vec![
      MockOperation::Operation(Operation::GotoInsertMode(
        GotoInsertModeVariant::Keep,
      )),
      MockOperation::Operation(Operation::CursorInsert(
        CursorInsertPayload::Text("Hello RSVIM!".to_compact_string()),
      )),
      MockOperation::Operation(Operation::GotoNormalMode),
      MockOperation::SleepFor(Duration::from_millis(1000)),
    ];

    // Open editor 1st time, f1 not exists, the `writeSync` will create new
    // file and write.
    {
      let mut event_loop = make_event_loop(
        terminal_cols,
        terminal_rows,
        CliOptions::new(false, vec![f1.to_path_buf()]),
      );

      event_loop.initialize()?;
      event_loop
        .run_with_mock_operations(MockOperationReader::new(mocked_ops.clone()))
        .await?;
      event_loop.shutdown()?;

      // After running
      let contents = lock!(event_loop.editor.cmdline_text);
      let payload = contents.message().rope().to_string();
      info!("After payload-1:{payload:?}");
      let payload = payload.trim();
      let expect =
        Regex::new(r"Buffer [0-9]+ has been saved, [0-9]+ bytes written")
          .unwrap();
      assert!(expect.is_match(payload) || payload.is_empty());

      let actual = std::fs::read_to_string(f1.path()).unwrap();
      info!("f1-1:{actual:?}");
      assert!(actual.match_indices("Hello RSVIM!").count() == 1);
    }

    // Open editor 2nd time, f1 already exists, the `writeSync` will overwrite
    // exist text contents.
    {
      let mut event_loop = make_event_loop(
        terminal_cols,
        terminal_rows,
        CliOptions::new(false, vec![f1.to_path_buf()]),
      );

      event_loop.initialize()?;
      event_loop
        .run_with_mock_operations(MockOperationReader::new(mocked_ops))
        .await?;
      event_loop.shutdown()?;

      // After running
      let contents = lock!(event_loop.editor.cmdline_text);
      let payload = contents.message().rope().to_string();
      info!("After payload-2:{payload:?}");
      let payload = payload.trim();
      let expect =
        Regex::new(r"Buffer [0-9]+ has been saved, [0-9]+ bytes written")
          .unwrap();
      assert!(expect.is_match(payload) || payload.is_empty());

      let actual = std::fs::read_to_string(f1.path()).unwrap();
      info!("f1-2:{actual:?}");
      assert!(actual.match_indices("Hello RSVIM!").count() == 2);
    }

    f1.close().unwrap();

    Ok(())
  }
}

#[test]
#[cfg_attr(miri, ignore)]
fn empty_editor_has_no_current_buffer() {
  test_log_init();
  with_kernel(|ctx, _kernel| {
    // `make_editor` builds no window, so both calls report emptiness rather
    // than raising. The js wrapper turns this into `undefined` / `[]`.
    let current: Option<i32> = ctx
      .eval("__InternalRsvimGlobalObject.buf_current()")
      .unwrap();
    assert_eq!(current, None);

    let list: Vec<i32> =
      ctx.eval("__InternalRsvimGlobalObject.buf_list()").unwrap();
    assert!(list.is_empty());
  });
}

#[test]
#[cfg_attr(miri, ignore)]
fn write_sync_on_a_missing_buffer_is_a_javascript_error() {
  test_log_init();
  with_kernel(|ctx, _kernel| {
    // Buffer ids start at 1, so 99 cannot exist in a fresh editor.
    let result =
      ctx.eval::<i32, _>("__InternalRsvimGlobalObject.buf_write_sync(99)");
    assert!(result.is_err());
  });
}
