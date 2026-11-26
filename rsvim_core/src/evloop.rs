//! Event loop.

pub mod ui;
pub mod writer;

use crate::buf::BufferArc;
use crate::buf::BufferId;
use crate::buf::BufferManager;
use crate::buf::BufferManagerArc;
use crate::cfg::path_cfg::PATH_CONFIG;
use crate::chan;
use crate::chan::JsMessage;
use crate::chan::MasterMessage;
use crate::cli::CliOptions;
use crate::cmdltext::CmdlineText;
use crate::cmdltext::CmdlineTextArc;
use crate::hl::ColorSchemeManager;
use crate::hl::ColorSchemeManagerArc;
use crate::js::JsHost;
use crate::js::JsRuntimeOptions;
use crate::js::command::CommandManager;
use crate::js::command::CommandManagerArc;
use crate::js::resource::ResourceTable;
use crate::js::resource::ResourceTableArc;
use crate::js::timers::DueTimer;
use crate::prelude::*;
use crate::state::State;
use crate::state::StateContext;
use crate::state::Stateful;
use crate::state::ops::cmdline_ops;
use crate::syntax;
use crate::syntax::SyntaxEdit;
use crate::syntax::SyntaxEditNew;
use crate::syntax::SyntaxManager;
use crate::syntax::SyntaxManagerArc;
use crate::ui::canvas::Canvas;
use crate::ui::canvas::CanvasArc;
use crate::ui::tree::*;
use crate::ui::widget::WidgetContext;
use crossterm::event::Event;
use crossterm::event::EventStream;
use futures::Stream;
use futures::StreamExt;
use itertools::Itertools;
use std::ops::ControlFlow;
use std::sync::Arc;
use taffy::Style;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::mpsc::unbounded_channel;
use tokio::time::Instant;
use tokio_util::sync::CancellationToken;
use tokio_util::task::TaskTracker;
use writer::StdoutWritable;
use writer::StdoutWriterValue;

#[cfg(test)]
use crate::tests::evloop::MockEventReader;
#[cfg(test)]
use crate::tests::evloop::MockOperation;
#[cfg(test)]
use crate::tests::evloop::MockOperationReader;
#[cfg(test)]
use bitflags::bitflags_match;
#[cfg(test)]
use crossterm::event::KeyCode;
#[cfg(test)]
use crossterm::event::KeyEventKind;
#[cfg(test)]
use crossterm::event::KeyModifiers;

/// The editor's shared data.
///
/// Cloning it is cheap and every clone is the same editor: the fields are
/// `Arc`s, except `cli_opts`, which is plain data. [`Rsvim`] owns the loop,
/// while the javascript bindings can only reach the kernel stored in the
/// context, i.e. [`crate::js::JsKernel`], through
/// [`get_state`](crate::js::get_state); they have no way to reach the outer
/// `Rsvim`. Both hold a clone of this struct.
#[derive(Debug, Clone)]
pub struct Editor {
  /// Command line options.
  pub cli_opts: CliOptions,
  /// Widget tree for UI.
  pub tree: TreeArc,
  /// Vim buffers.
  pub buffer_manager: BufferManagerArc,
  /// Cmdline text.
  pub cmdline_text: CmdlineTextArc,
  /// Syntax and parsers.
  pub syntax_manager: SyntaxManagerArc,
  /// Colorschemes.
  pub colorscheme_manager: ColorSchemeManagerArc,
  /// Vim commands.
  pub command_manager: CommandManagerArc,
  /// Resources.
  pub resource_table: ResourceTableArc,
}

/// The editor's owner: the javascript host, and all the shared editor data in
/// one place.
///
/// This used to be named `EventLoop`; the alias [`EventLoop`] is kept so that
/// the binary and the doc links don't have to change.
///
/// For slow tasks that are suitable to put in the background, this event loop
/// will spawn them in tokio's async tasks and let them sync back data once
/// they are done. The event loop controls all the tasks with
/// [`CancellationToken`] and [`TaskTracker`].
///
/// The javascript engine is not thread-safe and tokio's tasks are, so the two
/// only exchange plain data over channels; see [`crate::js::ops`].
pub struct Rsvim {
  /// Indicates the start time of the process.
  pub startup_moment: Instant,
  /// Specifies the timestamp which the current process began in Unix time.
  pub startup_unix_epoch: u128,

  /// Stdout writer for editor mode TUI.
  pub writer: StdoutWriterValue,

  /// Canvas for UI.
  pub canvas: CanvasArc,

  /// Finite-state machine for editing state.
  pub state: State,

  /// Editor's shared data (buffers, UI tree, managers, resources, cli options).
  ///
  /// The same `Arc`s are also cloned into [`crate::js::JsKernel`], because the
  /// javascript bindings can only reach the kernel.
  pub editor: Editor,

