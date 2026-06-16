//! Event loop.

pub mod ui;
pub mod writer;

use crate::buf::BufferArc;
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
use crate::js::JsRuntime;
use crate::js::JsRuntimeOptions;
use crate::js::SnapshotData;
use crate::js::binding::global_rsvim::fs::link::async_fs_link;
use crate::js::binding::global_rsvim::fs::mkdir::fs_mkdir;
use crate::js::binding::global_rsvim::fs::open::async_fs_open;
use crate::js::binding::global_rsvim::fs::read::fs_read;
use crate::js::binding::global_rsvim::fs::read_file::async_fs_read_file;
use crate::js::binding::global_rsvim::fs::read_text_file::async_fs_read_text_file;
use crate::js::binding::global_rsvim::fs::stat::async_fs_lstat;
use crate::js::binding::global_rsvim::fs::stat::async_fs_stat;
use crate::js::binding::global_rsvim::fs::symlink::fs_symlink;
use crate::js::binding::global_rsvim::fs::write::fs_write;
use crate::js::command::CommandManager;
use crate::js::command::CommandManagerArc;
use crate::js::module::async_load_import;
use crate::js::resource::Resource;
use crate::js::resource::ResourceTable;
use crate::js::resource::ResourceTableArc;
use crate::prelude::*;
use crate::state::State;
use crate::state::StateContext;
use crate::state::Stateful;
use crate::state::ops::cmdline_ops;
use crate::syntax;
use crate::syntax::SyntaxEdit;
use crate::syntax::SyntaxEditNew;
use crate::syntax::SyntaxLoadGrammarRequest;
use crate::syntax::SyntaxManager;
use crate::syntax::SyntaxManagerArc;
use crate::ui::canvas::Canvas;
use crate::ui::canvas::CanvasArc;
use crate::ui::tree::*;
use crate::ui::widget::WidgetContext;
use compact_str::ToCompactString;
use crossterm::event::Event;
use crossterm::event::EventStream;
use futures::StreamExt;
use itertools::Itertools;
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

#[derive(Debug)]
/// For slow tasks that are suitable to put in the background, this event loop
/// will spawn them in tokio's async tasks and let them sync back data once
/// they are done. The event loop controls all the tasks with
/// [`CancellationToken`] and [`TaskTracker`].
///
/// Js runtime and event loop also communicate via channels, since js runtime
/// is based on v8 engine, all v8 APIs are not thread-safe.
pub struct EventLoop {
  /// Indicates the start time of the process.
  pub startup_moment: Instant,
  /// Specifies the timestamp which the current process began in Unix time.
  pub startup_unix_epoch: u128,

  /// Command line options.
  pub cli_opts: CliOptions,

  /// Stdout writer for editor mode TUI.
  pub writer: StdoutWriterValue,

  /// Widget tree for UI.
  pub tree: TreeArc,
  /// Canvas for UI.
  pub canvas: CanvasArc,

  /// Finite-state machine for editing state.
  pub state_machine: State,

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

  /// Js runtime.
  pub js_runtime: JsRuntime,

  /// Channel-1
  pub master_tx: UnboundedSender<MasterMessage>,
  pub master_rx: UnboundedReceiver<MasterMessage>,

  /// Channel-2
  pub jsrt_forwarder_tx: UnboundedSender<JsMessage>,
  pub jsrt_forwarder_rx: UnboundedReceiver<JsMessage>,

  /// Channel-3
  pub jsrt_tx: UnboundedSender<JsMessage>,
  // pub jsrt_rx: UnboundedReceiver<JsMessage>,
}

