use crate::buf::BufferArc;
use crate::buf::BufferManagerArc;
use crate::buf::opt::BufferOptions;
use crate::buf::opt::BufferOptionsBuilder;
use crate::cmdltext::CmdlineText;
use crate::cmdltext::CmdlineTextArc;
use crate::prelude::*;
use crate::state::StateContext;
use crate::tests::buf::make_buffer_from_lines;
use crate::tests::buf::make_buffers_manager;
use crate::tests::tree::make_tree_with_buffers;
use crate::tests::tree::make_tree_with_buffers_cmdline;
use crate::ui::tree::*;
use crate::ui::widget::window::opt::WindowOptions;
use crossterm::event::Event;
use crossterm::event::KeyCode;
use crossterm::event::KeyEvent;
use crossterm::event::KeyEventKind;
use crossterm::event::KeyModifiers;
use tokio::sync::mpsc::unbounded_channel;

pub fn make_fsm(
  terminal_size: U16Size,
  buffer_local_opts: BufferOptions,
  window_local_opts: WindowOptions,
  lines: Vec<&str>,
) -> (
  Event,
  TreeArc,
  BufferManagerArc,
  BufferArc,
  CmdlineTextArc,
  StateContext,
) {
  let buf = make_buffer_from_lines(terminal_size, buffer_local_opts, lines);
  let bufs = make_buffers_manager(buffer_local_opts, vec![buf.clone()]);
  let tree =
    make_tree_with_buffers(terminal_size, window_local_opts, bufs.clone());
  let cmdline_text = CmdlineText::to_arc(CmdlineText::new(terminal_size));

  let key_event = KeyEvent::new_with_kind(
    KeyCode::Char('a'),
    KeyModifiers::empty(),
    KeyEventKind::Press,
  );
  let event = Event::Key(key_event);

  let (jsrt_tx, _jsrt_rx) = unbounded_channel();
  let (master_tx, _master_rx) = unbounded_channel();
  let context = StateContext::new(
    tree.clone(),
    bufs.clone(),
    cmdline_text.clone(),
    master_tx,
    jsrt_tx,
  );

  (event, tree, bufs, buf, cmdline_text, context)
}

pub fn make_fsm_default_bufopts(
  terminal_size: U16Size,
  window_local_opts: WindowOptions,
  lines: Vec<&str>,
) -> (
  Event,
  TreeArc,
  BufferManagerArc,
  BufferArc,
  CmdlineTextArc,
  StateContext,
) {
  let buf_opts = BufferOptionsBuilder::default().build().unwrap();
  make_fsm(terminal_size, buf_opts, window_local_opts, lines)
}

pub fn make_fsm_with_cmdline(
  terminal_size: U16Size,
  buffer_local_opts: BufferOptions,
  window_local_opts: WindowOptions,
  lines: Vec<&str>,
) -> (
  Event,
  TreeArc,
  BufferManagerArc,
  BufferArc,
  CmdlineTextArc,
  StateContext,
) {
  let buf = make_buffer_from_lines(terminal_size, buffer_local_opts, lines);
  let bufs = make_buffers_manager(buffer_local_opts, vec![buf.clone()]);
  let cmdline_text = CmdlineText::to_arc(CmdlineText::new(terminal_size));
  let tree = make_tree_with_buffers_cmdline(
    terminal_size,
    window_local_opts,
    bufs.clone(),
    cmdline_text.clone(),
  );

  let key_event = KeyEvent::new_with_kind(
    KeyCode::Char('a'),
    KeyModifiers::empty(),
    KeyEventKind::Press,
  );
  let event = Event::Key(key_event);
  let (jsrt_tx, _jsrt_rx) = unbounded_channel();
  let (master_tx, _master_rx) = unbounded_channel();
  let context = StateContext::new(
    tree.clone(),
    bufs.clone(),
    cmdline_text.clone(),
    master_tx,
    jsrt_tx,
  );

  (event, tree, bufs, buf, cmdline_text, context)
}

pub fn make_fsm_with_cmdline_default_bufopts(
  terminal_size: U16Size,
  window_local_opts: WindowOptions,
  lines: Vec<&str>,
) -> (
  Event,
  TreeArc,
  BufferManagerArc,
  BufferArc,
  CmdlineTextArc,
  StateContext,
) {
  let buf_opts = BufferOptionsBuilder::default().build().unwrap();
  make_fsm_with_cmdline(terminal_size, buf_opts, window_local_opts, lines)
}