  /// Cancellation token to notify the main loop to exit.
  pub cancellation_token: CancellationToken,
  /// Task tracker for spawned tasks, there are two trackers:
  ///
  /// 1. Cancellable/deteched tracker for those tasks that are safe to cancel.
  /// 2. Block tracker are for dangerous tasks, user will have to wait for them complete before
  ///    exit the editor.
  ///
  /// Most write file operations are spawned with block tracker to ensure they will be safely
  /// complete to avoid damage user data files. While for most reading operations and pure CPU
  /// calculations, they will be cancelled when editor exit.
  pub detached_tracker: TaskTracker,
  pub blocked_tracker: TaskTracker,
  pub exit_code: i32,

  /// The javascript host: the QuickJS runtime and everything around it.
  ///
  /// It is a synchronous object. Nothing in it makes progress on its own: the
  /// loop feeds it the three things that can make javascript run -- an ex
  /// command, the completion of an asynchronous operation, a due timer -- and
  /// every one of them goes through [`JsHost::enter`]. See [`crate::js::host`].
  pub js: JsHost,

  /// Channel-1
  pub master_tx: UnboundedSender<MasterMessage>,
  pub master_rx: UnboundedReceiver<MasterMessage>,

  /// Channel-2, the single javascript channel.
  ///
  /// The sender is cloned into the FSM (`StateContext`) and into the operation
  /// table of the host, so it carries both an `:Ex` command from the editor and
  /// the completion of every asynchronous operation a script started, in the
  /// order they happened.
  pub jsrt_tx: UnboundedSender<JsMessage>,
  pub jsrt_rx: UnboundedReceiver<JsMessage>,

  /// Whether the terminal needs to be repainted. Replaces the old
  /// "redraw after every select iteration" behaviour, which turned a busy
  /// javascript loop into a full repaint per tick.
  pub dirty: bool,
}

/// What [`Rsvim::tick_event_loop`] found ready.
enum Ready {
  /// A request javascript made to the editor.
  Master(MasterMessage),
  /// An ex command, or the completion of an asynchronous operation.
  Js(JsMessage),
  /// A timer that came due.
  Timer(DueTimer),
}

/// At most this many events are handled before the loop goes back to waiting
/// on terminal input.
///
/// The limit is not about javascript -- a run of microtasks is never cut -- but
/// about the *loop*: a source that keeps producing events (a 1ms interval whose
/// callback takes longer than 1ms) must not starve the terminal. What is left
/// over is not lost: it stays in the channels and the timer table, which wake
/// the next wait immediately, so nothing can be stranded.
const SETTLE_MAX_EVENTS: usize = 256;

/// ... and at most this long.
const SETTLE_BUDGET: std::time::Duration = std::time::Duration::from_millis(8);

/// How many terminal events are taken in one go, so that a burst of keys (a
/// paste arrives as one) is applied before the screen is repainted once.
const INPUT_BATCH: usize = 64;

impl Rsvim {
  /// Runs every javascript-side event that is ready *right now*, without
  /// waiting, and reports whether it ran any.
  ///
  /// Three sources can make javascript run, and they are polled in this order:
  ///
  /// 1. requests javascript made to the editor (`MasterMessage`);
  /// 2. ex commands and finished operations (`JsMessage`);
  /// 3. timers that are due.
  ///
  /// Running one may produce more (a command arms a timer, a completion queues
  /// an ex command), so the loop goes around until a pass finds nothing, the
  /// editor is asked to quit, or the budget is spent -- see
  /// `SETTLE_MAX_EVENTS`.
  ///
  /// This is what makes an event that was queued *before* the loop started
  /// (the config script asked to quit, or issued a write) get served before the
  /// loop looks at terminal input at all.
  pub fn tick_event_loop(&mut self) -> bool {
    let started = std::time::Instant::now();
    let mut worked = false;

    for _ in 0..SETTLE_MAX_EVENTS {
      let Some(ready) = self.pop_ready() else {
        break;
      };
      worked = true;
      self.apply(ready);

      if self.cancellation_token.is_cancelled()
        || started.elapsed() >= SETTLE_BUDGET
      {
        break;
      }
    }

    worked
  }

  /// Takes the next event that needs no waiting.
  fn pop_ready(&mut self) -> Option<Ready> {
    if let Ok(request) = self.master_rx.try_recv() {
      return Some(Ready::Master(request));
    }
    if let Ok(message) = self.jsrt_rx.try_recv() {
      return Some(Ready::Js(message));
    }
    self.js.pop_due_timer().map(Ready::Timer)
  }

  /// Handles one event, and notes whether the editor may have changed.
  fn apply(&mut self, ready: Ready) {
    match ready {
      Ready::Master(request) => {
        self.process_master_message(request);
        // A handled request may have changed `exit_code`, or dispatched a
        // background task whose completion redraws later. Marking dirty is the
        // cheap conservative choice: `render_if_dirty` gates it.
        self.dirty = true;
      }
      Ready::Js(message) => self.process_js_message(message),
      Ready::Timer(timer) => self.js.run_timer(timer),
    }

    // Every way into javascript went through `JsHost::enter`, which marks the
    // host dirty: javascript may have changed anything.
    self.dirty |= self.js.take_dirty();
  }

