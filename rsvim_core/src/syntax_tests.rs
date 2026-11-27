use super::syntax::*;
use crate::buf::opt::EndOfLineOption;
use crate::cli::CliOptions;
use crate::evloop::writer::StdoutWriterValue;
use crate::prelude::*;
use crate::state::ops as state_ops;
use crate::tests::evloop::*;
use crate::tests::log::init as test_log_init;
use crate::ui::canvas::ShaderCommand;
use assert_fs::NamedTempFile;
use assert_fs::prelude::FileTouch;
use assert_fs::prelude::FileWriteStr;
use assert_fs::prelude::PathChild;
use compact_str::ToCompactString;
use crossterm::style::Color;
use itertools::Itertools;
use std::time::Duration;

#[cfg(test)]
mod tests_syntax_manager {
  use super::*;

  #[test]
  #[cfg_attr(miri, ignore)]
  fn getter1() {
    test_log_init();
    let syntax_manager = SyntaxManager::new();

    let inputs = [
      (
        "c",
        vec!["c", "h"],
        Some("queries/highlights.scm"),
        Some("queries/tags.scm"),
        None,
      ),
      (
        "rust",
        vec!["rs"],
        Some("queries/highlights.scm"),
        Some("queries/tags.scm"),
        Some("queries/injections.scm"),
      ),
      (
        "markdown",
        vec!["md", "mdown"],
        Some("queries/highlights.scm"),
        None,
        Some("queries/injections.scm"),
      ),
      (
        "toml",
        vec!["toml"],
        Some("queries/highlights.scm"),
        None,
        None,
      ),
      (
        "html",
        vec!["html", "htm"],
        Some("queries/highlights.scm"),
        None,
        None,
      ),
    ];

    for input in inputs.iter() {
      let name = input.0;
      let expect_file_types =
        input.1.iter().map(|i| i.to_compact_string()).collect_vec();
      let expect_highlights_path = input.2;
      let expect_tags_path = input.3;
      let expect_injections_path = input.4;

      let actual_grammar = syntax_manager.get_grammar(name);
      assert!(actual_grammar.is_some());
      let actual_file_types =
        syntax_manager.get_file_types_by_grammar_name(name);
      assert_eq!(
        actual_file_types.map(|ft| ft.iter().cloned().collect::<FoldSet<_>>()),
        Some(expect_file_types.iter().cloned().collect::<FoldSet<_>>())
      );
      let actual_highlights_query = syntax_manager.get_highlights_query(name);
      assert_eq!(
        actual_highlights_query.is_some(),
        expect_highlights_path.is_some()
      );
      let actual_tags_query = syntax_manager.get_tags_query(name);
      assert_eq!(actual_tags_query.is_some(), expect_tags_path.is_some());
      let actual_injections_query = syntax_manager.get_injections_query(name);
      assert_eq!(
        actual_injections_query.is_some(),
        expect_injections_path.is_some()
      );
    }
  }
}

