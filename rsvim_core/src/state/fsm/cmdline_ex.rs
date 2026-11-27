//! The command-line ex mode.

use crate::chan;
use crate::chan::ExCommandReq;
use crate::chan::JsMessage;
use crate::prelude::*;
use crate::state::State;
use crate::state::StateContext;
use crate::state::Stateful;
use crate::state::ops::CursorInsertPayload;
use crate::state::ops::Operation;
use crate::state::ops::cmdline_ops;
use crate::state::ops::cursor_ops;
use crate::ui::canvas::CursorStyle;
use crate::ui::tree::*;
use crate::ui::widget::cmdline::indicator::CmdlineIndicatorSymbol;
use compact_str::CompactString;
use compact_str::ToCompactString;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEventKind;

#[derive(Debug, Copy, Clone, Default, PartialEq, Eq, Hash)]
/// The command-line ex mode.
pub struct CmdlineEx {}

impl CmdlineEx {
  fn get_operation(&self, event: &Event) -> Option<Operation> {
    match event {
      Event::FocusGained => None,
      Event::FocusLost => None,
      Event::Key(key_event) => match key_event.kind {
        KeyEventKind::Press => {
          trace!("key_event:{key_event:?}");
          match key_event.code {
            // KeyCode::Up | KeyCode::Char('k') => Some(Operation::CursorMoveUpBy(1)),
            // KeyCode::Down | KeyCode::Char('j') => Some(Operation::CursorMoveDownBy(1)),
            KeyCode::Left => Some(Operation::CursorMoveLeftBy(1)),
            KeyCode::Right => Some(Operation::CursorMoveRightBy(1)),
            KeyCode::Home => Some(Operation::CursorMoveLeftBy(usize::MAX)),
            KeyCode::End => Some(Operation::CursorMoveRightBy(usize::MAX)),
            KeyCode::Char(c) => Some(Operation::CursorInsert(
              CursorInsertPayload::Text(c.to_compact_string()),
            )),
            KeyCode::Tab => {
              Some(Operation::CursorInsert(CursorInsertPayload::Tab))
            }
            KeyCode::Backspace => Some(Operation::CursorDelete(-1)),
            KeyCode::Delete => Some(Operation::CursorDelete(1)),
            KeyCode::Esc => Some(Operation::GotoNormalMode),
            KeyCode::Enter => {
              Some(Operation::ConfirmExCommandAndGotoNormalMode)
            }
            _ => None,
          }
        }
        KeyEventKind::Repeat => None,
        KeyEventKind::Release => None,
      },
      Event::Mouse(_mouse_event) => None,
      Event::Paste(_paste_string) => None,
      Event::Resize(_columns, _rows) => None,
    }
  }
}

impl Stateful for CmdlineEx {
  fn handle(&self, context: &StateContext, event: Event) -> State {
    if let Some(op) = self.get_operation(&event) {
      return self.handle_op(context, op);
    }

    State::CmdlineEx(CmdlineEx::default())
  }

  fn handle_op(&self, context: &StateContext, op: Operation) -> State {
    match op {
      Operation::CursorMoveBy((_, _))
      | Operation::CursorMoveUpBy(_)
      | Operation::CursorMoveDownBy(_)
      | Operation::CursorMoveLeftBy(_)
      | Operation::CursorMoveRightBy(_)
      | Operation::CursorMoveTo((_, _)) => self.cursor_move(context, op),
      Operation::GotoNormalMode => self.goto_normal_mode(context),
      Operation::ConfirmExCommandAndGotoNormalMode => {
        self.confirm_ex_command_and_goto_normal_mode(context)
      }
      Operation::CursorInsert(text) => self.cursor_insert(context, text),
      Operation::CursorDelete(n) => self.cursor_delete(context, n),
      _ => unreachable!(),
    }
  }
}

impl CmdlineEx {
  pub fn confirm_ex_command_and_goto_normal_mode(
    &self,
    context: &StateContext,
  ) -> State {
    let cmdline_input_content = self._goto_normal_mode_impl(context);

    let tree = context.tree.clone();
    let tree = lock!(tree);
    let current_window = tree.current_window().unwrap();
    let current_win_id = current_window.id();
    let buffer = current_window.buffer().upgrade().unwrap();
    let buffer = lock!(buffer);
    let current_buf_id = buffer.id();

    chan::send_to_jsrt(
      context.jsrt_tx.clone(),
      JsMessage::ExCommandReq(ExCommandReq {
        payload: cmdline_input_content,
        current_buf_id,
        current_win_id,
      }),
    );

    State::Normal(super::Normal::default())
  }
}