  /// Runs one message that was queued for javascript.
  fn process_js_message(&mut self, message: JsMessage) {
    match message {
      JsMessage::ExCommandReq(req) => {
        trace!("Recv ExCommandReq:{:?}", req.payload);
        let parsed = lock!(self.editor.command_manager).parse(&req);
        let failure = match parsed {
          Some(command) => command.run(&mut self.js).map(TheErr::from),
          None => Some(TheErr::CommandNotFound(req.payload)),
        };
        if let Some(err) = failure {
          crate::js::err::report_js_error(&self.editor, err);
        }
      }
      JsMessage::OpDone(completion) => self.js.complete(completion),
    }
  }

  /// Executes javascript source code as ES module, i.e. ECMA standard.
  pub fn execute_module(&mut self, filename: &str, source: Option<&str>) {
    if let Err(e) = self.js.execute_module(filename, source) {
      crate::js::err::report_js_error(&self.editor, e);
    }
    self.dirty |= self.js.take_dirty();
  }
}

impl std::fmt::Debug for Rsvim {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Rsvim").finish_non_exhaustive()
  }
}

/// The editor's owner, under its former name.
pub type EventLoop = Rsvim;

#[cfg(test)]
pub(crate) fn is_ctrl_d(event: &Option<IoResult<Event>>) -> bool {
  match event {
    Some(Ok(Event::Key(key_event)))
      if key_event.code == KeyCode::Char('d')
        && key_event.kind == KeyEventKind::Press =>
    {
      bitflags_match!(key_event.modifiers, {
        KeyModifiers::CONTROL => true,
        _ => false
      })
    }
    Some(Ok(Event::Key(_key_event))) => false,
    _ => false,
  }
}

impl Rsvim {
  #[allow(clippy::type_complexity)]
  pub fn _internal_new(
    terminal_cols: u16,
    terminal_rows: u16,
    cli_opts: CliOptions,
  ) -> IoResult<(
    /* canvas */ CanvasArc,
    /* state */ State,
    /* editor */ Editor,
    /* cancellation_token */ CancellationToken,
    /* detached_tracker */ TaskTracker,
    /* blocked_tracker */ TaskTracker,
    /* exit_code */ i32,
    (
      /* master_tx */ UnboundedSender<MasterMessage>,
      /* master_rx */ UnboundedReceiver<MasterMessage>,
    ),
    (
      /* jsrt_tx */ UnboundedSender<JsMessage>,
      /* jsrt_rx */ UnboundedReceiver<JsMessage>,
    ),
  )> {
    // Canvas
    let canvas_size = size!(terminal_cols, terminal_rows);
    let canvas = Canvas::new(canvas_size);
    let canvas = Canvas::to_arc(canvas);

    // UI Tree
    let style = Style {
      display: taffy::Display::Grid,
      grid_template_rows: vec![
        taffy::prelude::fr(1_u16),
        taffy::prelude::length(1_u16),
      ],
      size: taffy::Size {
        width: taffy::prelude::length(canvas_size.width()),
        height: taffy::prelude::length(canvas_size.height()),
      },
      ..Default::default()
    };
    let tree = Tree::to_arc(Tree::new(style).unwrap());

    // Buffers
    let syntax_manager = SyntaxManager::to_arc(SyntaxManager::new());
    let colorscheme_manager =
      ColorSchemeManager::to_arc(ColorSchemeManager::new());
    let buffer_manager = BufferManager::new(
      Arc::downgrade(&syntax_manager),
      Arc::downgrade(&colorscheme_manager),
    );
    let cmdline_text = CmdlineText::to_arc(CmdlineText::new(canvas_size));
    let command_manager = CommandManager::to_arc(CommandManager::default());
    let buffer_manager = BufferManager::to_arc(buffer_manager);
    let resource_table = ResourceTable::to_arc(ResourceTable::new());

    // State
    let state = State::default();

    // Javascript's `Promise` and `async`/`await` are built on tokio tasks: a
    // binding starts the work as a task, and the task reports back with plain
    // data, because the javascript engine is not thread-safe and never leaves
    // this thread (see `crate::js::ops`).
    //
    // Two message channels connect them with the event loop:
    //
    // - Channel-1 `master_tx` => `master_rx` on `MasterMessage`: what
    //   javascript asks of the editor (quit, parse a buffer).
    // - Channel-2 `jsrt_tx` => `jsrt_rx` on `JsMessage`: what makes javascript
    //   run later -- an ex command from the state machine, or the completion of
    //   an operation a script started.

    // Channel-1
    let (master_tx, master_rx) = unbounded_channel();
    // Channel-2
    let (jsrt_tx, jsrt_rx) = unbounded_channel();

    let editor = Editor {
      cli_opts,
      tree,
      buffer_manager,
      cmdline_text,
      syntax_manager,
      colorscheme_manager,
      command_manager,
      resource_table,
    };

    Ok((
      canvas,
      state,
      editor,
      CancellationToken::new(),
      TaskTracker::new(),
      TaskTracker::new(),
      0,
      (master_tx, master_rx),
      (jsrt_tx, jsrt_rx),
    ))
  }

