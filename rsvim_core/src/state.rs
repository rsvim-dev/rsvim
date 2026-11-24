//! The finite-state machine for Vim editing mode.
//!
//! Vim's [editing mode](https://en.wikipedia.org/wiki/Vim_(text_editor)) is a global state,
//! i.e the editor starts with normal mode, then press `i` to insert mode, or press `SHIFT-V` to
//! visual mode. In insert mode, press `ESC` to back normal mode. And or so.
//!
//! Each editing mode handles user keyboard/mouse inputs in a different way, this a finite-state
//! machine (FSM) separates code logic in different modes. Each editing mode is a FSM state.
//!
//! Besides, there're some other internal states which are not editing modes or visible to
//! user, but help maintaining the internal state of the editor:

pub mod fsm;
pub mod mode;
pub mod ops;

#[cfg(test)]
mod mode_tests;

use crate::buf::BufferManagerArc;
use crate::chan::JsMessage;
use crate::chan::MasterMessage;
use crate::cmdltext::CmdlineTextArc;
use crate::state::ops::Operation;
use crate::ui::tree::TreeArc;
use crossterm::event::Event;
use fsm::CmdlineEx;
use fsm::CmdlineSearchBackward;
use fsm::CmdlineSearchForward;
use fsm::Insert;
use fsm::Normal;
use fsm::OperatorPending;
use fsm::Select;
use fsm::Terminal;
use fsm::Visual;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug)]
/// The mutable data passed to each state handler, and allow them access the editor.
pub struct StateContext {
  pub tree: TreeArc,
  pub buffer_manager: BufferManagerArc,
  pub cmdline_text: CmdlineTextArc,
  pub master_tx: UnboundedSender<MasterMessage>,
  pub jsrt_tx: UnboundedSender<JsMessage>,
}

impl StateContext {
  pub fn new(
    tree: TreeArc,
    buffer_manager: BufferManagerArc,
    cmdline_text: CmdlineTextArc,
    master_tx: UnboundedSender<MasterMessage>,
    jsrt_tx: UnboundedSender<JsMessage>,
  ) -> Self {
    StateContext {
      tree,
      buffer_manager,
      cmdline_text,
      master_tx,
      jsrt_tx,
    }
  }
}

/// The FSM trait.
pub trait Stateful {
  /// Handle user's keyboard/mouse event, this method can access the editor's data and update UI tree.
  ///
  /// Returns next state.
  fn handle(&self, context: &StateContext, event: Event) -> State;

  /// Handle user's operation, this method can access the editor's data and update UI tree.
  ///
  /// Returns next state.
  fn handle_op(&self, context: &StateContext, op: Operation) -> State;
}

#[derive(
  Debug, Copy, Clone, PartialEq, Eq, Hash, rsvim_macro::StatefulEnum,
)]
/// The value holder for each state machine.
pub enum State {
  Normal(Normal),
  Visual(Visual),
  Select(Select),
  OperatorPending(OperatorPending),
  Insert(Insert),
  CmdlineEx(CmdlineEx),
  CmdlineSearchForward(CmdlineSearchForward),
  CmdlineSearchBackward(CmdlineSearchBackward),
  Terminal(Terminal),
}

impl Default for State {
  /// Returns the default FMS state, by default it's the [`Normal`] editing
  /// mode.
  fn default() -> Self {
    State::Normal(Normal::default())
  }
}