impl CmdlineEx {
  pub fn _goto_normal_mode_impl(
    &self,
    context: &StateContext,
  ) -> CompactString {
    let tree = context.tree.clone();
    let mut tree = lock!(tree);

    if cfg!(debug_assertions) {
      debug_assert!(tree.cmdline_id().is_some());
      debug_assert!(tree.current_window_id().is_some());
      let cursor_id = tree.cursor_id();
      debug_assert!(cursor_id.is_some());
      let cursor_id = cursor_id.unwrap();
      let cursor_parent_id = tree.parent_id(cursor_id);
      debug_assert!(cursor_parent_id.is_some());
      let cursor_parent_id = cursor_parent_id.unwrap();
      let cursor_parent = tree.node(cursor_parent_id);
      debug_assert!(cursor_parent.is_some());
      let cursor_parent = cursor_parent.unwrap();
      debug_assert!(matches!(cursor_parent, Node::CmdlineInput(_)));
    }

    // Show message/hide input, and update layouts/shapes.
    tree.cmdline_show_message().unwrap();

    let current_window_id = tree.current_window_id().unwrap();
    let _old_widget_id = cursor_ops::cursor_jump(&mut tree, current_window_id);

    if cfg!(debug_assertions) {
      debug_assert_eq!(_old_widget_id, tree.cmdline_id());
      let cursor_id = tree.cursor_id();
      debug_assert!(cursor_id.is_some());
      let cursor_id = cursor_id.unwrap();
      let cursor_parent_id = tree.parent_id(cursor_id);
      debug_assert!(cursor_parent_id.is_some());
      let cursor_parent_id = cursor_parent_id.unwrap();
      let cursor_parent = tree.node(cursor_parent_id);
      debug_assert!(cursor_parent.is_some());
      let cursor_parent = cursor_parent.unwrap();
      debug_assert!(matches!(cursor_parent, Node::WindowContent(_)));
    }

    let cursor_viewport = tree.editable_cursor_viewport(current_window_id);
    tree
      .cursor_move_position_to(
        cursor_viewport.column_idx() as isize,
        cursor_viewport.row_idx() as isize,
      )
      .unwrap();
    tree
      .cursor_mut()
      .unwrap()
      .set_cursor_style(CursorStyle::SteadyBlock);
    tree.set_cmdline_indicator_symbol(CmdlineIndicatorSymbol::Empty);

    // Clear command-line both input content and message.
    let contents = context.cmdline_text.clone();
    let mut contents = lock!(contents);
    let cmdline_input_content = contents.input().rope().to_compact_string();

    cmdline_ops::cmdline_clear_message(&mut tree, &mut contents);
    cmdline_ops::cmdline_clear_input(&mut tree, &mut contents);

    cmdline_input_content.trim().to_compact_string()
  }

  pub fn goto_normal_mode(&self, context: &StateContext) -> State {
    self._goto_normal_mode_impl(context);

    State::Normal(super::Normal::default())
  }
}

impl CmdlineEx {
  pub fn cursor_move(&self, context: &StateContext, op: Operation) -> State {
    let tree = context.tree.clone();
    let mut tree = lock!(tree);
    debug_assert!(tree.cmdline_id().is_some());
    let cmdline_id = tree.cmdline_id().unwrap();
    let contents = context.cmdline_text.clone();
    let contents = lock!(contents);

    cursor_ops::cursor_move(&mut tree, cmdline_id, contents.input(), op, true);

    State::CmdlineEx(CmdlineEx::default())
  }
}

impl CmdlineEx {
  pub fn cursor_insert(
    &self,
    context: &StateContext,
    payload: CursorInsertPayload,
  ) -> State {
    let tree = context.tree.clone();
    let mut tree = lock!(tree);
    debug_assert!(tree.cmdline_id().is_some());
    let cmdline_id = tree.cmdline_id().unwrap();
    let contents = context.cmdline_text.clone();
    let mut contents = lock!(contents);

    let payload = match payload {
      CursorInsertPayload::Text(c) => c,
      CursorInsertPayload::Tab => '\t'.to_compact_string(),
      _ => unreachable!(),
    };

    cursor_ops::cursor_insert(
      &mut tree,
      cmdline_id,
      contents.input_mut(),
      payload,
    );

    State::CmdlineEx(CmdlineEx::default())
  }
}

impl CmdlineEx {
  pub fn cursor_delete(&self, context: &StateContext, n: isize) -> State {
    let tree = context.tree.clone();
    let mut tree = lock!(tree);
    let contents = context.cmdline_text.clone();
    let mut contents = lock!(contents);
    let text = contents.input_mut();

    let cmdline_id = tree.cmdline_id().unwrap();
    debug_assert_eq!(
      tree.cmdline().unwrap().input_cursor_viewport().line_idx(),
      0
    );
    debug_assert!(
      text
        .rope()
        .get_line(tree.cmdline().unwrap().input_cursor_viewport().line_idx())
        .is_some()
    );

    cursor_ops::cursor_delete(&mut tree, cmdline_id, text, n);

    State::CmdlineEx(CmdlineEx::default())
  }
}