  /// Make new event loop.
  pub fn new(
    startup_moment: Instant,
    startup_unix_epoch: u128,
    cli_opts: CliOptions,
  ) -> IoResult<Self> {
    let (cols, rows) = crossterm::terminal::size()?;
    let (
      canvas,
      state,
      editor,
      cancellation_token,
      detached_tracker,
      blocked_tracker,
      exit_code,
      (master_tx, master_rx),
      (jsrt_tx, jsrt_rx),
    ) = Self::_internal_new(cols, rows, cli_opts.clone())?;

    let writer = StdoutWriterValue::editor();

    // Js Runtime
    let js = JsHost::new(
      JsRuntimeOptions::default(),
      master_tx.clone(),
      jsrt_tx.clone(),
      editor.clone(),
      blocked_tracker.clone(),
      detached_tracker.clone(),
    )?;

    Ok(Rsvim {
      startup_moment,
      startup_unix_epoch,
      canvas,
      state,
      editor,
      writer,
      cancellation_token,
      detached_tracker,
      blocked_tracker,
      exit_code,
      js,
      master_tx,
      master_rx,
      jsrt_tx,
      jsrt_rx,
      dirty: false,
    })
  }

  #[cfg(test)]
  /// Make new event loop for testing.
  pub fn mock_new(
    terminal_columns: u16,
    terminal_rows: u16,
    cli_opts: CliOptions,
  ) -> IoResult<Self> {
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;

    let (
      canvas,
      state,
      editor,
      cancellation_token,
      detached_tracker,
      blocked_tracker,
      exit_code,
      (master_tx, master_rx),
      (jsrt_tx, jsrt_rx),
    ) = Self::_internal_new(terminal_columns, terminal_rows, cli_opts.clone())?;

    let startup_moment = Instant::now();
    let startup_unix_epoch = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap()
      .as_millis();
    let writer = StdoutWriterValue::dev_null();

    // Js Runtime
    let js = JsHost::new(
      JsRuntimeOptions::default(),
      master_tx.clone(),
      jsrt_tx.clone(),
      editor.clone(),
      blocked_tracker.clone(),
      detached_tracker.clone(),
    )?;

    Ok(Rsvim {
      startup_moment,
      startup_unix_epoch,
      canvas,
      state,
      editor,
      writer,
      cancellation_token,
      detached_tracker,
      blocked_tracker,
      exit_code,
      js,
      master_tx,
      master_rx,
      jsrt_tx,
      jsrt_rx,
      dirty: false,
    })
  }

  /// Initialize the editor
  pub fn initialize(&mut self) -> IoResult<()> {
    // Initialize user js configs
    self._init_config()?;

    self.writer.init()?;

    // Initialize TUI
    self._init_buffers()?;
    self._init_windows()?;
    self._init_pending_messages();

    // Flush logic UI to terminal, i.e. print UI to stdout
    let context = WidgetContext::new(
      self.editor.syntax_manager.clone(),
      self.editor.colorscheme_manager.clone(),
    );
    lock!(self.editor.tree).draw(self.canvas.clone(), &context);
    self.writer.init_complete(&mut lock!(self.canvas))?;

    Ok(())
  }

  /// Initialize user config file.
  fn _init_config(&mut self) -> IoResult<()> {
    if let Some(config_entry) = PATH_CONFIG.config_entry() {
      // QuickJS resolves the whole import graph inside `Module::evaluate`, and
      // `JsHost::enter` drains the microtask queue, so there is nothing left to
      // "tick" here. What the config queued (an ex command, a request to quit,
      // a write) is served by the first iteration of the loop, before it looks
      // at any terminal input.
      self.execute_module(&config_entry.to_string_lossy(), None);
    }
    Ok(())
  }

  fn _add_pending_syntax_edit(&self, buf: BufferArc) {
    let mut buf = lock!(buf);
    if buf.syntax().is_some() {
      let payload = buf.text().rope().clone();
      let version = buf.editing_version();
      buf
        .syntax_mut()
        .as_mut()
        .unwrap()
        .add_pending_edits(SyntaxEdit::New(SyntaxEditNew { payload, version }));
      chan::send_to_master(
        self.master_tx.clone(),
        MasterMessage::SyntaxEditReq(chan::SyntaxEditReq {
          buffer_id: buf.id(),
        }),
      );
    }
  }