#[cfg(test)]
fn is_ctrl_d(event: &Option<IoResult<Event>>) -> bool {
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

impl EventLoop {
  #[allow(clippy::type_complexity)]
  pub fn _internal_new(
    terminal_cols: u16,
    terminal_rows: u16,
  ) -> IoResult<(
    /* canvas */ CanvasArc,
    /* tree */ TreeArc,
    /* state_machine */ State,
    /* buffer_manager */ BufferManagerArc,
    /* cmdline_text */ CmdlineTextArc,
    /* syntax_manager */ SyntaxManagerArc,
    /* colorscheme_manager */ ColorSchemeManagerArc,
    /* command_manager */ CommandManagerArc,
    /* resource_table */ ResourceTableArc,
    /* cancellation_token */ CancellationToken,
    /* detached_tracker */ TaskTracker,
    /* blocked_tracker */ TaskTracker,
    /* exit_code */ i32,
    (
      /* master_tx */ UnboundedSender<MasterMessage>,
      /* master_rx */ UnboundedReceiver<MasterMessage>,
    ),
    (
      /* jsrt_forwarder_tx */ UnboundedSender<JsMessage>,
      /* jsrt_forwarder_rx */ UnboundedReceiver<JsMessage>,
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
    let state_machine = State::default();

    // When implements `Promise`, `async`/`await` APIs for javascript runtime,
    // we need to leverage tokio's async runtime. i.e. first we send js task
    // requests to master (i.e. "this" event loop, here call it "master"), let
    // the master handles these tasks with tokio's async tasks. Once a task is
    // done, we send js task results back to js runtime.
    //
    // These tasks are low-level infrastructures, for example:
    //
    // - File IO
    // - Network
    // - Timer
    // - And more...
    //
    // But there are technical limitations that we cannot use tokio APIs along
    // with V8 engine, since V8 rust bindings are not Arc/Mutex (i.e. not
    // thread safe), while tokio async runtime requires Arc/Mutex (i.e. thread
    // safe).
    //
    // The request/response data flow uses below message channels:
    //
    // - Channel-1 `master_tx` => `master_rx` on `MasterMessage`.
    // - Channel-2 `jsrt_forwarder_tx` => `jsrt_forwarder_rx` on `JsMessage`.
    // - Channel-3 `jsrt_tx` => `jsrt_rx` on `JsMessage`.
    //
    // The dataflow follows below steps:
    //
    // 1. Js runtime --- [`MasterMessage`] (channel-1) --> Event loop
    // 2. Event loop handles js requests with tokio async tasks
    // 3. Event loop --- JsMessage (channel-2) --> Event loop
    // 4. Event loop --- JsMessage (channel-3) --> Js runtime
    // 5. Js runtime completes all async results.
    //
    // NOTE: You must notice, the step-3 and channel-2 seems unnecessary. Yes,
    // they're simply for trigger the `tokio::select!` loop.

    // Channel-1
    let (master_tx, master_rx) = unbounded_channel();
    // Channel-2
    let (jsrt_forwarder_tx, jsrt_forwarder_rx) = unbounded_channel();
    // Channel-3
    let (jsrt_tx, jsrt_rx) = unbounded_channel();

    Ok((
      canvas,
      tree,
      state_machine,
      buffer_manager,
      cmdline_text,
      syntax_manager,
      colorscheme_manager,
      command_manager,
      resource_table,
      CancellationToken::new(),
      TaskTracker::new(),
      TaskTracker::new(),
      0,
      (master_tx, master_rx),
      (jsrt_forwarder_tx, jsrt_forwarder_rx),
      (jsrt_tx, jsrt_rx),
    ))
  }

  /// Make new event loop.
  pub fn new(
    startup_moment: Instant,
    startup_unix_epoch: u128,
    cli_opts: CliOptions,
    snapshot: SnapshotData,
  ) -> IoResult<Self> {
    let (cols, rows) = crossterm::terminal::size()?;
    let (
      canvas,
      tree,
      state_machine,
      buffer_manager,
      cmdline_text,
      syntax_manager,
      colorscheme_manager,
      command_manager,
      resource_table,
      cancellation_token,
      detached_tracker,
      blocked_tracker,
      exit_code,
      (master_tx, master_rx),
      (jsrt_forwarder_tx, jsrt_forwarder_rx),
      (jsrt_tx, jsrt_rx),
    ) = Self::_internal_new(cols, rows)?;

    let writer = StdoutWriterValue::editor();

    // Js Runtime
    let js_runtime = JsRuntime::new(
      JsRuntimeOptions::default(),
      snapshot,
      startup_moment,
      startup_unix_epoch,
      master_tx.clone(),
      jsrt_rx,
      cli_opts.clone(),
      tree.clone(),
      buffer_manager.clone(),
      cmdline_text.clone(),
      syntax_manager.clone(),
      colorscheme_manager.clone(),
      command_manager.clone(),
      resource_table.clone(),
    );

    Ok(EventLoop {
      startup_moment,
      startup_unix_epoch,
      cli_opts,
      canvas,
      tree,
      state_machine,
      buffer_manager,
      cmdline_text,
      syntax_manager,
      colorscheme_manager,
      command_manager,
      resource_table,
      writer,
      cancellation_token,
      detached_tracker,
      blocked_tracker,
      exit_code,
      js_runtime,
      master_tx,
      master_rx,
      jsrt_forwarder_tx,
      jsrt_forwarder_rx,
      jsrt_tx,
    })
  }

  #[cfg(test)]
  /// Make new event loop for testing.
  pub fn mock_new_without_snapshot(
    terminal_columns: u16,
    terminal_rows: u16,
    cli_opts: CliOptions,
  ) -> IoResult<Self> {
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;

    let (
      canvas,
      tree,
      state_machine,
      buffer_manager,
      cmdline_text,
      syntax_manager,
      colorscheme_manager,
      command_manager,
      resource_table,
      cancellation_token,
      detached_tracker,
      blocked_tracker,
      exit_code,
      (master_tx, master_rx),
      (jsrt_forwarder_tx, jsrt_forwarder_rx),
      (jsrt_tx, jsrt_rx),
    ) = Self::_internal_new(terminal_columns, terminal_rows)?;

    let startup_moment = Instant::now();
    let startup_unix_epoch = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap()
      .as_millis();
    let writer = StdoutWriterValue::dev_null();

    // Js Runtime
    let js_runtime = JsRuntime::new_without_snapshot(
      JsRuntimeOptions::default(),
      startup_moment,
      startup_unix_epoch,
      master_tx.clone(),
      jsrt_rx,
      cli_opts.clone(),
      tree.clone(),
      buffer_manager.clone(),
      cmdline_text.clone(),
      syntax_manager.clone(),
      colorscheme_manager.clone(),
      command_manager.clone(),
      resource_table.clone(),
    );

    Ok(EventLoop {
      startup_moment,
      startup_unix_epoch,
      cli_opts,
      canvas,
      tree,
      state_machine,
      buffer_manager,
      cmdline_text,
      syntax_manager,
      colorscheme_manager,
      command_manager,
      resource_table,
      writer,
      cancellation_token,
      detached_tracker,
      blocked_tracker,
      exit_code,
      js_runtime,
      master_tx,
      master_rx,
      jsrt_forwarder_tx,
      jsrt_forwarder_rx,
      jsrt_tx,
    })
  }

  #[cfg(test)]
  /// Make new event loop for testing.
  pub fn mock_new_with_snapshot(
    terminal_columns: u16,
    terminal_rows: u16,
    cli_opts: CliOptions,
    snapshot: SnapshotData,
  ) -> IoResult<Self> {
    use std::time::SystemTime;
    use std::time::UNIX_EPOCH;

    let (
      canvas,
      tree,
      state_machine,
      buffer_manager,
      cmdline_text,
      syntax_manager,
      colorscheme_manager,
      command_manager,
      resource_table,
      cancellation_token,
      detached_tracker,
      blocked_tracker,
      exit_code,
      (master_tx, master_rx),
      (jsrt_forwarder_tx, jsrt_forwarder_rx),
      (jsrt_tx, jsrt_rx),
    ) = Self::_internal_new(terminal_columns, terminal_rows)?;

    let startup_moment = Instant::now();
    let startup_unix_epoch = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap()
      .as_millis();
    let writer = StdoutWriterValue::dev_null();

    // Js Runtime
    let js_runtime = JsRuntime::new(
      JsRuntimeOptions::default(),
      snapshot,
      startup_moment,
      startup_unix_epoch,
      master_tx.clone(),
      jsrt_rx,
      cli_opts.clone(),
      tree.clone(),
      buffer_manager.clone(),
      cmdline_text.clone(),
      syntax_manager.clone(),
      colorscheme_manager.clone(),
      command_manager.clone(),
      resource_table.clone(),
    );

    Ok(EventLoop {
      startup_moment,
      startup_unix_epoch,
      cli_opts,
      canvas,
      tree,
      state_machine,
      buffer_manager,
      cmdline_text,
      syntax_manager,
      colorscheme_manager,
      command_manager,
      resource_table,
      writer,
      cancellation_token,
      detached_tracker,
      blocked_tracker,
      exit_code,
      js_runtime,
      master_tx,
      master_rx,
      jsrt_forwarder_tx,
      jsrt_forwarder_rx,
      jsrt_tx,
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
      self.syntax_manager.clone(),
      self.colorscheme_manager.clone(),
    );
    lock!(self.tree).draw(self.canvas.clone(), &context);
    self.writer.init_complete(&mut lock!(self.canvas))?;

    Ok(())
  }

  /// Initialize user config file.
  fn _init_config(&mut self) -> IoResult<()> {
    if let Some(config_entry) = PATH_CONFIG.config_entry() {
      self
        .js_runtime
        .execute_module(&config_entry.to_string_lossy(), None);

      // Extra tick for some pending imports.
      self.js_runtime.tick_event_loop();
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
    let input_files = &self.cli_opts.file();
    if !input_files.is_empty() {
      for input_file in input_files.iter() {
        let maybe_buf_id =
          lock!(self.buffer_manager).new_file_buffer(canvas_size, input_file);
        match maybe_buf_id {
          Ok(buf_id) => {
            let buf = lock!(self.buffer_manager).get(&buf_id).unwrap().clone();
            self._add_pending_syntax_edit(buf);
            trace!("Created file buffer {:?}:{:?}", input_file, buf_id);
          }
          Err(e) => {
            // Send error message to command-line
            error!("Failed to create file buffer {:?}:{:?}", input_file, e);

            // Append error message to command line message history, wait for
            // print once TUI initialized.
            let mut cmdline_text = lock!(self.cmdline_text);
            cmdline_text
              .message_history_mut()
              .push_overwrite(e.to_string());
          }
        }
      }
    } else {
      let (buf_id, buf) = {
        let mut buffer_manager = lock!(self.buffer_manager);
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
    let mut tree = lock!(self.tree);
    let (_buf_id, buf) = {
      let buffer_manager = lock!(self.buffer_manager);
      let (buf_id, buf) = buffer_manager.first_key_value().unwrap();
      (*buf_id, buf.clone())
    };
    let buf = Arc::downgrade(&buf);
    let cmdline_text = Arc::downgrade(&self.cmdline_text);

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
  // initializing TUI/UI-tree. If user calls the `Rsvim.cmd.echo` API directly
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
    let mut cmdline_text = lock!(self.cmdline_text);
    let mut tree = lock!(self.tree);
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

  async fn process_event(&mut self, event: Option<IoResult<Event>>) {
    match event {
      Some(Ok(event)) => {
        trace!("Polled terminal event ok: {:?}", event);
        let context = StateContext::new(
          self.tree.clone(),
          self.buffer_manager.clone(),
          self.cmdline_text.clone(),
          self.master_tx.clone(),
          self.jsrt_forwarder_tx.clone(),
        );

        // Handle by state machine
        let stateful = self.state_machine;
        let next_stateful = stateful.handle(&context, event);
        self.state_machine = next_stateful;
      }
      Some(Err(e)) => {
        error!("Polled terminal event error: {:?}", e);
        // self.cancellation_token.cancel();
      }
      None => {
        error!("Terminal event stream is exhausted");
        // self.cancellation_token.cancel();
      }
    }
  }

  #[cfg(test)]
  async fn _process_mocked_operations(
    &mut self,
    op: Option<IoResult<MockOperation>>,
  ) {
    match op {
      Some(Ok(op)) => {
        trace!("Polled editor operation ok: {:?}", op);
        match op {
          MockOperation::Operation(op) => {
            let context = StateContext::new(
              self.tree.clone(),
              self.buffer_manager.clone(),
              self.cmdline_text.clone(),
              self.master_tx.clone(),
              self.jsrt_forwarder_tx.clone(),
            );

            // Handle by state machine
            let stateful = self.state_machine;
            let next_stateful = stateful.handle_op(&context, op);
            self.state_machine = next_stateful;
          }
          MockOperation::Exit => {
            self.cancellation_token.cancel();
          }
          _ => unreachable!(),
        }
      }
      Some(Err(e)) => {
        error!("Polled terminal event error: {:?}", e);
        // self.cancellation_token.cancel();
      }
      None => {
        error!("Terminal event stream is exhausted");
        // self.cancellation_token.cancel();
      }
    }
  }

  async fn process_master_message(&mut self, message: Option<MasterMessage>) {
    if let Some(message) = message {
      match message {
        MasterMessage::ExitReq(req) => {
          trace!("Recv ExitReq:{:?}", req.exit_code);
          self.exit_code = req.exit_code;
          self.cancellation_token.cancel();
        }
        MasterMessage::TimeoutReq(req) => {
          trace!("Recv TimeoutReq:{:?}", req.timer_id);
          let jsrt_forwarder_tx = self.jsrt_forwarder_tx.clone();
          self.detached_tracker.spawn(async move {
            let expire_at = req.start_at
              + tokio::time::Duration::from_millis(req.delay as u64);
            tokio::time::sleep_until(expire_at).await;
            jsrt_forwarder_tx
              .send(JsMessage::TimeoutResp(chan::TimeoutResp {
                timer_id: req.timer_id,
                expire_at,
                delay: req.delay,
                repeated: req.repeated,
              }))
              .unwrap();
          });
        }
        MasterMessage::LoadImportReq(req) => {
          trace!("Recv LoadImportReq:{:?}", req.task_id);
          let jsrt_forwarder_tx = self.jsrt_forwarder_tx.clone();
          self.detached_tracker.spawn(async move {
            let maybe_source = async_load_import(&req.specifier, false).await;
            jsrt_forwarder_tx
              .send(JsMessage::LoadImportResp(chan::LoadImportResp {
                task_id: req.task_id,
                maybe_source: match maybe_source {
                  Ok(source) => {
                    Some(Ok(postcard::to_allocvec(&source).unwrap()))
                  }
                  Err(e) => Some(Err(e)),
                },
              }))
              .unwrap();
          });
        }
        MasterMessage::TickAgainReq => {
          trace!("Recv TickAgainReq");
          let jsrt_forwarder_tx = self.jsrt_forwarder_tx.clone();
          self.detached_tracker.spawn(async move {
            jsrt_forwarder_tx.send(JsMessage::TickAgainResp).unwrap();
          });
        }
        MasterMessage::FsOpenReq(req) => {
          trace!("Recv FsOpenReq:{:?}", req.task_id);
          let jsrt_forwarder_tx = self.jsrt_forwarder_tx.clone();
          let resource_table = self.resource_table.clone();
          self.detached_tracker.spawn(async move {
            let maybe_result =
              async_fs_open(resource_table, req.path.as_path(), req.options)
                .await;
            jsrt_forwarder_tx
              .send(JsMessage::FsOpenResp(chan::FsOpenResp {
                task_id: req.task_id,
                maybe_result: match maybe_result {
                  Ok(result) => {
                    Some(Ok(postcard::to_allocvec(&result).unwrap()))
                  }
                  Err(e) => Some(Err(e)),
                },
              }))
              .unwrap();
          });
        }
        MasterMessage::FsReadReq(req) => {
          trace!("Recv FsReadReq:{:?}", req.task_id);
          let resource_table = self.resource_table.clone();
          let jsrt_forwarder_tx = self.jsrt_forwarder_tx.clone();
          self.detached_tracker.spawn(async move {
            let maybe_result =
              fs_read(resource_table, req.file_rid, req.bufsize);
            jsrt_forwarder_tx
              .send(JsMessage::FsReadResp(chan::FsReadResp {
                task_id: req.task_id,
                maybe_result: Some(maybe_result),
              }))
              .unwrap();
          });
        }
        MasterMessage::FsWriteReq(req) => {
          trace!("Recv FsWriteReq:{:?}", req.task_id);
          let resource_table = self.resource_table.clone();
          let jsrt_forwarder_tx = self.jsrt_forwarder_tx.clone();
          self.detached_tracker.spawn_blocking(move || {
            let maybe_result = fs_write(resource_table, req.file_rid, req.buf);
            jsrt_forwarder_tx
              .send(JsMessage::FsWriteResp(chan::FsWriteResp {
                task_id: req.task_id,
                maybe_result: match maybe_result {
                  Ok(result) => {
                    Some(Ok(postcard::to_allocvec(&result).unwrap()))
                  }
                  Err(e) => Some(Err(e)),
                },
              }))
              .unwrap();
          });
        }
        MasterMessage::FsReadFileReq(req) => {
          trace!("Recv FsReadFileReq:{:?}", req.task_id);
          let jsrt_forwarder_tx = self.jsrt_forwarder_tx.clone();
          self.detached_tracker.spawn(async move {
            let maybe_result = async_fs_read_file(req.path.as_path()).await;
            jsrt_forwarder_tx
              .send(JsMessage::FsReadFileResp(chan::FsReadFileResp {
                task_id: req.task_id,
                maybe_result: match maybe_result {
                  Ok(result) => Some(Ok(result)),
                  Err(e) => Some(Err(e)),
                },
              }))
              .unwrap();
          });
        }
        MasterMessage::FsReadTextFileReq(req) => {
          trace!("Recv FsReadTextFileReq:{:?}", req.task_id);
          let jsrt_forwarder_tx = self.jsrt_forwarder_tx.clone();
          self.detached_tracker.spawn(async move {
            let maybe_result =
              async_fs_read_text_file(req.path.as_path()).await;
            jsrt_forwarder_tx
              .send(JsMessage::FsReadTextFileResp(chan::FsReadTextFileResp {
                task_id: req.task_id,
                maybe_result: match maybe_result {
                  Ok(result) => {
                    Some(Ok(postcard::to_allocvec(&result).unwrap()))
                  }
                  Err(e) => Some(Err(e)),
                },
              }))
              .unwrap();
          });
        }
        MasterMessage::FsStatReq(req) => {
          trace!("Recv FsStatReq:{:?}", req.task_id);
          let jsrt_forwarder_tx = self.jsrt_forwarder_tx.clone();
          self.detached_tracker.spawn(async move {
            let maybe_result = if req.follow_symlink {
              async_fs_stat(req.path.as_path()).await
            } else {
              async_fs_lstat(req.path.as_path()).await
            };
            jsrt_forwarder_tx
              .send(JsMessage::FsStatResp(chan::FsStatResp {
                task_id: req.task_id,
                maybe_result: match maybe_result {
                  Ok(result) => {
                    Some(Ok(postcard::to_allocvec(&result).unwrap()))
                  }
                  Err(e) => Some(Err(e)),
                },
              }))
              .unwrap();
          });
        }
        MasterMessage::FsSymlinkReq(req) => {
          trace!("Recv FsSymlinkReq:{:?}", req.task_id);
          let jsrt_forwarder_tx = self.jsrt_forwarder_tx.clone();
          self.detached_tracker.spawn_blocking(move || {
            let maybe_result = fs_symlink(
              req.oldpath.as_path(),
              req.newpath.as_path(),
              req.options,
            );
            jsrt_forwarder_tx
              .send(JsMessage::FsSymlinkResp(chan::FsSymlinkResp {
                task_id: req.task_id,
                maybe_result: match maybe_result {
                  Ok(_) => Some(Ok(postcard::to_allocvec(&0_u32).unwrap())),
                  Err(e) => Some(Err(e)),
                },
              }))
              .unwrap();
          });
        }
        MasterMessage::FsLinkReq(req) => {
          trace!("Recv FsLinkReq:{:?}", req.task_id);
          let jsrt_forwarder_tx = self.jsrt_forwarder_tx.clone();
          self.detached_tracker.spawn(async move {
            let maybe_result =
              async_fs_link(req.oldpath.as_path(), req.newpath.as_path()).await;
            jsrt_forwarder_tx
              .send(JsMessage::FsLinkResp(chan::FsLinkResp {
                task_id: req.task_id,
                maybe_result: match maybe_result {
                  Ok(_) => Some(Ok(postcard::to_allocvec(&0_u32).unwrap())),
                  Err(e) => Some(Err(e)),
                },
              }))
              .unwrap();
          });
        }
        MasterMessage::FsMkdirReq(req) => {
          trace!("Recv FsMkdirReq:{:?}", req.task_id);
          let jsrt_forwarder_tx = self.jsrt_forwarder_tx.clone();
          self.detached_tracker.spawn_blocking(move || {
            let maybe_result = fs_mkdir(req.path.as_path(), req.options);
            jsrt_forwarder_tx
              .send(JsMessage::FsMkdirResp(chan::FsMkdirResp {
                task_id: req.task_id,
                maybe_result: match maybe_result {
                  Ok(_) => Some(Ok(postcard::to_allocvec(&0_u32).unwrap())),
                  Err(e) => Some(Err(e)),
                },
              }))
              .unwrap();
          });
        }
        MasterMessage::SyntaxEditReq(req) => {
          trace!("Recv SyntaxEditReq:{:?}", req.buffer_id);
          if let Some(buf) = lock!(self.buffer_manager).get(&req.buffer_id) {
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

            let buffer_manager = self.buffer_manager.clone();
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
              if let Some(buf) = lock!(buffer_manager).get(&req.buffer_id) {
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
        MasterMessage::SyntaxEditResp(resp) => {
          trace!("Recv SyntaxEditResp:{:?}", resp.buffer_id);
        }
        MasterMessage::LoadTreeSitterParserReq(req) => {
          trace!("Recv LoadTreeSitterGrammarReq:{:?}", req.task_id);
          let syn_manager = self.syntax_manager.clone();
          let jsrt_forwarder_tx = self.jsrt_forwarder_tx.clone();

          self.detached_tracker.spawn(async move {
            match req.grammar_path.absolutize() {
              Ok(grammar_path) => {
                let load_req = SyntaxLoadGrammarRequest {
                  grammar_path: grammar_path.to_path_buf(),
                };
                let load_result =
                  syntax::async_load_syntax_grammar(syn_manager, &load_req)
                    .await;
                match load_result {
                  Ok(metainfo) => {
                    let grammar_names = metainfo
                      .grammars
                      .iter()
                      .map(|grammar| grammar.name.to_string())
                      .collect_vec();
                    trace!("Load tree-sitter grammars:{:?}", grammar_names);
                    jsrt_forwarder_tx
                      .send(JsMessage::LoadTreeSitterParserResp(
                        chan::LoadTreeSitterParserResp {
                          task_id: req.task_id,
                          maybe_result: Some(Ok(
                            postcard::to_allocvec(&grammar_names).unwrap(),
                          )),
                        },
                      ))
                      .unwrap();
                  }
                  Err(e) => {
                    jsrt_forwarder_tx
                      .send(JsMessage::LoadTreeSitterParserResp(
                        chan::LoadTreeSitterParserResp {
                          task_id: req.task_id,
                          maybe_result: Some(Err(e)),
                        },
                      ))
                      .unwrap();
                  }
                }
              }
              Err(_e) => {
                let e = TheErr::TreeSitterParserNotFound(
                  req.grammar_path.to_string_lossy().to_compact_string(),
                );
                jsrt_forwarder_tx
                  .send(JsMessage::LoadTreeSitterParserResp(
                    chan::LoadTreeSitterParserResp {
                      task_id: req.task_id,
                      maybe_result: Some(Err(e)),
                    },
                  ))
                  .unwrap();
              }
            }
          });
        }
        MasterMessage::ReadTextFromChildProcessStdioReq(req) => {
          trace!(
            "Recv ReadTextFromChildProcessStdioReq:{:?} rid:{}",
            req.task_id, req.rid
          );

          let resource_table = self.resource_table.clone();
          let jsrt_forwarder_tx = self.jsrt_forwarder_tx.clone();

          self.detached_tracker.spawn_blocking(move || {
            let handle_read = |result, payload| match result {
              Ok(_n) => jsrt_forwarder_tx
                .send(JsMessage::ReadTextFromChildProcessStdioResp(
                  chan::ReadTextFromChildProcessStdioResp {
                    task_id: req.task_id,
                    maybe_result: Some(Ok(
                      postcard::to_allocvec(&payload).unwrap(),
                    )),
                  },
                ))
                .unwrap(),
              Err(e) => jsrt_forwarder_tx
                .send(JsMessage::ReadTextFromChildProcessStdioResp(
                  chan::ReadTextFromChildProcessStdioResp {
                    task_id: req.task_id,
                    maybe_result: Some(Err(
                      TheErr::ReadChildProcessStdioFailed(req.rid, e),
                    )),
                  },
                ))
                .unwrap(),
            };

            let child_stdio = lock!(resource_table).get(&req.rid).cloned();
            match child_stdio {
              Some(child_stdio) => match child_stdio {
                Resource::ChildProcessStdout(resource) => {
                  use std::io::Read;
                  let resource = resource.data();
                  let mut resource = lock!(resource);
                  let mut payload = String::new();
                  let result = resource.read_to_string(&mut payload);
                  handle_read(result, payload)
                }
                Resource::ChildProcessStderr(resource) => {
                  use std::io::Read;
                  let resource = resource.data();
                  let mut resource = lock!(resource);
                  let mut payload = String::new();
                  let result = resource.read_to_string(&mut payload);
                  handle_read(result, payload)
                }
                _ => unreachable!(),
              },
              None => {
                // Child process (stdio) not found
                jsrt_forwarder_tx
                  .send(JsMessage::ReadTextFromChildProcessStdioResp(
                    chan::ReadTextFromChildProcessStdioResp {
                      task_id: req.task_id,
                      maybe_result: Some(Err(TheErr::ChildProcessNotFound(
                        req.rid,
                      ))),
                    },
                  ))
                  .unwrap();
              }
            }
          });
        }
        MasterMessage::WaitChildProcessReq(req) => {
          trace!("Recv WaitChildProcessReq:{:?} rid:{}", req.task_id, req.rid);

          let resource_table = self.resource_table.clone();
          let jsrt_forwarder_tx = self.jsrt_forwarder_tx.clone();

          self.detached_tracker.spawn_blocking(move || {
            let child = lock!(resource_table).get(&req.rid).cloned();
            match child {
              Some(child) => match child {
                Resource::ChildProcess(resource) => {
                  let resource = resource.data();
                  let mut resource = lock!(resource);
                  match resource.wait() {
                    Ok(exit_status) => {
                      let success = exit_status.success();
                      let exit_code = exit_status.code();
                      let signal: Option<i32> = {
                        #[cfg(target_family = "unix")]
                        {
                          use std::os::unix::process::ExitStatusExt;
                          exit_status.signal()
                        }
                        #[cfg(not(target_family = "unix"))]
                        {
                          None
                        }
                      };
                      jsrt_forwarder_tx
                        .send(JsMessage::WaitChildProcessResp(
                          chan::WaitChildProcessResp {
                            task_id: req.task_id,
                            maybe_result: Some(Ok(
                              postcard::to_allocvec(&(
                                success, exit_code, signal,
                              ))
                              .unwrap(),
                            )),
                          },
                        ))
                        .unwrap();
                    }
                    Err(e) => {
                      jsrt_forwarder_tx
                        .send(JsMessage::WaitChildProcessResp(
                          chan::WaitChildProcessResp {
                            task_id: req.task_id,
                            maybe_result: Some(Err(
                              TheErr::WaitChildProcessFailed(req.rid, e),
                            )),
                          },
                        ))
                        .unwrap();
                    }
                  }
                }
                _ => unreachable!(),
              },
              None => {
                // Child process not found
                jsrt_forwarder_tx
                  .send(JsMessage::WaitChildProcessResp(
                    chan::WaitChildProcessResp {
                      task_id: req.task_id,
                      maybe_result: Some(Err(TheErr::ChildProcessNotFound(
                        req.rid,
                      ))),
                    },
                  ))
                  .unwrap();
              }
            }
          });
        }
      }
    }
  }

  async fn forward_js_message(&mut self, message: Option<JsMessage>) {
    if let Some(message) = message {
      trace!("Process resp msg:{:?}", message);
      self.jsrt_tx.send(message).unwrap();
      self.js_runtime.tick_event_loop();
    }
  }

  async fn process_cancellation_notify(&mut self) {
    trace!("Receive cancellation token, exit loop");
    self.detached_tracker.close();
    self.blocked_tracker.close();
    self.blocked_tracker.wait().await;
  }

  /// Running the loop, it repeatedly do following steps:
  ///
  /// 1. Receives several things:
  ///    1. User keyboard/mouse events.
  ///    2. Received messages.
  ///    3. Cancellation request (which tells this event loop to quit).
  /// 2. Use the editing state (FSM) to handle the event.
  /// 3. Render the terminal.
  pub async fn run(&mut self) -> IoResult<i32> {
    let mut reader = EventStream::new();
    loop {
      tokio::select! {
        // Receive keyboard/mouse events
        event = reader.next() => {
          self.process_event(event).await;
        }
        // Receive master message
        master_msg = self.master_rx.recv() => {
          self.process_master_message(master_msg).await;
        }
        // Receive loopback js message (should be sent to js runtime)
        js_msg = self.jsrt_forwarder_rx.recv() => {
          self.forward_js_message(js_msg).await;
        }
        // Receive cancellation notify
        _ = self.cancellation_token.cancelled() => {
          self.process_cancellation_notify().await;
          break;
        }
      }

      // Flush logic UI to terminal, i.e. print UI to stdout
      let context = WidgetContext::new(
        self.syntax_manager.clone(),
        self.colorscheme_manager.clone(),
      );
      lock!(self.tree).draw(self.canvas.clone(), &context);
      self.writer.write(&mut lock!(self.canvas))?;
    }

    Ok(self.exit_code)
  }

  #[cfg(test)]
  pub async fn run_with_mock_events(
    &mut self,
    mut reader: MockEventReader,
  ) -> IoResult<i32> {
    loop {
      tokio::select! {
        // Receive mocked keyboard/mouse events
        event = reader.next() => {
          if is_ctrl_d(&event) {
            break;
          }
          self.process_event(event).await;
        }
        master_msg = self.master_rx.recv() => {
          self.process_master_message(master_msg).await;
        }
        js_msg = self.jsrt_forwarder_rx.recv() => {
          self.forward_js_message(js_msg).await;
        }
        _ = self.cancellation_token.cancelled() => {
          self.process_cancellation_notify().await;
          break;
        }
      }

      // Flush logic UI to terminal, i.e. print UI to stdout
      let context = WidgetContext::new(
        self.syntax_manager.clone(),
        self.colorscheme_manager.clone(),
      );
      lock!(self.tree).draw(self.canvas.clone(), &context);
      self.writer.write(&mut lock!(self.canvas))?;
    }

    Ok(self.exit_code)
  }

  #[cfg(test)]
  pub async fn run_with_mock_operations(
    &mut self,
    mut reader: MockOperationReader,
  ) -> IoResult<i32> {
    loop {
      tokio::select! {
        // Receive mocked keyboard/mouse events
        op = reader.next() => {
          self._process_mocked_operations(op).await;
        }
        master_msg = self.master_rx.recv() => {
          self.process_master_message(master_msg).await;
        }
        js_msg = self.jsrt_forwarder_rx.recv() => {
          self.forward_js_message(js_msg).await;
        }
        _ = self.cancellation_token.cancelled() => {
          self.process_cancellation_notify().await;
          break;
        }
      }

      // Flush logic UI to terminal, i.e. print UI to stdout
      let context = WidgetContext::new(
        self.syntax_manager.clone(),
        self.colorscheme_manager.clone(),
      );
      lock!(self.tree).draw(self.canvas.clone(), &context);
      self.writer.write(&mut lock!(self.canvas))?;
    }

    Ok(self.exit_code)
  }
}