#[cfg(test)]
mod tests_buffer_editing {
  use super::*;

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn rust_err1() -> IoResult<()> {
    test_log_init();

    let src: &str = r#""#;

    // Prepare $RSVIM_CONFIG/rsvim.js
    let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

    let terminal_cols = 10_u16;
    let terminal_rows = 10_u16;
    let mocked_ops = vec![
      MockOperation::Operation(state_ops::Operation::GotoInsertMode(
        state_ops::GotoInsertModeVariant::Keep,
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text("Hello".to_compact_string()),
      )),
      MockOperation::SleepFor(Duration::from_millis(100)),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text(", ".to_compact_string()),
      )),
      MockOperation::SleepFor(Duration::from_millis(100)),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text("World".to_compact_string()),
      )),
      MockOperation::Operation(state_ops::Operation::GotoNormalMode),
      MockOperation::SleepFor(Duration::from_millis(500)),
    ];

    let mut event_loop = make_event_loop(
      terminal_cols,
      terminal_rows,
      CliOptions::new(false, vec![Path::new("err1.rs").to_path_buf()]),
    );

    event_loop.initialize()?;
    event_loop
      .run_with_mock_operations(MockOperationReader::new(mocked_ops))
      .await?;
    event_loop.shutdown()?;

    // After running
    {
      let buf = lock!(event_loop.editor.buffer_manager)
        .first_key_value()
        .unwrap()
        .1
        .clone();
      let buf = lock!(buf);
      let buf_eol = Into::<EndOfLineOption>::into(buf.options().file_format());
      let payload = buf.text().rope().to_string();
      assert_eq!(format!("Hello, World{}", buf_eol), payload);
      let buf_editing_version = buf.editing_version();
      let syn_editing_version =
        buf.syntax().as_ref().unwrap().editing_version();
      assert_eq!(buf_editing_version, syn_editing_version);
      let syn_tree = buf.syntax().as_ref().unwrap().treesitter_tree();
      assert!(syn_tree.as_ref().is_some());
      info!(
        "syn tree:{:?}",
        syn_tree.as_ref().unwrap().root_node().to_string()
      );
      assert_eq!(
        syn_tree.as_ref().unwrap().root_node().to_string(),
        "(source_file (ERROR (identifier) (identifier)))"
      );
    }

    Ok(())
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn rust_err2() -> IoResult<()> {
    test_log_init();

    let src: &str = r#""#;

    // Prepare $RSVIM_CONFIG/rsvim.js
    let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

    let terminal_cols = 10_u16;
    let terminal_rows = 10_u16;
    let mocked_ops = vec![
      MockOperation::Operation(state_ops::Operation::GotoInsertMode(
        state_ops::GotoInsertModeVariant::Keep,
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text("Hello".to_compact_string()),
      )),
      MockOperation::Operation(state_ops::Operation::GotoNormalMode),
      MockOperation::SleepFor(Duration::from_millis(100)),
      MockOperation::Operation(state_ops::Operation::GotoInsertMode(
        state_ops::GotoInsertModeVariant::Append,
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text(", ".to_compact_string()),
      )),
      MockOperation::Operation(state_ops::Operation::GotoNormalMode),
      MockOperation::SleepFor(Duration::from_millis(100)),
      MockOperation::Operation(state_ops::Operation::GotoInsertMode(
        state_ops::GotoInsertModeVariant::Append,
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text("World".to_compact_string()),
      )),
      MockOperation::Operation(state_ops::Operation::GotoNormalMode),
      MockOperation::SleepFor(Duration::from_millis(100)),
      // Hello, World
      MockOperation::Operation(state_ops::Operation::CursorMoveTo((7, 0))),
      MockOperation::Operation(state_ops::Operation::GotoInsertMode(
        state_ops::GotoInsertModeVariant::Keep,
      )),
      MockOperation::Operation(state_ops::Operation::CursorDelete(-2)),
      MockOperation::Operation(state_ops::Operation::GotoNormalMode),
      MockOperation::SleepFor(Duration::from_millis(500)),
    ];

    let mut event_loop = make_event_loop(
      terminal_cols,
      terminal_rows,
      CliOptions::new(false, vec![Path::new("err2.rs").to_path_buf()]),
    );

    event_loop.initialize()?;
    event_loop
      .run_with_mock_operations(MockOperationReader::new(mocked_ops))
      .await?;
    event_loop.shutdown()?;

    // After running
    {
      let buf = lock!(event_loop.editor.buffer_manager)
        .first_key_value()
        .unwrap()
        .1
        .clone();
      let buf = lock!(buf);
      let buf_eol = Into::<EndOfLineOption>::into(buf.options().file_format());
      let payload = buf.text().rope().to_string();
      assert_eq!(format!("HelloWorld{}", buf_eol), payload);
      let buf_editing_version = buf.editing_version();
      let syn_editing_version =
        buf.syntax().as_ref().unwrap().editing_version();
      assert_eq!(buf_editing_version, syn_editing_version);
      let syn_tree = buf.syntax().as_ref().unwrap().treesitter_tree();
      assert!(syn_tree.as_ref().is_some());
      info!(
        "syn tree:{:?}",
        syn_tree.as_ref().unwrap().root_node().to_string()
      );
      assert_eq!(
        syn_tree.as_ref().unwrap().root_node().to_string(),
        "(source_file (ERROR (identifier)))"
      );
    }

    Ok(())
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn rust_ok1() -> IoResult<()> {
    test_log_init();

    let src: &str = r#""#;

    // Prepare $RSVIM_CONFIG/rsvim.js
    let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

    let terminal_cols = 10_u16;
    let terminal_rows = 10_u16;
    let mocked_ops = vec![
      MockOperation::Operation(state_ops::Operation::GotoInsertMode(
        state_ops::GotoInsertModeVariant::Keep,
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text("use ".to_compact_string()),
      )),
      MockOperation::Operation(state_ops::Operation::GotoNormalMode),
      MockOperation::SleepFor(Duration::from_millis(100)),
      MockOperation::Operation(state_ops::Operation::GotoInsertMode(
        state_ops::GotoInsertModeVariant::Append,
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text("std::sy".to_compact_string()),
      )),
      MockOperation::Operation(state_ops::Operation::GotoNormalMode),
      MockOperation::SleepFor(Duration::from_millis(100)),
      MockOperation::Operation(state_ops::Operation::GotoInsertMode(
        state_ops::GotoInsertModeVariant::Append,
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text("nc::Arc;".to_compact_string()),
      )),
      MockOperation::Operation(state_ops::Operation::GotoNormalMode),
      MockOperation::SleepFor(Duration::from_millis(1000)),
    ];

    let mut event_loop = make_event_loop(
      terminal_cols,
      terminal_rows,
      CliOptions::new(false, vec![Path::new("ok1.rs").to_path_buf()]),
    );

    event_loop.initialize()?;
    event_loop
      .run_with_mock_operations(MockOperationReader::new(mocked_ops))
      .await?;
    event_loop.shutdown()?;

    // After running
    {
      let buf = lock!(event_loop.editor.buffer_manager)
        .first_key_value()
        .unwrap()
        .1
        .clone();
      let buf = lock!(buf);
      let buf_eol = Into::<EndOfLineOption>::into(buf.options().file_format());
      let payload = buf.text().rope().to_string();
      assert_eq!(format!("use std::sync::Arc;{}", buf_eol), payload);
      let buf_editing_version = buf.editing_version();
      let syn_editing_version =
        buf.syntax().as_ref().unwrap().editing_version();
      assert_eq!(buf_editing_version, syn_editing_version);
      let syn_tree = buf.syntax().as_ref().unwrap().treesitter_tree();
      assert!(syn_tree.as_ref().is_some());
      info!(
        "syn tree:{:?}",
        syn_tree.as_ref().unwrap().root_node().to_string()
      );
      assert_eq!(
        syn_tree.as_ref().unwrap().root_node().to_string(),
        "(source_file (use_declaration argument: (scoped_identifier path: (scoped_identifier path: (identifier) name: (identifier)) name: (identifier))))"
      );
    }

    Ok(())
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn rust_ok2() -> IoResult<()> {
    test_log_init();

    let src: &str = r#""#;

    // Prepare $RSVIM_CONFIG/rsvim.js
    let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

    let terminal_cols = 10_u16;
    let terminal_rows = 10_u16;
    let mocked_ops = vec![
      MockOperation::Operation(state_ops::Operation::GotoInsertMode(
        state_ops::GotoInsertModeVariant::Keep,
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text("use ".to_compact_string()),
      )),
      MockOperation::Operation(state_ops::Operation::GotoNormalMode),
      MockOperation::SleepFor(Duration::from_millis(100)),
      MockOperation::Operation(state_ops::Operation::GotoInsertMode(
        state_ops::GotoInsertModeVariant::Append,
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text("std::sy".to_compact_string()),
      )),
      MockOperation::Operation(state_ops::Operation::GotoNormalMode),
      MockOperation::SleepFor(Duration::from_millis(100)),
      MockOperation::Operation(state_ops::Operation::GotoInsertMode(
        state_ops::GotoInsertModeVariant::Append,
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text("nc::Arc;".to_compact_string()),
      )),
      MockOperation::Operation(state_ops::Operation::GotoNormalMode),
      MockOperation::SleepFor(Duration::from_millis(100)),
      MockOperation::Operation(state_ops::Operation::GotoInsertMode(
        state_ops::GotoInsertModeVariant::NewLine,
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text("f".to_compact_string()),
      )),
      MockOperation::SleepFor(Duration::from_millis(100)),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text("n mai".to_compact_string()),
      )),
      MockOperation::SleepFor(Duration::from_millis(100)),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text("n() {".to_compact_string()),
      )),
      MockOperation::Operation(state_ops::Operation::GotoNormalMode),
      MockOperation::SleepFor(Duration::from_millis(100)),
      MockOperation::Operation(state_ops::Operation::GotoInsertMode(
        state_ops::GotoInsertModeVariant::NewLine,
      )),
      MockOperation::SleepFor(Duration::from_millis(100)),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text("  println".to_compact_string()),
      )),
      MockOperation::SleepFor(Duration::from_millis(100)),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text("!(\"".to_compact_string()),
      )),
      MockOperation::Operation(state_ops::Operation::GotoNormalMode),
      MockOperation::SleepFor(Duration::from_millis(100)),
      MockOperation::Operation(state_ops::Operation::GotoInsertMode(
        state_ops::GotoInsertModeVariant::Append,
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text(
          "Hello, World".to_compact_string(),
        ),
      )),
      MockOperation::SleepFor(Duration::from_millis(100)),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text("\"".to_compact_string()),
      )),
      MockOperation::SleepFor(Duration::from_millis(100)),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text(");".to_compact_string()),
      )),
      MockOperation::Operation(state_ops::Operation::GotoNormalMode),
      MockOperation::SleepFor(Duration::from_millis(100)),
      MockOperation::Operation(state_ops::Operation::GotoInsertMode(
        state_ops::GotoInsertModeVariant::Append,
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Eol,
      )),
      MockOperation::SleepFor(Duration::from_millis(100)),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text("}".to_compact_string()),
      )),
      MockOperation::Operation(state_ops::Operation::GotoNormalMode),
      MockOperation::SleepFor(Duration::from_millis(1000)),
    ];

    let mut event_loop = make_event_loop(
      terminal_cols,
      terminal_rows,
      CliOptions::new(false, vec![Path::new("ok2.rs").to_path_buf()]),
    );

    event_loop.initialize()?;
    event_loop
      .run_with_mock_operations(MockOperationReader::new(mocked_ops))
      .await?;
    event_loop.shutdown()?;

    // After running
    {
      let buf = lock!(event_loop.editor.buffer_manager)
        .first_key_value()
        .unwrap()
        .1
        .clone();
      let buf = lock!(buf);
      let buf_eol = Into::<EndOfLineOption>::into(buf.options().file_format());
      let payload = buf.text().rope().to_string();
      assert_eq!(
        format!(
          "use std::sync::Arc;{buf_eol}fn main() {{{buf_eol}  println!(\"Hello, World\");{buf_eol}}}{buf_eol}"
        ),
        payload
      );
      let buf_editing_version = buf.editing_version();
      let syn_editing_version =
        buf.syntax().as_ref().unwrap().editing_version();
      assert_eq!(buf_editing_version, syn_editing_version);
      let syn_tree = buf.syntax().as_ref().unwrap().treesitter_tree();
      assert!(syn_tree.as_ref().is_some());
      info!(
        "syn tree:{:?}",
        syn_tree.as_ref().unwrap().root_node().to_string()
      );
      assert_eq!(
        syn_tree.as_ref().unwrap().root_node().to_string(),
        "(source_file (use_declaration argument: (scoped_identifier path: (scoped_identifier path: (identifier) name: (identifier)) name: (identifier))) (function_item name: (identifier) parameters: (parameters) body: (block (expression_statement (macro_invocation macro: (identifier) (token_tree (string_literal (string_content))))))))"
      );
    }

    Ok(())
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn rust_ok3() -> IoResult<()> {
    test_log_init();

    let src: &str = r#""#;

    // Prepare $RSVIM_CONFIG/rsvim.js
    let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

    let terminal_cols = 10_u16;
    let terminal_rows = 10_u16;
    let mocked_ops = vec![
      MockOperation::Operation(state_ops::Operation::GotoInsertMode(
        state_ops::GotoInsertModeVariant::Keep,
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text(
          "use std::sync::Arc;".to_compact_string(),
        ),
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Eol,
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text("fn main() {".to_compact_string()),
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Eol,
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text(
          "  println!(\"你好，世界！\");".to_compact_string(),
        ),
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Eol,
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text("}".to_compact_string()),
      )),
      MockOperation::Operation(state_ops::Operation::GotoNormalMode),
      MockOperation::SleepFor(Duration::from_millis(1000)),
    ];

    let mut event_loop = make_event_loop(
      terminal_cols,
      terminal_rows,
      CliOptions::new(false, vec![Path::new("ok3.rs").to_path_buf()]),
    );

    event_loop.initialize()?;
    event_loop
      .run_with_mock_operations(MockOperationReader::new(mocked_ops))
      .await?;
    event_loop.shutdown()?;

    // After running
    {
      let buf = lock!(event_loop.editor.buffer_manager)
        .first_key_value()
        .unwrap()
        .1
        .clone();
      let buf = lock!(buf);
      let buf_eol = Into::<EndOfLineOption>::into(buf.options().file_format());
      let payload = buf.text().rope().to_string();
      assert_eq!(
        format!(
          "use std::sync::Arc;{buf_eol}fn main() {{{buf_eol}  println!(\"你好，世界！\");{buf_eol}}}{buf_eol}"
        ),
        payload
      );
      let buf_editing_version = buf.editing_version();
      let syn_editing_version =
        buf.syntax().as_ref().unwrap().editing_version();
      assert_eq!(buf_editing_version, syn_editing_version);
      let syn_tree = buf.syntax().as_ref().unwrap().treesitter_tree();
      assert!(syn_tree.as_ref().is_some());
      info!(
        "syn tree:{:?}",
        syn_tree.as_ref().unwrap().root_node().to_string()
      );
      assert_eq!(
        syn_tree.as_ref().unwrap().root_node().to_string(),
        "(source_file (use_declaration argument: (scoped_identifier path: (scoped_identifier path: (identifier) name: (identifier)) name: (identifier))) (function_item name: (identifier) parameters: (parameters) body: (block (expression_statement (macro_invocation macro: (identifier) (token_tree (string_literal (string_content))))))))"
      );
    }

    Ok(())
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn rust_err3() -> IoResult<()> {
    test_log_init();

    let src: &str = r#""#;

    // Prepare $RSVIM_CONFIG/rsvim.js
    let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

    let terminal_cols = 10_u16;
    let terminal_rows = 10_u16;
    let mocked_ops = vec![
      MockOperation::Operation(state_ops::Operation::GotoInsertMode(
        state_ops::GotoInsertModeVariant::Keep,
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text(
          "use std::sync::Arc;".to_compact_string(),
        ),
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Eol,
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text("fn main() {".to_compact_string()),
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Eol,
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text(
          "  println!(\"Hello, World!\"".to_compact_string(),
        ),
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Eol,
      )),
      MockOperation::Operation(state_ops::Operation::CursorInsert(
        state_ops::CursorInsertPayload::Text("}".to_compact_string()),
      )),
      MockOperation::Operation(state_ops::Operation::GotoNormalMode),
      MockOperation::SleepFor(Duration::from_millis(1000)),
    ];

    let mut event_loop = make_event_loop(
      terminal_cols,
      terminal_rows,
      CliOptions::new(false, vec![Path::new("err3.rs").to_path_buf()]),
    );

    event_loop.initialize()?;
    event_loop
      .run_with_mock_operations(MockOperationReader::new(mocked_ops))
      .await?;
    event_loop.shutdown()?;

    // After running
    {
      let buf = lock!(event_loop.editor.buffer_manager)
        .first_key_value()
        .unwrap()
        .1
        .clone();
      let buf = lock!(buf);
      let buf_eol = Into::<EndOfLineOption>::into(buf.options().file_format());
      let payload = buf.text().rope().to_string();
      assert_eq!(
        format!(
          "use std::sync::Arc;{buf_eol}fn main() {{{buf_eol}  println!(\"Hello, World!\"{buf_eol}}}{buf_eol}"
        ),
        payload
      );
      let buf_editing_version = buf.editing_version();
      let syn_editing_version =
        buf.syntax().as_ref().unwrap().editing_version();
      assert_eq!(buf_editing_version, syn_editing_version);
      let syn_tree = buf.syntax().as_ref().unwrap().treesitter_tree();
      assert!(syn_tree.as_ref().is_some());
      info!(
        "syn tree:{:?}",
        syn_tree.as_ref().unwrap().root_node().to_string()
      );
      assert_eq!(
        syn_tree.as_ref().unwrap().root_node().to_string(),
        "(source_file (use_declaration argument: (scoped_identifier path: (scoped_identifier path: (identifier) name: (identifier)) name: (identifier))) (function_item name: (identifier) parameters: (parameters) body: (block (macro_invocation macro: (identifier) (token_tree (string_literal (string_content)) (MISSING \")\"))))))"
      );
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests_buffer_scrolling {
  use super::*;

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn rust1() -> IoResult<()> {
    test_log_init();

    let src: &str = r#""#;

    // Prepare $RSVIM_CONFIG/rsvim.js
    let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

    let terminal_cols = 30_u16;
    let terminal_rows = 20_u16;
    let mocked_ops = vec![
      MockOperation::SleepFor(Duration::from_millis(1000)),
      MockOperation::Operation(state_ops::Operation::CursorMoveTo((0, 11))),
      MockOperation::SleepFor(Duration::from_millis(1000)),
    ];

    let tmpfile = NamedTempFile::new("rust1.rs").unwrap();
    tmpfile.touch().unwrap();
    tmpfile
      .write_str(
    r###"use git2::Repository;
use rsvim_core::js::JsRuntimeForSnapshot;
use rsvim_core::js::quickjs_version;
use std::path::Path;

// pub const LOG: &str = "[RSVIM]";
pub const LOG: &str = "cargo:warning=[RSVIM]";

fn version() {
  let profile_env = std::env::var("PROFILE").unwrap_or("debug".to_string());
  let opt_level_env = std::env::var("OPT_LEVEL").unwrap_or("0".to_string());
  let debug_env = std::env::var("DEBUG").unwrap_or("0".to_string());
  let host = std::env::var("HOST").unwrap_or("unknown".to_string());
  println!(
    "{LOG} Env profile:{:?}, opt_level:{:?}, debug:{:?}, host:{:?}",
    profile_env, opt_level_env, debug_env, host
  );

  let workspace_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
  let version = env!("CARGO_PKG_VERSION").to_string();

  // profile
  let is_release_profile = profile_env == "release"
    && (opt_level_env == "s" || opt_level_env == "z")
    && debug_env != "true";
  let profile = if is_release_profile {
    "release".to_string()
  } else if profile_env == "release" {
    "nightly".to_string()
  } else {
    profile_env.clone()
  };

  // git commit
  let git_commit = match Repository::open(&workspace_dir) {
    Ok(repo) => {
      let head = repo.head().unwrap();
      let oid = head.target().unwrap();
      let commit = repo.find_commit(oid).unwrap();
      let id = commit.id();
      let id = id.to_string();
      println!("{LOG} Git id:{:?}", id);
      Some(id[0..8].to_string())
    }
    Err(e) => {
      println!("{LOG} Git error:{:?}", e);
      None
    }
  };

  // swc core
  let swc_core = match std::fs::read_to_string(workspace_dir.join("Cargo.toml"))
  {
    Ok(manifest) => match manifest.parse::<toml::Table>() {
      Ok(parsed_manifest) => {
        let deps = &parsed_manifest["workspace"]["dependencies"];
        let core = deps["swc_core"].as_str();
        println!("{LOG} Swc core:{:?}", core);
        Some(core.unwrap().trim_start_matches("=").to_string())
      }
      Err(e) => {
        println!("{LOG} Parse Cargo.toml error:{:?}", e);
        None
      }
    },
    Err(e) => {
      println!("{LOG} Read Cargo.toml error:{:?}", e);
      None
    }
  };
  let quickjs_version = quickjs_version();

  println!(
    "{LOG} Resolved version:{:?}, profile:{:?}, host:{:?}, git_commit:{:?}, quickjs:{:?}, swc_core:{:?}",
    version, profile, host, git_commit, quickjs_version, swc_core
  );

  let mut resolved = format!(
    "version={}\nprofile={}\nhost={}\nquickjs={}\n",
    version, profile, host, quickjs_version
  );
  if let Some(git_commit) = git_commit {
    resolved = format!("{}git_commit={}\n", resolved, git_commit);
  }
  if let Some(swc_core) = swc_core {
    resolved = format!("{}swc_core={}\n", resolved, swc_core);
  }

  let output_path =
    Path::new(env!("CARGO_MANIFEST_DIR")).join("RSVIM_VERSION.TXT");
  println!("{LOG} Writing version into {:?}...", output_path.as_path());

  std::fs::write(output_path.as_path(), resolved.as_bytes()).unwrap();
}

fn snapshot() {
  let js_runtime = JsRuntimeForSnapshot::new();
  let snapshot = js_runtime.create_snapshot();
  let snapshot = Box::from(&snapshot);
  let mut vec = Vec::with_capacity(snapshot.len());
  vec.extend_from_slice(&snapshot);

  let output_path =
    Path::new(env!("CARGO_MANIFEST_DIR")).join("RSVIM_SNAPSHOT.BIN");
  println!("{LOG} Writing snapshot into {:?}...", output_path.as_path());
  std::fs::write(output_path.as_path(), vec.into_boxed_slice()).unwrap();
}

fn main() {
  version();
  snapshot();
}
"###).unwrap();

    let mut event_loop = make_event_loop(
      terminal_cols,
      terminal_rows,
      CliOptions::new(false, vec![tmpfile.path().to_path_buf()]),
    );

    event_loop.initialize()?;
    event_loop
      .run_with_mock_operations(MockOperationReader::new(mocked_ops))
      .await?;
    event_loop.shutdown()?;

    // After running
    {
      let shaders = match event_loop.writer {
        StdoutWriterValue::DevNullWriter(w) => w.shaders().clone(),
        _ => unreachable!(),
      };
      let text_shaders = shaders
        .iter()
        .filter(|cmd| {
          matches!(
            cmd,
            ShaderCommand::StylePrintStyledContentString(_)
              | ShaderCommand::StylePrintString(_)
          )
        })
        .collect_vec();
      for (i, shader_cmd) in text_shaders.iter().enumerate() {
        if let ShaderCommand::StylePrintStyledContentString(content) =
          shader_cmd
        {
          info!(
            "shader [{:>2}]:{:?} ({:?})",
            i,
            content.0.content(),
            content.0.style()
          );
        } else {
          unreachable!();
        }
      }
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests_buffer_viewing {
  use super::*;

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn markdown1() -> IoResult<()> {
    test_log_init();

    let src: &str = r#""#;

    // Prepare $RSVIM_CONFIG/rsvim.js
    let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

    let terminal_cols = 50_u16;
    let terminal_rows = 40_u16;
    let mocked_ops = vec![MockOperation::SleepFor(Duration::from_millis(1000))];

    let tmpfile = NamedTempFile::new("markdown1.md").unwrap();
    tmpfile.touch().unwrap();
    tmpfile
      .write_str(
    r###"<p align="center">
  <img alt="logo.svg" src="https://raw.githubusercontent.com/rsvim/assets/main/logo/RSVIM-logo.svg" />
</p>

<p align="center">
The VIM editor reinvented in Rust+TypeScript.
</p>

<p align="center">
  <a href="https://crates.io/crates/rsvim"><img alt="rsvim" src="https://img.shields.io/crates/v/rsvim" /></a>
  <a href="https://www.npmjs.com/package/@rsvim/types"><img alt="rsvim" src="https://img.shields.io/npm/v/%40rsvim%2Ftypes" /></a>
  <a href="https://docs.rs/rsvim_core/latest/"><img alt="rsvim_core" src="https://img.shields.io/docsrs/rsvim_core?label=docs.rs" /></a>
  <a href="https://github.com/rsvim/rsvim/actions/workflows/release.yml"><img alt="release.yml" src="https://img.shields.io/github/actions/workflow/status/rsvim/rsvim/release.yml" /></a>
  <a href="https://github.com/rsvim/rsvim/actions/workflows/ci.yml"><img alt="ci.yml" src="https://img.shields.io/github/actions/workflow/status/rsvim/rsvim/ci.yml?branch=main&label=ci" /></a>
  <a href="https://app.codecov.io/gh/rsvim/rsvim"><img alt="codecov" src="https://img.shields.io/codecov/c/github/rsvim/rsvim" /></a>
  <a href="https://discord.gg/5KtRUCAByB"><img alt="discord" src="https://img.shields.io/discord/1220171472329379870?logo=discord&style=social&label=discord" /></a>
</p>

> [!CAUTION]
>
> _**This project is still in very early stage of development and not ready for use. Please choose alternatives [Neovim](https://neovim.io/) and [Vim](https://www.vim.org/).**_

## About

RSVIM is an open source terminal based text editor, built from scratch with [Rust](https://www.rust-lang.org/), [Tokio](https://tokio.rs/) and [V8](https://v8.dev/) javascript engine. It strives to be highly extensible by following main features, concepts, philosophy of ([Neo](https://neovim.io/))[Vim](https://www.vim.org/), while also to be:

- A fast editor that fully utilizes all CPU cores and never freezes.
- A powerful TUI engine that provides widgets, event handlers, layouts, etc.
- A consistent JavaScript-based runtime with built-in support for TypeScript.
- An editing service that allows multiple users to access remotely and work together.
- A text processing tool that integrates with shell environment.

## Installation

Please download prebuilt executables in [releases](https://github.com/rsvim/rsvim/releases) page, or build with cargo:

```bash
cargo install --locked rsvim
```

To get latest updates, build with git source on `main` branch:

```bash
cargo install --locked rsvim --git https://github.com/rsvim/rsvim.git --branch main
```

## Get Started

Please check out [Documentation](https://rsvim.github.io/) for more details!

## Contribution

Contributions to RSVIM are always welcomed. A few guidelines that help quickly set up development environment can be found in [DEVELOPMENT.md](https://github.com/rsvim/rsvim/blob/main/DEVELOPMENT.md).

Road map and high-level design can be found in [RFC](https://github.com/rsvim/rfc), please submit your ideas and designs there if they need fairly large efforts.

## Credits

Some source code are studied from following projects for implementing the initial prototype of javascript runtime and [Minimum Common Web Platform API](https://min-common-api.proposal.wintertc.org/):

- **[dune](https://github.com/aalykiot/dune)**: A hobby runtime for JavaScript and TypeScript 🚀.
- **[deno](https://github.com/denoland/deno)**: A modern runtime for JavaScript and TypeScript.

## Supporting the Project

If you like RSVIM, please consider sponsoring it. Your support encourages contributors and maintainers of this project, and other fees or efforts spent on it.

- [GitHub Sponsor](https://github.com/sponsors/rsvim)
- [Open Collective](https://opencollective.com/rsvim)

## License

Licensed under [Vim License](https://github.com/rsvim/rsvim/blob/main/LICENSE.txt).
"###).unwrap();

    let mut event_loop = make_event_loop(
      terminal_cols,
      terminal_rows,
      CliOptions::new(false, vec![tmpfile.path().to_path_buf()]),
    );

    event_loop.initialize()?;
    event_loop
      .run_with_mock_operations(MockOperationReader::new(mocked_ops))
      .await?;
    event_loop.shutdown()?;

    // After running
    {
      let canvas = event_loop.canvas;
      let canvas = lock!(canvas);
      let frame = canvas.frame();
      for (i, c) in frame.get_cells().iter().enumerate() {
        info!("cell[{}]:{:?}", i, c);
        assert_eq!(*c.bg(), Color::Black);
      }
    }

    Ok(())
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn html1() -> IoResult<()> {
    test_log_init();

    let src: &str = r#""#;

    // Prepare $RSVIM_CONFIG/rsvim.js
    let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

    let terminal_cols = 50_u16;
    let terminal_rows = 40_u16;
    let mocked_ops = vec![MockOperation::SleepFor(Duration::from_millis(1000))];

    let tmpfile = NamedTempFile::new("html1.html").unwrap();
    tmpfile.touch().unwrap();
    tmpfile
      .write_str(
    r###"<html>
  <body>
    <h1>Hello, World!</h1>
  </body>

  <p align="center">
    <img
      alt="logo.svg"
      src="https://raw.githubusercontent.com/rsvim/assets/main/logo/RSVIM-logo.svg"
    />
  </p>

  <p align="center">The VIM editor reinvented in Rust+TypeScript.</p>

  <p align="center">
    <a href="https://crates.io/crates/rsvim"
      ><img alt="rsvim" src="https://img.shields.io/crates/v/rsvim"
    /></a>
    <a href="https://www.npmjs.com/package/@rsvim/types"
      ><img alt="rsvim" src="https://img.shields.io/npm/v/%40rsvim%2Ftypes"
    /></a>
    <a href="https://docs.rs/rsvim_core/latest/"
      ><img
        alt="rsvim_core"
        src="https://img.shields.io/docsrs/rsvim_core?label=docs.rs"
    /></a>
    <a href="https://github.com/rsvim/rsvim/actions/workflows/release.yml"
      ><img
        alt="release.yml"
        src="https://img.shields.io/github/actions/workflow/status/rsvim/rsvim/release.yml"
    /></a>
    <a href="https://github.com/rsvim/rsvim/actions/workflows/ci.yml"
      ><img
        alt="ci.yml"
        src="https://img.shields.io/github/actions/workflow/status/rsvim/rsvim/ci.yml?branch=main&label=ci"
    /></a>
    <a href="https://app.codecov.io/gh/rsvim/rsvim"
      ><img
        alt="codecov"
        src="https://img.shields.io/codecov/c/github/rsvim/rsvim"
    /></a>
    <a href="https://discord.gg/5KtRUCAByB"
      ><img
        alt="discord"
        src="https://img.shields.io/discord/1220171472329379870?logo=discord&style=social&label=discord"
    /></a>
  </p>
</html>
"###).unwrap();

    let mut event_loop = make_event_loop(
      terminal_cols,
      terminal_rows,
      CliOptions::new(false, vec![tmpfile.path().to_path_buf()]),
    );

    event_loop.initialize()?;
    event_loop
      .run_with_mock_operations(MockOperationReader::new(mocked_ops))
      .await?;
    event_loop.shutdown()?;

    // After running
    {
      let canvas = event_loop.canvas;
      let canvas = lock!(canvas);
      let frame = canvas.frame();
      for (i, c) in frame.get_cells().iter().enumerate() {
        info!("cell[{}]:{:?}", i, c);
        assert_eq!(*c.bg(), Color::Black);
      }
    }

    Ok(())
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn rust1() -> IoResult<()> {
    test_log_init();

    let src: &str = r#""#;

    // Prepare $RSVIM_CONFIG/rsvim.js
    let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

    let terminal_cols = 50_u16;
    let terminal_rows = 40_u16;
    let mocked_ops = vec![MockOperation::SleepFor(Duration::from_millis(1000))];

    let tmpfile = NamedTempFile::new("rust1.rs").unwrap();
    tmpfile.touch().unwrap();
    tmpfile
      .write_str(
        r###"use std::sync::Arc;
fn main() {
  println!("Hello, World!");
}
"###,
      )
      .unwrap();

    let mut event_loop = make_event_loop(
      terminal_cols,
      terminal_rows,
      CliOptions::new(false, vec![tmpfile.path().to_path_buf()]),
    );

    event_loop.initialize()?;
    event_loop
      .run_with_mock_operations(MockOperationReader::new(mocked_ops))
      .await?;
    event_loop.shutdown()?;

    // After running
    {
      let canvas = event_loop.canvas;
      let canvas = lock!(canvas);
      let frame = canvas.frame();
      for (i, c) in frame.get_cells().iter().enumerate() {
        info!("cell[{}]:{:?}", i, c);
        assert_eq!(*c.bg(), Color::Black);
      }
    }

    Ok(())
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn c1() -> IoResult<()> {
    test_log_init();

    let src: &str = r#""#;

    // Prepare $RSVIM_CONFIG/rsvim.js
    let _tp = make_configs(vec![(Path::new("rsvim.js"), src)]);

    let terminal_cols = 50_u16;
    let terminal_rows = 40_u16;
    let mocked_ops = vec![MockOperation::SleepFor(Duration::from_millis(1000))];

    let tmpfile = NamedTempFile::new("c1.c").unwrap();
    tmpfile.touch().unwrap();
    tmpfile
      .write_str(
        r###"#include <stdio.h>
int main() {
  printf("Hello, World!\n");
  return 0;
}
"###,
      )
      .unwrap();

    let mut event_loop = make_event_loop(
      terminal_cols,
      terminal_rows,
      CliOptions::new(false, vec![tmpfile.path().to_path_buf()]),
    );

    event_loop.initialize()?;
    event_loop
      .run_with_mock_operations(MockOperationReader::new(mocked_ops))
      .await?;
    event_loop.shutdown()?;

    // After running
    {
      let canvas = event_loop.canvas;
      let canvas = lock!(canvas);
      let frame = canvas.frame();
      for (i, c) in frame.get_cells().iter().enumerate() {
        info!("cell[{}]:{:?}", i, c);
        assert_eq!(*c.bg(), Color::Black);
      }
    }

    Ok(())
  }
}

#[cfg(test)]
mod tests_grammar_loader {
  use super::*;

  fn _run_loader(grammar_path: &str, hint: &str) {
    let grammar_path = Path::new(grammar_path);
    let syntax_manager = SyntaxManager::to_arc(SyntaxManager::new());
    let req = SyntaxLoadGrammarRequest {
      grammar_path: grammar_path.to_path_buf(),
    };
    let grammar = load_syntax_grammar(syntax_manager.clone(), &req);
    info!("{}:{:?}", hint, grammar);
    assert!(grammar.is_ok());
    let metainfo = grammar.unwrap();
    info!("{}:{:?}", hint, metainfo);

    let grammar = load_syntax_grammar(syntax_manager.clone(), &req);
    info!("{}:{:?}", hint, grammar);
    assert!(grammar.is_ok());
    let metainfo = grammar.unwrap();
    info!("{}:{:?}", hint, metainfo);
  }

  // This test case always fail.
  // #[test]
  #[cfg_attr(miri, ignore)]
  fn _rust1() {
    test_log_init();

    let grammar_path = concat!(
      env!("CARGO_MANIFEST_DIR"),
      "/../tests_and_benchmarks/tree-sitter-rust"
    );
    _run_loader(grammar_path, "rust1");
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn c1() {
    test_log_init();

    let grammar_path = concat!(
      env!("CARGO_MANIFEST_DIR"),
      "/../tests_and_benchmarks/tree-sitter-c"
    );
    _run_loader(grammar_path, "c1");
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn python1() {
    test_log_init();

    let grammar_path = concat!(
      env!("CARGO_MANIFEST_DIR"),
      "/../tests_and_benchmarks/tree-sitter-python"
    );
    _run_loader(grammar_path, "python1");
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn failed1() {
    test_log_init();

    let grammar_path = assert_fs::TempDir::new().unwrap();

    let syntax_loader = SyntaxLoader::new();
    let opts = SyntaxLoadGrammarRequest {
      grammar_path: grammar_path.to_path_buf(),
    };
    let grammar = syntax_loader.load_grammar(&opts);
    assert!(grammar.is_err());
    if let Err(e) = grammar {
      info!("failed1:{:?}", e)
    }

    let grammar = syntax_loader.load_grammar(&opts);
    assert!(grammar.is_err());
    if let Err(e) = grammar {
      info!("failed1:{:?}", e)
    }
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn failed2() {
    test_log_init();

    let grammar_path = assert_fs::TempDir::new().unwrap();
    let grammar_json_path = grammar_path.child("src").child("grammar.json");
    grammar_json_path.touch().unwrap();
    grammar_json_path.write_str(r###"{
  "$schema": "https://tree-sitter.github.io/tree-sitter/assets/schemas/grammar.schema.json",
  "name": "rust",
  "word": "identifier"
}"###).unwrap();

    let syntax_loader = SyntaxLoader::new();
    let opts = SyntaxLoadGrammarRequest {
      grammar_path: grammar_path.to_path_buf(),
    };
    let grammar = syntax_loader.load_grammar(&opts);
    assert!(grammar.is_err());
    if let Err(e) = grammar {
      info!("failed2:{:?}", e)
    }

    let grammar = syntax_loader.load_grammar(&opts);
    assert!(grammar.is_err());
    if let Err(e) = grammar {
      info!("failed2:{:?}", e)
    }
  }
}