  /// Initialize buffers.
  pub fn _init_buffers(&mut self) -> IoResult<()> {
    let canvas_size = lock!(self.canvas).size();

    // Create default buffer from `FILES` arguments from cli, or with an empty buffer.
    let input_files = &self.editor.cli_opts.file();
    if !input_files.is_empty() {
      for input_file in input_files.iter() {
        let maybe_buf_id = lock!(self.editor.buffer_manager)
          .new_file_buffer(canvas_size, input_file);
        match maybe_buf_id {
          Ok(buf_id) => {
            let buf = lock!(self.editor.buffer_manager)
              .get(&buf_id)
              .unwrap()
              .clone();
            self._add_pending_syntax_edit(buf);
            trace!("Created file buffer {:?}:{:?}", input_file, buf_id);
          }
          Err(e) => {
            // Send error message to command-line
            error!("Failed to create file buffer {:?}:{:?}", input_file, e);

            // Append error message to command line message history, wait for
            // print once TUI initialized.
            let mut cmdline_text = lock!(self.editor.cmdline_text);
            cmdline_text
              .message_history_mut()
              .push_overwrite(e.to_string());
          }
        }
      }
    } else {
      let (buf_id, buf) = {
        let mut buffer_manager = lock!(self.editor.buffer_manager);
        let buf_id = buffer_manager.new_empty_buffer(canvas_size);
        let buf = buffer_manager.get(&buf_id).unwrap().clone();
        (buf_id, buf)
      };
      self._add_pending_syntax_edit(buf);
      trace!("Created empty buffer {:?}", buf_id);
    }

    Ok(())
  }

  /// Initialize windows.
  pub fn _init_windows(&mut self) -> IoResult<()> {
    // Initialize default window, with default buffer.
    let (cursor_blinking, cursor_hidden, cursor_style) = {
      let canvas = lock!(self.canvas);
      let canvas_cursor = canvas.frame().cursor();
      (
        canvas_cursor.blinking(),
        canvas_cursor.hidden(),
        canvas_cursor.style(),
      )
    };
    let mut tree = lock!(self.editor.tree);
    let (_buf_id, buf) = {
      let buffer_manager = lock!(self.editor.buffer_manager);
      let (buf_id, buf) = buffer_manager.first_key_value().unwrap();
      (*buf_id, buf.clone())
    };
    let buf = Arc::downgrade(&buf);
    let cmdline_text = Arc::downgrade(&self.editor.cmdline_text);

    ui::init_default_window(
      &mut tree,
      buf,
      cmdline_text,
      cursor_blinking,
      cursor_hidden,
      cursor_style,
    );

    Ok(())
  }

  // Since we run user's config script (i.e. `.rsvim/rsvim.js`) before
  // initializing TUI/UI-tree. If user calls the `cmd.echo` API directly
  // in their configs right before the editor TUI initialize, the UI tree is
  // not created, and the "command-line-message" widget inside UI tree does not
  // exist.
  //
  // Thus we will have to store the printed messages in
  // `contents.command_line_message_history` temporarily. If the messages are
  // just too many, old messages will be thrown, only new messages are left.
  //
  // And all messages will be print once the editor TUI is initialized.
  fn _init_pending_messages(&mut self) {
    let mut cmdline_text = lock!(self.editor.cmdline_text);
    let mut tree = lock!(self.editor.tree);
    cmdline_ops::cmdline_set_last_pending_message_on_initialize(
      &mut tree,
      &mut cmdline_text,
    );
  }

  /// Shutdown.
  pub fn shutdown(&self) -> IoResult<()> {
    self.writer.shutdown()?;

    Ok(())
  }

  /// Handles one terminal event, and reports whether it may have changed the
  /// editor, i.e. whether the screen needs repainting.
  pub(crate) fn process_event(
    &mut self,
    event: Option<IoResult<Event>>,
  ) -> bool {
    match event {
      Some(Ok(event)) => {
        trace!("Polled terminal event ok: {:?}", event);
        let may_change = event_may_change_state(&event);
        let context = StateContext::new(
          self.editor.tree.clone(),
          self.editor.buffer_manager.clone(),
          self.editor.cmdline_text.clone(),
          self.master_tx.clone(),
          self.jsrt_tx.clone(),
        );

        // Handle by state machine
        let stateful = self.state;
        let next_stateful = stateful.handle(&context, event);
        self.state = next_stateful;
        may_change
      }
      Some(Err(e)) => {
        error!("Polled terminal event error: {:?}", e);
        // self.cancellation_token.cancel();
        false
      }
      None => {
        error!("Terminal event stream is exhausted");
        // self.cancellation_token.cancel();
        false
      }
    }
  }

  #[cfg(test)]
  fn _process_mocked_operations(
    &mut self,
    op: Option<IoResult<MockOperation>>,
  ) -> bool {
    match op {
      Some(Ok(op)) => {
        trace!("Polled editor operation ok: {:?}", op);
        match op {
          MockOperation::Operation(op) => {
            let context = StateContext::new(
              self.editor.tree.clone(),
              self.editor.buffer_manager.clone(),
              self.editor.cmdline_text.clone(),
              self.master_tx.clone(),
              self.jsrt_tx.clone(),
            );

            // Handle by state machine
            let stateful = self.state;
            let next_stateful = stateful.handle_op(&context, op);
            self.state = next_stateful;
          }
          MockOperation::Exit => {
            self.cancellation_token.cancel();
          }
          _ => unreachable!(),
        }
        true
      }
      Some(Err(e)) => {
        error!("Polled terminal event error: {:?}", e);
        // self.cancellation_token.cancel();
        false
      }
      None => {
        error!("Terminal event stream is exhausted");
        // self.cancellation_token.cancel();
        false
      }
    }
  }

