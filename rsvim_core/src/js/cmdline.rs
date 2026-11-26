//! Engine-neutral command line plumbing.
//!
//! `cmd.echo` is the same operation no matter which javascript engine called
//! it: put the payload where the user will see it. The v8 backend and the
//! quickjs backend both end up here, so the function lives outside either of
//! them.

use crate::evloop::Editor;
use crate::prelude::*;
use crate::state::ops::cmdline_ops;

/// Puts `payload` in front of the user.
///
/// With the TUI up that means the command line widget; without it (tests, and
/// the first moments of startup, before the widgets exist) the message history
/// is the only place a user could ever read it, so it goes there.
pub fn send_cmdline_message(editor: &Editor, payload: String) {
  trace!("|cmd| send_cmdline_message:{:?}", payload);
  let mut tree = lock!(editor.tree);
  let mut cmdline_text = lock!(editor.cmdline_text);
  if tree.cmdline_id().is_some() {
    cmdline_ops::cmdline_set_message(&mut tree, &mut cmdline_text, payload);
  } else {
    // If `command_line` widget does not exist, it means the TUI is not
    // initialized yet. All we can do is store the message in the history; the
    // event loop flushes the history into the TUI once the widgets exist.
    cmdline_text.message_history_mut().push_overwrite(payload);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::tests::js::make_editor;

  #[test]
  #[cfg_attr(miri, ignore)]
  fn message_lands_in_history_without_a_tui() {
    crate::tests::log::init();
    let editor = make_editor();
    send_cmdline_message(&editor, "hello".to_string());
    let history = lock!(editor.cmdline_text)
      .message_history()
      .iter()
      .cloned()
      .collect::<Vec<String>>();
    assert_eq!(history, vec!["hello".to_string()]);
  }
}