  fn process_master_message(&mut self, message: MasterMessage) {
    match message {
      MasterMessage::ExitReq(req) => {
        trace!("Recv ExitReq:{:?}", req.exit_code);
        self.exit_code = req.exit_code;
        self.cancellation_token.cancel();
      }
      MasterMessage::SyntaxEditReq(req) => {
        self.process_syntax_edit_request(req.buffer_id);
      }
      MasterMessage::SyntaxEditResp(resp) => {
        trace!("Recv SyntaxEditResp:{:?}", resp.buffer_id);
      }
    }
  }

  /// Parses and queries the syntax of `buffer_id`, then reports back.
  ///
  /// Split out of [`Rsvim::process_master_message`] so the three remaining
  /// master messages (quit, this, and its completion) read as three lines.
  fn process_syntax_edit_request(&mut self, buffer_id: BufferId) {
    if let Some(buf) = lock!(self.editor.buffer_manager).get(&buffer_id) {
      let mut buf = lock!(buf);

      // Early quit if any below conditions are met:
      // 1. Has no syntax
      // 2. Syntax is parsing
      // 3. Syntax has no pending edits
      let has_no_syntax = buf.syntax().is_none();
      let syntax_is_parsing = buf
        .syntax()
        .as_ref()
        .map(|s| s.is_parsing())
        .unwrap_or(false);
      let syntax_has_pending_edits = buf
        .syntax()
        .as_ref()
        .map(|s| !s.pending_edits_is_empty())
        .unwrap_or(false);
      if has_no_syntax || syntax_is_parsing || !syntax_has_pending_edits {
        return;
      }

      let (pending_edits, syn_id, ts_parser, ts_tree, ts_highlight_query) = {
        let syn = buf.syntax_mut().as_mut().unwrap();
        syn.set_is_parsing(true);
        let syn_id = syn.id();
        let pending_edits = syn.drain_pending_edits(..).collect_vec();
        let ts_parser = syn.treesitter_parser();
        let ts_tree = syn.treesitter_tree().clone();
        let ts_highlight_query = syn.treesitter_highlight_query();
        (
          pending_edits,
          syn_id,
          ts_parser,
          ts_tree,
          ts_highlight_query,
        )
      };

      // release lock on the buffer
      drop(buf);

      let buffer_manager = self.editor.buffer_manager.clone();
      let master_tx = self.master_tx.clone();

      self.detached_tracker.spawn_blocking(move || {
        let (parsed_tree, parsed_editing_version, highlight_capture) =
          syntax::parse_and_query(
            ts_parser,
            ts_tree,
            ts_highlight_query,
            pending_edits,
          );

        // If the buffer and its syntax remains the same
        if let Some(buf) = lock!(buffer_manager).get(&buffer_id) {
          let mut buf = lock!(buf);
          if let Some(syn) = buf.syntax_mut() {
            if syn.id() == syn_id {
              syn.set_treesitter_tree(parsed_tree);
              syn.set_editing_version(parsed_editing_version);
              syn.set_highlight_capture(highlight_capture);
              syn.set_is_parsing(false);
            }

            // If buffer already has more pending editings, trigger next
            // parsing immediately.
            if !syn.pending_edits_is_empty() {
              chan::send_to_master(
                master_tx.clone(),
                MasterMessage::SyntaxEditReq(chan::SyntaxEditReq {
                  buffer_id: buf.id(),
                }),
              );
            }
          }
          // Notify syntax parsing/query is completed
          chan::send_to_master(
            master_tx,
            MasterMessage::SyntaxEditResp(chan::SyntaxEditResp {
              buffer_id: buf.id(),
            }),
          );
        }
      });
    }
  }

  /// Stops accepting background work and waits for the part of it that cannot
  /// be abandoned.
  ///
  /// `detached_tracker` holds work whose result nobody would see anyway once
  /// the loop is gone (parser loading, colorscheme parsing, read-only fs
  /// calls), so it is only closed. `blocked_tracker` holds work whose result
  /// was promised to the user -- a write, a child-process wait -- and dropping
  /// it silently is exactly the bug this stage is about, so we wait.
  ///
  /// Every way out of the loop -- Ctrl-D, cancellation, the javascript channel
  /// closing -- funnels through [`Rsvim::run_inner`], which calls this once.
  pub(crate) async fn close_trackers(&mut self) {
    self.detached_tracker.close();
    self.blocked_tracker.close();
    self.blocked_tracker.wait().await;
  }

  /// Repaints the terminal, but only if something actually changed.
  ///
  /// The loop used to draw and write on every iteration, which meant a busy
  /// javascript tick turned into a full repaint per tick.
  pub(crate) fn render_if_dirty(&mut self) -> IoResult<()> {
    if !self.dirty {
      return Ok(());
    }
    self.dirty = false;
    self.render()
  }

  fn render(&mut self) -> IoResult<()> {
    let context = WidgetContext::new(
      self.editor.syntax_manager.clone(),
      self.editor.colorscheme_manager.clone(),
    );
    lock!(self.editor.tree).draw(self.canvas.clone(), &context);
    self.writer.write(&mut lock!(self.canvas))
  }

  /// Running the loop, it repeatedly do following steps:
  ///
  /// 1. Receives several things:
  ///    1. User keyboard/mouse events.
  ///    2. Received messages.
  ///    3. Cancellation request (which tells this event loop to quit).
  /// 2. Use the editing state (FSM) to handle the event.
  /// 3. Render the terminal, if anything changed.
  pub async fn run(&mut self) -> IoResult<i32> {
    self
      .run_inner(EventStream::new(), |_| false, Rsvim::process_event)
      .await
  }

  #[cfg(test)]
  pub async fn run_with_mock_events(
    &mut self,
    reader: MockEventReader,
  ) -> IoResult<i32> {
    self
      .run_inner(reader, is_ctrl_d, Rsvim::process_event)
      .await
  }

  #[cfg(test)]
  pub async fn run_with_mock_operations(
    &mut self,
    reader: MockOperationReader,
  ) -> IoResult<i32> {
    // `MockOperation::Exit` keeps its current meaning: it cancels the token
    // (`_process_mocked_operations`), and the `[D]` branch breaks the loop.
    self
      .run_inner(reader, |_| false, Rsvim::_process_mocked_operations)
      .await
  }

  /// The single event loop.
  ///
  /// The three entry points above differ in exactly two ways: where the input
  /// stream comes from, and which input item means "quit". Everything else --
  /// the javascript tick, the cancellation, the rendering -- is shared.
  async fn run_inner<S, F, H>(
    &mut self,
    mut io: S,
    is_exit: F,
    on_input: H,
  ) -> IoResult<i32>
  where
    S: Stream + Unpin,
    F: Fn(&Option<S::Item>) -> bool,
    H: Fn(&mut Rsvim, Option<S::Item>) -> bool,
  {
    // The first frame is not drawn here: `initialize()` already did
    // `lock!(tree).draw(...)` + `writer.init_complete(...)`; drawing again
    // would flash two frames on startup.
    loop {
      if self.tick(&mut io, &is_exit, &on_input).await?.is_break() {
        break;
      }
    }

    // The loop can stop for three different reasons -- Ctrl-D, cancellation,
    // or the javascript channel closing -- and only the cancellation branch
    // used to wait for the trackers. Every way out funnels through here so
    // that quitting right after `await file.write(buf)` cannot drop the bytes
    // that were already promised to the user. `close_trackers` is idempotent,
    // so the cancellation branch having done it already costs nothing.
    self.close_trackers().await;

    Ok(self.exit_code)
  }

  /// One iteration of `Rsvim::run_inner`: **the** top-level loop body.
  ///
  /// The loop is event-driven, and javascript is just one of the things that
  /// produce events. One iteration is:
  ///
  /// 1. **Settle.** Handle everything javascript-side that is ready right now
  ///    ([`Rsvim::tick_event_loop`]): requests javascript made to the editor, ex
  ///    commands, finished operations, due timers. Bounded, so a source that
  ///    never runs dry cannot starve the terminal. This comes first so that
  ///    what was queued before the loop began -- the config script asked to
  ///    quit, or issued a write -- is served before any terminal input is
  ///    looked at.
  /// 2. **Paint** if anything changed, because the next step can block for as
  ///    long as the user likes.
  /// 3. **Wait for one event** from any source, and handle it. If it was
  ///    terminal input, take the rest of the burst that is already waiting, so
  ///    a run of keys is applied before the screen is repainted once.
  ///
  /// There is no step that drives javascript. Javascript never makes progress
  /// on its own: it runs when an event above hands it something, through
  /// [`JsHost::enter`], which also drains the microtask queue and reports
  /// unhandled rejections. That is why nothing here polls an engine scheduler
  /// or needs a wake-up bridge, and why every callback the user wrote is
  /// followed by exactly one chance to repaint.
  ///
  /// Split out of the `loop` so tests can drive the loop one iteration at a
  /// time instead of racing against a real input stream. Production calls it
  /// only from `Rsvim::run_inner`, so the split changes no behaviour:
  /// `run_inner` is exactly `loop { if tick(..).is_break() { break } }`.
  pub(crate) async fn tick<S, F, H>(
    &mut self,
    io: &mut S,
    is_exit: &F,
    on_input: &H,
  ) -> IoResult<ControlFlow<()>>
  where
    S: Stream + Unpin,
    F: Fn(&Option<S::Item>) -> bool,
    H: Fn(&mut Rsvim, Option<S::Item>) -> bool,
  {
    // ---- 1. settle ----
    self.tick_event_loop();
    if self.cancellation_token.is_cancelled() {
      return Ok(ControlFlow::Break(()));
    }

    // ---- 2. paint before blocking ----
    self.render_if_dirty()?;

    // ---- 3. wait for one event ----
    let mut flow = ControlFlow::Continue(());

    // NOTE: deliberately **not** `biased;`. `biased` polls the branches in
    // source order and lets the first ready one win, which is the opposite of
    // fairness: a steady stream of javascript completions would starve terminal
    // input -- the user could press keys and see nothing. Tokio's default random
    // choice is what provides fairness here.
    tokio::select! {

      // [A] An ex command or a finished operation.
      msg = self.jsrt_rx.recv() => {
        match msg {
          Some(message) => self.apply(Ready::Js(message)),
          // Every sender is gone, so no javascript result can ever arrive.
          // This must not happen on the normal exit path (`jsrt_tx` is owned by
          // `Rsvim`), so bail out instead of hanging silently.
          None => flow = ControlFlow::Break(()),
        }
      }

      // [B] A javascript-initiated request to the editor.
      req = self.master_rx.recv() => {
        if let Some(req) = req {
          self.apply(Ready::Master(req));
        }
      }

      // [C] Terminal input, or the mocked equivalent.
      item = io.next() => {
        flow = self.handle_input::<S, F, H>(item, is_exit, on_input);
        if flow.is_continue() {
          flow = self.drain_input::<S, F, H>(io, is_exit, on_input).await;
        }
      }

      // [D] Cancellation.
      _ = self.cancellation_token.cancelled() => {
        flow = ControlFlow::Break(());
      }

      // [E] A timer came due. `Ready(None)` -- no timer is armed -- disables
      // the branch for this wait instead of spinning on it.
      Some(timer) = std::future::poll_fn(|cx| self.js.poll_timer(cx)) => {
        self.apply(Ready::Timer(timer));
      }
    }

    // Leaving the loop: put the final state on screen. Otherwise the repaint
    // belongs to the next iteration, after it has settled whatever this event
    // queued (an ex command), so that one event costs one frame.
    if flow.is_break() {
      self.render_if_dirty()?;
    }

    Ok(flow)
  }

  /// Handles one terminal event.
  fn handle_input<S, F, H>(
    &mut self,
    item: Option<S::Item>,
    is_exit: &F,
    on_input: &H,
  ) -> ControlFlow<()>
  where
    S: Stream + Unpin,
    F: Fn(&Option<S::Item>) -> bool,
    H: Fn(&mut Rsvim, Option<S::Item>) -> bool,
  {
    if is_exit(&item) {
      return ControlFlow::Break(());
    }
    if on_input(self, item) {
      self.dirty = true;
    }
    ControlFlow::Continue(())
  }

  /// Takes the terminal events that are already waiting, without waiting for
  /// more, up to `INPUT_BATCH`.
  ///
  /// The stream is polled with the loop's *own* waker, never with a throwaway
  /// one (`now_or_never()` polls with a no-op waker). That matters because
  /// crossterm's `EventStream` remembers the waker of the poll that found
  /// nothing, wakes only that one when input arrives, and does not register
  /// another while one is pending. Polling it with a no-op waker therefore
  /// parks the stream behind a waker nobody listens to: the first key is
  /// handled, and every later key waits until something else wakes the loop.
  /// A mocked stream cannot show this; only a real terminal does.
  async fn drain_input<S, F, H>(
    &mut self,
    io: &mut S,
    is_exit: &F,
    on_input: &H,
  ) -> ControlFlow<()>
  where
    S: Stream + Unpin,
    F: Fn(&Option<S::Item>) -> bool,
    H: Fn(&mut Rsvim, Option<S::Item>) -> bool,
  {
    for _ in 0..INPUT_BATCH {
      // `poll!` polls with the waker of the task that is running.
      let std::task::Poll::Ready(item) = futures::poll!(io.next()) else {
        break;
      };
      if self
        .handle_input::<S, F, H>(item, is_exit, on_input)
        .is_break()
      {
        return ControlFlow::Break(());
      }
    }
    ControlFlow::Continue(())
  }
}

/// Whether a terminal event can change what is on screen.
///
/// The state machines ignore mouse and focus events, but crossterm's
/// `EnableMouseCapture` asks the terminal to report *every* mouse movement
/// (`?1003h`), so treating each event as a reason to repaint would repaint on
/// every twitch of the mouse. Update this when the state machines start
/// handling them.
fn event_may_change_state(event: &Event) -> bool {
  !matches!(
    event,
    Event::Mouse(_) | Event::FocusGained | Event::FocusLost
  )
}
