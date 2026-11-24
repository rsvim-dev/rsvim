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
use crate::js::JsKernelRc;
use crate::js::JsRuntimeOptions;
use crate::js::binding::fs::link::fs_link_a;
use crate::js::binding::fs::mkdir::fs_mkdir_s;
use crate::js::binding::fs::open::fs_open_a;
use crate::js::binding::fs::read::fs_read_s;
use crate::js::binding::fs::read_dir::fs_read_dir_next_s;
use crate::js::binding::fs::read_dir::fs_read_dir_s;
use crate::js::binding::fs::read_file::fs_read_file_a;
use crate::js::binding::fs::read_text_file::fs_read_text_file_a;
use crate::js::binding::fs::stat::fs_lstat_a;
use crate::js::binding::fs::stat::fs_stat_a;
use crate::js::binding::fs::symlink::fs_symlink_s;
use crate::js::binding::fs::write::fs_write_s;
use crate::js::command::CommandManager;
use crate::js::command::CommandManagerArc;
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
use futures::Stream;
use futures::StreamExt;
use itertools::Itertools;
use std::ops::ControlFlow;
use std::sync::Arc;
use taffy::Style;
use tokio::sync::Notify;
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
/// Every field is behind an `Arc`, and `Arc`s are what makes the split work:
/// [`Rsvim`] owns the loop, while v8 callbacks can only reach the isolate
/// slot, i.e. [`crate::js::JsKernel`]. Both hold a clone of this struct, and
/// all the clones point to the very same `Arc`s.
///
/// It cannot be stored in a single place because v8 callbacks
/// (`promise_reject_cb`, `module_resolve_cb`, all the bindings) reach the
/// kernel through [`JsRuntime::state`](crate::js::JsRuntime::state), and have
/// no way to reach the outer `Rsvim`.
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

/// The editor's owner: the v8 isolate, its kernel, and all the shared editor
/// data in one place.
///
/// This used to be named `EventLoop`; the alias [`EventLoop`] is kept so that
/// the binary and the doc links don't have to change.
///
/// For slow tasks that are suitable to put in the background, this event loop
/// will spawn them in tokio's async tasks and let them sync back data once
/// they are done. The event loop controls all the tasks with
/// [`CancellationToken`] and [`TaskTracker`].
///
/// Js runtime and event loop also communicate via channels, since js runtime
/// is based on v8 engine, all v8 APIs are not thread-safe.
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
  pub state_machine: State,

  /// Editor's shared data (buffers, UI tree, managers, resources, cli options).
  ///
  /// The same `Arc`s are also cloned into [`crate::js::JsKernel`], because v8 callbacks
  /// can only reach the isolate slot.
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

  /// V8 isolate.
  pub isolate: v8::OwnedIsolate,
  /// The kernel of the js runtime, i.e. the state stored inside the v8
  /// isolate's slot.
  pub kernel: JsKernelRc,

  /// Channel-1
  pub master_tx: UnboundedSender<MasterMessage>,
  pub master_rx: UnboundedReceiver<MasterMessage>,

  /// Channel-2, the single javascript channel.
  ///
  /// The sender is cloned into the FSM (`StateContext`) and into every
  /// background task; the receiver is the `[A]` branch of
  /// `Rsvim::run_inner`, i.e. the only tick driver.
  ///
  /// This used to be two channels with a forwarding hop in between. The hop
  /// is gone, and the two channels are folded into this one: a completion
  /// event is both the result and the trigger to advance the isolate.
  pub jsrt_tx: UnboundedSender<JsMessage>,
  pub jsrt_rx: UnboundedReceiver<JsMessage>,

  /// Whether there is javascript work that can be advanced right now, i.e.
  /// the previous tick returned [`TickOutcome::MoreWork`]. Consumed at the
  /// top of every `Rsvim::run_inner` iteration.
  pub js_tick_again: bool,
  /// How many consecutive self-driven ticks have been performed without
  /// going back to the `select!`. Bounded by `JS_SPIN_BUDGET`.
  pub js_spin: usize,
  /// Whether the exhausted-budget warning has already been emitted for the
  /// current run of self-driven ticks, so a long microtask chain is reported
  /// once instead of once per retry.
  pub js_spin_warned: bool,
  /// Whether the terminal needs to be repainted. Replaces the old
  /// "redraw after every select iteration" behaviour, which turned a busy
  /// javascript loop into a full repaint per tick.
  pub dirty: bool,
  /// Wakes the loop up for javascript work that has no payload.
  ///
  /// One producer exists today: when the self-driving budget is exhausted
  /// while `pending_futures` is still non-empty, a short delayed
  /// `notify_one` is scheduled, so that work is retried instead of being
  /// stranded forever. A custom v8 platform would be the second producer.
  pub js_wake: Arc<Notify>,
  /// Bounded backoff for [`v8::Isolate::has_pending_background_tasks`].
  ///
  /// "A v8 background task is still running" is a *waiting* state, not a
  /// *runnable* one, so it must not drive the self-driving tick (see
  /// `evloop-plan.md` §5.4(b)). But nothing else notifies us either, so the
  /// loop would never re-check. This interval does exactly that: it is only
  /// armed while that predicate is true, it doubles `1ms -> 2ms -> ... ->
  /// 16ms`, and it is reset by any real progress.
  pub js_backoff: std::time::Duration,
}

/// The self-assessment of a single javascript tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickOutcome {
  /// There's no javascript work that can be advanced right now, it's safe to
  /// go back to the `select!` and wait.
  Idle,
  /// There's still work that is ready *right now* (newly ready `JsFuture`s,
  /// newly arrived messages), it's worth ticking once more -- but the number
  /// of consecutive self-driven ticks must be bounded by
  /// `JS_SPIN_BUDGET`.
  MoreWork,
}

impl Rsvim {
  /// Advances the javascript event loop by a single tick.
  ///
  /// The order strictly follows Dune (`dune/src/runtime.rs:427-432`):
  ///   1. microtask checkpoint
  ///   2. consume the completion events, each followed by
  ///      checkpoint + flush errors + the [`crate::js::JsFuture`]s it just made ready
  ///   3. run whatever [`crate::js::JsFuture`] was pending before the tick started
  ///   4. flush the exceptions captured outside the two loops above
  ///   5. decide whether there's more work that can be advanced immediately
  ///
  /// Fixes P0-1: step 2 and step 3 flush the errors after **each** item,
  /// instead of only calling `check_exceptions` for `JsFuture`s like the old
  /// `run_pending_futures` did.
  ///
  /// The `inbox` is owned by the caller: the select branches drain `jsrt_rx`
  /// and pass the messages in, so that this function never touches the
  /// receiver (which would be a double borrow of `self` while `isolate` is
  /// borrowed by the scope).
  pub fn tick_js(&mut self, inbox: Vec<JsMessage>) -> TickOutcome {
    // Split the borrow: `isolate` and `kernel` are two disjoint fields, this
    // is the keystone of the whole merged design.
    let Self {
      ref mut isolate,
      ref kernel,
      ..
    } = *self;
    let context = kernel.borrow().context.clone();

    let mut worked = false;

    let more = {
      v8::scope_with_context!(scope, isolate, context);

      // 1) microtask checkpoint
      Self::checkpoint(scope);

      // 2) consume the completion events, running the `JsFuture`s each one
      // produces **before** the next event is observed.
      //
      // NOTE: the whole inbox is drained up-front (`Rsvim::drain_js_inbox`)
      // because the event loop owns whatever already landed in the channel.
      // But a completion event's *consequences* must be applied before the
      // next event is looked at, otherwise the callbacks observe an ordering
      // that cannot happen in Node. Concrete case: `setInterval(3ms)` plus
      // `setTimeout(20ms, () => clearInterval(id))`; when both `TimeoutResp`s
      // are in the same inbox, running all the messages first and the futures
      // afterwards makes the interval callback fire *after* the `clearInterval`
      // that was supposed to cancel it, so an extra tick shows up on the
      // command line.
      for msg in inbox {
        crate::js::process_js_message(kernel, scope, msg);
        Self::flush_js_errors(kernel, scope);
        Self::checkpoint(scope);
        Self::run_ready_futures(kernel, scope);
        worked = true;
      }

      // 3) the futures that were already pending when the tick started.
      //
      // This is a no-op when the inbox was non-empty (step 2 already ran
      // them) and the actual work when it was empty. It keeps the old
      // `run_pending_futures` behaviour for the "no message, only futures"
      // case.
      worked |= Self::run_ready_futures(kernel, scope);

      // 4) report whatever was captured by the js engine outside of the two
      // loops above -- typically a rejection raised while evaluating a module
      // (`execute_module`), which has no message and no future to hang the
      // flush on. `check_exceptions` takes the captured vector, so this is
      // idempotent: a second tick reports nothing.
      Self::flush_js_errors(kernel, scope);

      // 5) work that can be advanced immediately (**excluding**
      // `has_pending_background_tasks`, see §5.4)
      Self::has_runnable_js_work(kernel)
    };

    // A tick that executed javascript may have touched the editor -- a
    // `cmd.echo`, an option, a buffer. A tick that executed nothing cannot
    // have, so it must not buy itself a full repaint: that is exactly the
    // "busy javascript loop turns into a repaint per tick" problem the dirty
    // flag exists to solve.
    if worked {
      self.dirty = true;
    }

    if more {
      TickOutcome::MoreWork
    } else {
      TickOutcome::Idle
    }
  }

  fn checkpoint(scope: &mut v8::PinScope) {
    crate::js::checkpoint(scope);
  }

  /// Turns the "captured exceptions" into "reported exceptions".
  ///
  /// Fixes P0-1: `check_exceptions` used to `mem::take` the whole
  /// `promise_rejections` vector but then `return Some(error)` from every
  /// branch, so N rejections only reported the first one. It now returns a
  /// `Vec<JsError>` and we report all of them.
  fn flush_js_errors(kernel: &JsKernelRc, scope: &mut v8::PinScope) {
    for err in crate::js::check_exceptions(scope) {
      let state = kernel.borrow();
      crate::js::err::report_js_error(&state, TheErr::JsError(Box::new(err)));
    }
  }

  /// Runs the [`crate::js::JsFuture`]s that are ready *right now*, once.
  ///
  /// `mem::take` first, so the borrow on `kernel` is released before the
  /// futures run and they are free to push new ones (which then land in the
  /// *next* tick, matching the old `run_pending_futures`).
  /// Runs every `JsFuture` that was pending when the tick reached this point.
  ///
  /// Returns whether anything actually ran: `tick_js` uses that to decide if
  /// the terminal has to be repainted, so an idle tick must not repaint.
  ///
  /// NOTE: `mem::take` releases the borrow before the futures run, so a
  /// future that pushes a new `JsFuture` into `pending_futures` lands in the
  /// *next* tick. That matches the old `run_pending_futures` behaviour.
  fn run_ready_futures(kernel: &JsKernelRc, scope: &mut v8::PinScope) -> bool {
    let futures: Vec<Box<dyn crate::js::JsFuture>> =
      std::mem::take(&mut kernel.borrow_mut().pending_futures);
    let ran = !futures.is_empty();
    for mut fut in futures {
      fut.run(scope);
      Self::flush_js_errors(kernel, scope);
      Self::checkpoint(scope);
    }
    ran
  }

  /// The work that is ready *right now*.
  ///
  /// This is **not** the same thing as "is there pending work" (which is what
  /// keeps the runtime alive).
  ///
  /// NOTE: this deliberately only looks at `pending_futures` and ignores
  /// `pending_tasks`. `pending_futures` is a Dune-style "ready completion
  /// queue" (see the `JsFuture` trait: `fn run(&mut self, scope)` -- no
  /// poll/Waker), so a non-empty queue means "it can run now". `pending_tasks`
  /// on the other hand holds the waiters of tasks that were already dispatched
  /// but whose result hasn't come back: non-empty only means "there's work in
  /// flight", not "it can be advanced now". Counting it as runnable is the
  /// same class of bug as P0-1 (mistaking *waiting* for *runnable*).
  fn has_runnable_js_work(kernel: &JsKernelRc) -> bool {
    !kernel.borrow().pending_futures.is_empty()
  }

  /// Drains the javascript inbox and advances the runtime by one tick.
  ///
  /// This is the "one-shot" entry point, used by `_init_config` (to settle the
  /// imports of the config file) and by tests. The event loop proper does not
  /// call it: `Rsvim::run_inner` drives [`Rsvim::tick_js`] directly, so that
  /// the inbox drain and the tick happen inside a single `select!` iteration.
  pub fn tick_event_loop(&mut self) -> TickOutcome {
    let inbox = self.drain_js_inbox(None);
    self.tick_js(inbox)
  }

  /// Executes javascript source code as ES module, i.e. ECMA standard.
  pub fn execute_module(&mut self, filename: &str, source: Option<&str>) {
    let Self {
      ref mut isolate,
      ref kernel,
      ..
    } = *self;
    let context = kernel.borrow().context.clone();
    v8::scope_with_context!(scope, isolate, context);
    crate::js::execute_module(scope, filename, source);
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
    /* state_machine */ State,
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
    // - Channel-2 `jsrt_tx` => `jsrt_rx` on `JsMessage`.
    //
    // The dataflow follows below steps:
    //
    // 1. Js runtime --- [`MasterMessage`] (channel-1) --> Event loop
    // 2. Event loop handles js requests with tokio async tasks
    // 3. Background tasks --- JsMessage (channel-2) --> Event loop
    // 4. Event loop advances the js runtime in-place, it already owns the
    //    isolate.
    //
    // NOTE: there used to be a third channel that carried the results from
    // channel-2 into the js runtime, plus a forwarding hop. Both are gone: the
    // event loop *is* the js runtime now, so the completion event is also the
    // tick trigger.

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
      state_machine,
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
      state_machine,
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
    let (isolate, kernel) = crate::js::new_isolate(
      JsRuntimeOptions::default(),
      master_tx.clone(),
      editor.clone(),
    );

    Ok(Rsvim {
      startup_moment,
      startup_unix_epoch,
      canvas,
      state_machine,
      editor,
      writer,
      cancellation_token,
      detached_tracker,
      blocked_tracker,
      exit_code,
      isolate,
      kernel,
      master_tx,
      master_rx,
      jsrt_tx,
      jsrt_rx,
      js_tick_again: false,
      js_spin: 0,
      js_spin_warned: false,
      dirty: false,
      js_wake: Arc::new(Notify::new()),
      js_backoff: Self::JS_BACKOFF_MIN,
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
      state_machine,
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
    let (isolate, kernel) = crate::js::new_isolate(
      JsRuntimeOptions::default(),
      master_tx.clone(),
      editor.clone(),
    );

    Ok(Rsvim {
      startup_moment,
      startup_unix_epoch,
      canvas,
      state_machine,
      editor,
      writer,
      cancellation_token,
      detached_tracker,
      blocked_tracker,
      exit_code,
      isolate,
      kernel,
      master_tx,
      master_rx,
      jsrt_tx,
      jsrt_rx,
      js_tick_again: false,
      js_spin: 0,
      js_spin_warned: false,
      dirty: false,
      js_wake: Arc::new(Notify::new()),
      js_backoff: Self::JS_BACKOFF_MIN,
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
      self.execute_module(&config_entry.to_string_lossy(), None);

      // Extra tick for some pending imports.
      self.tick_event_loop();
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

  fn process_event(&mut self, event: Option<IoResult<Event>>) {
    match event {
      Some(Ok(event)) => {
        trace!("Polled terminal event ok: {:?}", event);
        let context = StateContext::new(
          self.editor.tree.clone(),
          self.editor.buffer_manager.clone(),
          self.editor.cmdline_text.clone(),
          self.master_tx.clone(),
          self.jsrt_tx.clone(),
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
  fn _process_mocked_operations(
    &mut self,
    op: Option<IoResult<MockOperation>>,
  ) {
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

  fn process_master_message(&mut self, message: MasterMessage) {
    {
      match message {
        MasterMessage::ExitReq(req) => {
          trace!("Recv ExitReq:{:?}", req.exit_code);
          self.exit_code = req.exit_code;
          self.cancellation_token.cancel();
        }
        MasterMessage::TimeoutReq(req) => {
          trace!("Recv TimeoutReq:{:?}", req.timer_id);
          let jsrt_tx = self.jsrt_tx.clone();
          self.detached_tracker.spawn(async move {
            let expire_at = req.start_at
              + tokio::time::Duration::from_millis(req.delay as u64);
            tokio::time::sleep_until(expire_at).await;
            let _ = jsrt_tx.send(JsMessage::TimeoutResp(chan::TimeoutResp {
              timer_id: req.timer_id,
              expire_at,
              delay: req.delay,
              repeated: req.repeated,
            }));
          });
        }
        MasterMessage::FsOpenReq(req) => {
          trace!("Recv FsOpenReq:{:?}", req.task_id);
          let jsrt_tx = self.jsrt_tx.clone();
          let resource_table = self.editor.resource_table.clone();
          self.detached_tracker.spawn(async move {
            let maybe_result =
              fs_open_a(resource_table, req.path.as_path(), req.options).await;
            let _ = jsrt_tx.send(JsMessage::FsOpenResp(chan::FsOpenResp {
              task_id: req.task_id,
              maybe_result: match maybe_result {
                Ok(result) => Some(Ok(postcard::to_allocvec(&result).unwrap())),
                Err(e) => Some(Err(e)),
              },
            }));
          });
        }
        MasterMessage::FsReadReq(req) => {
          trace!("Recv FsReadReq:{:?}", req.task_id);
          let resource_table = self.editor.resource_table.clone();
          let jsrt_tx = self.jsrt_tx.clone();
          self.detached_tracker.spawn(async move {
            let maybe_result =
              fs_read_s(resource_table, req.file_rid, req.bufsize);
            let _ = jsrt_tx.send(JsMessage::FsReadResp(chan::FsReadResp {
              task_id: req.task_id,
              maybe_result: Some(maybe_result),
            }));
          });
        }
        MasterMessage::FsWriteReq(req) => {
          trace!("Recv FsWriteReq:{:?}", req.task_id);
          let resource_table = self.editor.resource_table.clone();
          let jsrt_tx = self.jsrt_tx.clone();
          // A write is not abortable: the bytes are already promised to the
          // user. Run it on `blocked_tracker`, which `close_trackers` waits
          // for on every exit path, so quitting right after
          // `await file.write(buf)` cannot silently drop the tail of the file.
          self.blocked_tracker.spawn_blocking(move || {
            let maybe_result =
              fs_write_s(resource_table, req.file_rid, req.buf);
            let _ = jsrt_tx.send(JsMessage::FsWriteResp(chan::FsWriteResp {
              task_id: req.task_id,
              maybe_result: match maybe_result {
                Ok(result) => Some(Ok(postcard::to_allocvec(&result).unwrap())),
                Err(e) => Some(Err(e)),
              },
            }));
          });
        }
        MasterMessage::FsReadDirReq(req) => {
          trace!("Recv FsReadDirReq:{:?}", req.task_id);
          let jsrt_tx = self.jsrt_tx.clone();
          let resource_table = self.editor.resource_table.clone();
          self.detached_tracker.spawn_blocking(move || {
            let maybe_result =
              fs_read_dir_s(resource_table, req.path.as_path());
            let _ =
              jsrt_tx.send(JsMessage::FsReadDirResp(chan::FsReadDirResp {
                task_id: req.task_id,
                maybe_result: match maybe_result {
                  Ok(result) => {
                    Some(Ok(postcard::to_allocvec(&result).unwrap()))
                  }
                  Err(e) => Some(Err(e)),
                },
              }));
          });
        }
        MasterMessage::FsReadDirNextReq(req) => {
          trace!("Recv FsReadDirNextReq:{:?}", req.task_id);
          let jsrt_tx = self.jsrt_tx.clone();
          let resource_table = self.editor.resource_table.clone();
          self.detached_tracker.spawn_blocking(move || {
            let maybe_result = fs_read_dir_next_s(resource_table, req.rid);
            let _ =
              jsrt_tx.send(JsMessage::FsReadDirResp(chan::FsReadDirResp {
                task_id: req.task_id,
                maybe_result: match maybe_result {
                  Some(Ok(result)) => {
                    Some(Ok(postcard::to_allocvec(&result).unwrap()))
                  }
                  Some(Err(e)) => Some(Err(e)),
                  None => None,
                },
              }));
          });
        }
        MasterMessage::FsReadFileReq(req) => {
          trace!("Recv FsReadFileReq:{:?}", req.task_id);
          let jsrt_tx = self.jsrt_tx.clone();
          self.detached_tracker.spawn(async move {
            let maybe_result = fs_read_file_a(req.path.as_path()).await;
            let _ =
              jsrt_tx.send(JsMessage::FsReadFileResp(chan::FsReadFileResp {
                task_id: req.task_id,
                maybe_result: match maybe_result {
                  Ok(result) => Some(Ok(result)),
                  Err(e) => Some(Err(e)),
                },
              }));
          });
        }
        MasterMessage::FsReadTextFileReq(req) => {
          trace!("Recv FsReadTextFileReq:{:?}", req.task_id);
          let jsrt_tx = self.jsrt_tx.clone();
          self.detached_tracker.spawn(async move {
            let maybe_result = fs_read_text_file_a(req.path.as_path()).await;
            let _ = jsrt_tx.send(JsMessage::FsReadTextFileResp(
              chan::FsReadTextFileResp {
                task_id: req.task_id,
                maybe_result: match maybe_result {
                  Ok(result) => {
                    Some(Ok(postcard::to_allocvec(&result).unwrap()))
                  }
                  Err(e) => Some(Err(e)),
                },
              },
            ));
          });
        }
        MasterMessage::FsStatReq(req) => {
          trace!("Recv FsStatReq:{:?}", req.task_id);
          let jsrt_tx = self.jsrt_tx.clone();
          self.detached_tracker.spawn(async move {
            let maybe_result = if req.follow_symlink {
              fs_stat_a(req.path.as_path()).await
            } else {
              fs_lstat_a(req.path.as_path()).await
            };
            let _ = jsrt_tx.send(JsMessage::FsStatResp(chan::FsStatResp {
              task_id: req.task_id,
              maybe_result: match maybe_result {
                Ok(result) => Some(Ok(postcard::to_allocvec(&result).unwrap())),
                Err(e) => Some(Err(e)),
              },
            }));
          });
        }
        MasterMessage::FsSymlinkReq(req) => {
          trace!("Recv FsSymlinkReq:{:?}", req.task_id);
          let jsrt_tx = self.jsrt_tx.clone();
          self.detached_tracker.spawn_blocking(move || {
            let maybe_result = fs_symlink_s(
              req.oldpath.as_path(),
              req.newpath.as_path(),
              req.options,
            );
            let _ =
              jsrt_tx.send(JsMessage::FsSymlinkResp(chan::FsSymlinkResp {
                task_id: req.task_id,
                maybe_result: match maybe_result {
                  Ok(_) => Some(Ok(postcard::to_allocvec(&0_u32).unwrap())),
                  Err(e) => Some(Err(e)),
                },
              }));
          });
        }
        MasterMessage::FsLinkReq(req) => {
          trace!("Recv FsLinkReq:{:?}", req.task_id);
          let jsrt_tx = self.jsrt_tx.clone();
          self.detached_tracker.spawn(async move {
            let maybe_result =
              fs_link_a(req.oldpath.as_path(), req.newpath.as_path()).await;
            let _ = jsrt_tx.send(JsMessage::FsLinkResp(chan::FsLinkResp {
              task_id: req.task_id,
              maybe_result: match maybe_result {
                Ok(_) => Some(Ok(postcard::to_allocvec(&0_u32).unwrap())),
                Err(e) => Some(Err(e)),
              },
            }));
          });
        }
        MasterMessage::FsMkdirReq(req) => {
          trace!("Recv FsMkdirReq:{:?}", req.task_id);
          let jsrt_tx = self.jsrt_tx.clone();
          self.detached_tracker.spawn_blocking(move || {
            let maybe_result = fs_mkdir_s(req.path.as_path(), req.options);
            let _ = jsrt_tx.send(JsMessage::FsMkdirResp(chan::FsMkdirResp {
              task_id: req.task_id,
              maybe_result: match maybe_result {
                Ok(_) => Some(Ok(postcard::to_allocvec(&0_u32).unwrap())),
                Err(e) => Some(Err(e)),
              },
            }));
          });
        }
        MasterMessage::SyntaxEditReq(req) => {
          trace!("Recv SyntaxEditReq:{:?}", req.buffer_id);
          if let Some(buf) =
            lock!(self.editor.buffer_manager).get(&req.buffer_id)
          {
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
          let syn_manager = self.editor.syntax_manager.clone();
          let jsrt_tx = self.jsrt_tx.clone();

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
                    let _ = jsrt_tx.send(JsMessage::LoadTreeSitterParserResp(
                      chan::LoadTreeSitterParserResp {
                        task_id: req.task_id,
                        maybe_result: Some(Ok(
                          postcard::to_allocvec(&grammar_names).unwrap(),
                        )),
                      },
                    ));
                  }
                  Err(e) => {
                    let _ = jsrt_tx.send(JsMessage::LoadTreeSitterParserResp(
                      chan::LoadTreeSitterParserResp {
                        task_id: req.task_id,
                        maybe_result: Some(Err(e)),
                      },
                    ));
                  }
                }
              }
              Err(_e) => {
                let e = TheErr::TreeSitterParserNotFound(
                  req.grammar_path.to_string_lossy().to_compact_string(),
                );
                let _ = jsrt_tx.send(JsMessage::LoadTreeSitterParserResp(
                  chan::LoadTreeSitterParserResp {
                    task_id: req.task_id,
                    maybe_result: Some(Err(e)),
                  },
                ));
              }
            }
          });
        }
        MasterMessage::ReadTextFromChildProcessStdioReq(req) => {
          trace!(
            "Recv ReadTextFromChildProcessStdioReq:{:?} rid:{}",
            req.task_id, req.rid
          );

          let resource_table = self.editor.resource_table.clone();
          let jsrt_tx = self.jsrt_tx.clone();

          self.detached_tracker.spawn_blocking(move || {
            let handle_read = |result, payload| match result {
              Ok(_n) => jsrt_tx
                .send(JsMessage::ReadTextFromChildProcessStdioResp(
                  chan::ReadTextFromChildProcessStdioResp {
                    task_id: req.task_id,
                    maybe_result: Some(Ok(
                      postcard::to_allocvec(&payload).unwrap(),
                    )),
                  },
                ))
                .unwrap(),
              Err(e) => jsrt_tx
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
                let _ =
                  jsrt_tx.send(JsMessage::ReadTextFromChildProcessStdioResp(
                    chan::ReadTextFromChildProcessStdioResp {
                      task_id: req.task_id,
                      maybe_result: Some(Err(TheErr::ChildProcessNotFound(
                        req.rid,
                      ))),
                    },
                  ));
              }
            }
          });
        }
        MasterMessage::WaitChildProcessReq(req) => {
          trace!("Recv WaitChildProcessReq:{:?} rid:{}", req.task_id, req.rid);

          let resource_table = self.editor.resource_table.clone();
          let jsrt_tx = self.jsrt_tx.clone();

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
                      let _ = jsrt_tx.send(JsMessage::WaitChildProcessResp(
                        chan::WaitChildProcessResp {
                          task_id: req.task_id,
                          maybe_result: Some(Ok(
                            postcard::to_allocvec(&(
                              success, exit_code, signal,
                            ))
                            .unwrap(),
                          )),
                        },
                      ));
                    }
                    Err(e) => {
                      let _ = jsrt_tx.send(JsMessage::WaitChildProcessResp(
                        chan::WaitChildProcessResp {
                          task_id: req.task_id,
                          maybe_result: Some(Err(
                            TheErr::WaitChildProcessFailed(req.rid, e),
                          )),
                        },
                      ));
                    }
                  }
                }
                _ => unreachable!(),
              },
              None => {
                // Child process not found
                let _ = jsrt_tx.send(JsMessage::WaitChildProcessResp(
                  chan::WaitChildProcessResp {
                    task_id: req.task_id,
                    maybe_result: Some(Err(TheErr::ChildProcessNotFound(
                      req.rid,
                    ))),
                  },
                ));
              }
            }
          });
        }
      }
    }
  }

  pub(crate) async fn process_cancellation_notify(&mut self) {
    trace!("Receive cancellation token, exit loop");
    self.close_trackers().await;
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
  /// Idempotent: `[D]` closes the trackers, and `[A]`/`[C]` reach the same
  /// place on their own exit paths, so every way out funnels through here.
  async fn close_trackers(&mut self) {
    self.detached_tracker.close();
    self.blocked_tracker.close();
    self.blocked_tracker.wait().await;
  }

  /// Reports what the js side was still holding when the loop ended, then
  /// lets it all be dropped.
  ///
  /// Three things happen here, and only these three:
  ///
  /// 1. A last [`Rsvim::flush_js_errors`]. A rejection raised while a module is
  ///    being evaluated never gets a tick to be flushed in, so without this it
  ///    would vanish without a word.
  /// 2. A one-shot diagnostic when a module is still sitting on a top-level
  ///    `await`. Pending timers and pending [`crate::js::JsFuture`]s are
  ///    dropped silently, which matches Node and Deno: `setTimeout(..., 10000)`
  ///    must not keep the process alive. A never-settling top-level await is a
  ///    different animal -- the user wrote code that was supposed to run, so
  ///    exiting 0 in silence is a trap. We still do not block the exit (the
  ///    user asked to quit), we just say what got skipped.
  ///
  /// NOTE: the trackers are deliberately *not* touched here.
  /// [`Rsvim::process_cancellation_notify`] already closes and waits for them
  /// on every exit path; doing it a second time would only lean on
  /// `TaskTracker::close` being idempotent.
  ///
  /// Consequently there is nothing left to `.await`, which is why this is not
  /// an `async fn`.
  pub(crate) fn shutdown_js(&mut self) {
    let Self {
      ref mut isolate,
      ref kernel,
      ..
    } = *self;
    let context = kernel.borrow().context.clone();

    v8::scope_with_context!(scope, isolate, context);

    Self::flush_js_errors(kernel, scope);

    let pending = kernel.borrow().module_map.pending_paths(scope);
    if !pending.is_empty() {
      warn!(
        "Exiting with {} module(s) still pending on a top-level await: {}",
        pending.len(),
        pending.iter().join(", ")
      );
    }
  }

  /// How many consecutive self-driven ticks are allowed before the loop
  /// insists on going back to the `select!`.
  ///
  /// A self-driven tick means: the previous [`Rsvim::tick_js`] returned
  /// [`TickOutcome::MoreWork`], so there is javascript work that can be
  /// advanced *right now* -- running it in the same iteration is both faster
  /// and more correct than bouncing through a channel. The budget bounds the
  /// pathological case (a promise chain that keeps queueing more work), so
  /// that terminal input and cancellation always get a chance to be polled.
  pub(crate) const JS_SPIN_BUDGET: usize = 8;

  /// Delay before retrying self-driven work whose budget was exhausted.
  ///
  /// Long enough that a pathological microtask chain cannot spin the CPU, short
  /// enough that a legitimate chain (e.g. a long `await`-free loop building a
  /// buffer) completes without a visible stall.
  pub(crate) const JS_RETRY_DELAY: std::time::Duration =
    std::time::Duration::from_millis(1);

  /// Lower bound of the background-task backoff, see [`Rsvim::js_backoff`].
  pub(crate) const JS_BACKOFF_MIN: std::time::Duration =
    std::time::Duration::from_millis(1);
  /// Upper bound of the background-task backoff. 16ms is one frame at 60Hz:
  /// above that a v8 background task would visibly delay the UI.
  pub(crate) const JS_BACKOFF_MAX: std::time::Duration =
    std::time::Duration::from_millis(16);

  /// The backoff to arm for this iteration, or `None` when nothing is
  /// waiting on a v8 background task.
  fn js_backoff_deadline(&self) -> Option<Instant> {
    if !self.isolate.has_pending_background_tasks() {
      return None;
    }
    Some(Instant::now() + self.js_backoff)
  }

  /// Resolves at `deadline`, or never when there is no deadline.
  ///
  /// `select!` needs a branch that is always present, so "disabled" is
  /// expressed as a future that never resolves instead of a conditional arm.
  async fn js_backoff_tick(deadline: Option<Instant>) {
    match deadline {
      Some(deadline) => tokio::time::sleep_until(deadline).await,
      None => std::future::pending::<()>().await,
    }
  }

  /// Doubles the backoff up to [`Rsvim::JS_BACKOFF_MAX`].
  fn grow_js_backoff(&mut self) {
    self.js_backoff =
      std::cmp::min(self.js_backoff.saturating_mul(2), Self::JS_BACKOFF_MAX);
  }

  /// Any real progress means the next background-task check can be prompt
  /// again.
  fn reset_js_backoff(&mut self) {
    self.js_backoff = Self::JS_BACKOFF_MIN;
  }

  /// Diagnostics for an exhausted self-driving budget.
  ///
  /// Deliberately *not* calling `v8::Isolate::terminate_execution()`: that
  /// leaves the isolate in an unrecoverable state and there is no reset path.
  fn report_js_spin_warning(&self) {
    warn!(
      "js self-driving tick budget ({}) exhausted; pending_futures={}, \
       has_pending_background_tasks={}",
      Self::JS_SPIN_BUDGET,
      self.kernel.borrow().pending_futures.len(),
      self.isolate.has_pending_background_tasks(),
    );
  }

  /// Takes every javascript message that has already arrived.
  ///
  /// `first` is the message the `[A]` branch already `recv`ed, so it does not
  /// have to travel through the channel twice.
  ///
  /// This is the only source of [`Rsvim::tick_js`]'s inbox, which is what
  /// keeps `tick_js` from ever touching `self.jsrt_rx` -- otherwise the
  /// receiver borrow would collide with the isolate borrow inside the tick.
  fn drain_js_inbox(&mut self, first: Option<JsMessage>) -> Vec<JsMessage> {
    let mut inbox: Vec<JsMessage> = first.into_iter().collect();
    while let Ok(msg) = self.jsrt_rx.try_recv() {
      inbox.push(msg);
    }
    inbox
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
    H: Fn(&mut Rsvim, Option<S::Item>),
  {
    // The first frame is not drawn here: `initialize()` already did
    // `lock!(tree).draw(...)` + `writer.init_complete(...)`; drawing again
    // would flash two frames on startup.
    loop {
      if self.step(&mut io, &is_exit, &on_input).await?.is_break() {
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

    // Everything that the js side was still holding is reported and dropped
    // here, before the caller (`rsvim_cli`) reaches `std::process::exit`.
    self.shutdown_js();

    Ok(self.exit_code)
  }

  /// One iteration of `Rsvim::run_inner`.
  ///
  /// Split out of the `loop` so tests can drive the loop one iteration at a
  /// time instead of racing against a real input stream. Production calls it
  /// only from `Rsvim::run_inner`, so the split changes no behaviour:
  /// `run_inner` is exactly `loop { if step(..).is_break() { break } }`.
  pub(crate) async fn step<S, F, H>(
    &mut self,
    io: &mut S,
    is_exit: &F,
    on_input: &H,
  ) -> IoResult<ControlFlow<()>>
  where
    S: Stream + Unpin,
    F: Fn(&Option<S::Item>) -> bool,
    H: Fn(&mut Rsvim, Option<S::Item>),
  {
    // Self-drive: advance the work that is ready *right now* in this very
    // iteration instead of manufacturing a message to come back through the
    // select.
    if self.js_tick_again {
      // NOTE: don't write `self.tick_js(self.drain_js_inbox(None))` -- both
      // take `&mut self`, nesting them is E0499. Drain first, then tick.
      let inbox = self.drain_js_inbox(None);
      let outcome = self.tick_js(inbox);
      match outcome {
        TickOutcome::MoreWork => {
          self.js_spin += 1;
          if self.js_spin <= Self::JS_SPIN_BUDGET {
            // Yield without blocking: IO tasks and background tasks get a
            // chance to make progress, while the next tick needs no channel.
            tokio::task::yield_now().await;
            self.render_if_dirty()?;
            return Ok(ControlFlow::Continue(()));
          }
          // Budget exhausted: the js side is sustaining itself (typically an
          // endless microtask chain). Stop self-driving so the select regains
          // control, and report once rather than once per retry.
          if !self.js_spin_warned {
            self.report_js_spin_warning();
            self.js_spin_warned = true;
          }
          self.js_tick_again = false;
          // Exhausting the budget does **not** mean the work is gone:
          // `MoreWork` means `pending_futures` is still non-empty. Without a
          // nudge, that chain would be stranded forever -- no new message
          // will arrive for it. Schedule one bounded retry (at most one per
          // exhaustion, so this cannot become a hot loop) through the same
          // `js_wake` the `[E]` branch consumes.
          let wake = self.js_wake.clone();
          self.detached_tracker.spawn(async move {
            tokio::time::sleep(Self::JS_RETRY_DELAY).await;
            wake.notify_one();
          });
        }
        TickOutcome::Idle => {
          self.js_spin = 0;
          self.js_spin_warned = false;
          self.js_tick_again = false;
        }
      }
      self.render_if_dirty()?;
    }

    let mut flow = ControlFlow::Continue(());

    // Armed only while v8 still has background tasks: those are *waiting*
    // work, so they must not self-drive (that was P0-1), but nothing else
    // would ever come back to check them.
    let backoff = self.js_backoff_deadline();

    // NOTE: deliberately **not** `biased;`. `biased` polls the branches in
    // source order and lets the first ready one win, which is the opposite of
    // fairness: with `[A]` first, a steady stream of javascript completions
    // would starve terminal input (`[C]`) and master messages (`[B]`) -- the
    // user could press keys and see nothing. Tokio's default random choice is
    // what provides fairness here; `JS_SPIN_BUDGET` governs the self-driving
    // loop *outside* `select!` and cannot compensate for branch order.
    tokio::select! {

      // [A] JS completion -- the only tick driver.
      msg = self.jsrt_rx.recv() => {
        match msg {
          Some(first) => {
            self.reset_js_backoff();
            let inbox = self.drain_js_inbox(Some(first));
            let outcome = self.tick_js(inbox);
            self.js_tick_again = outcome == TickOutcome::MoreWork;
          }
          // Every sender is gone, so no javascript result can ever arrive.
          // This must not happen on the normal exit path (`jsrt_tx`
          // is owned by `Rsvim`), so bail out instead of hanging silently.
          None => flow = ControlFlow::Break(()),
        }
      }

      // [B] A javascript-initiated IO request.
      req = self.master_rx.recv() => {
        if let Some(req) = req {
          self.process_master_message(req);
          self.dirty = true;
        }
      }

      // [C] Terminal input, or the mocked equivalent.
      item = io.next() => {
        if is_exit(&item) {
          flow = ControlFlow::Break(());
        } else {
          on_input(self, item);
          self.dirty = true;
        }
      }

      // [D] Cancellation.
      _ = self.cancellation_token.cancelled() => {
        self.process_cancellation_notify().await;
        flow = ControlFlow::Break(());
      }

      // [E] A javascript wake-up with no payload (a v8 background task
      // finished, or an external interrupt). Nothing notifies it yet; it is
      // kept as the precise-wakeup slot for a `post_task` hook.
      _ = self.js_wake.notified() => {
        self.reset_js_backoff();
        // Resetting the budget here is what makes the retry path useful: the
        // wake means "real progress may be possible again", so the next eight
        // ticks are allowed. A pathological microtask chain therefore runs in
        // bounded bursts separated by `JS_RETRY_DELAY`, instead of either hot
        // spinning or being stranded.
        self.js_spin = 0;
        self.js_spin_warned = false;
        let inbox = self.drain_js_inbox(None);
        let outcome = self.tick_js(inbox);
        self.js_tick_again = outcome == TickOutcome::MoreWork;
      }

      // [F] Bounded backoff for v8 background tasks, see `js_backoff`.
      //
      // A false tick here means "nothing happened yet", so the interval
      // doubles. It stays `waiting`, never `runnable`: this branch does not
      // set `js_tick_again`.
      _ = Self::js_backoff_tick(backoff) => {
        self.grow_js_backoff();
        let inbox = self.drain_js_inbox(None);
        let outcome = self.tick_js(inbox);
        self.js_tick_again = outcome == TickOutcome::MoreWork;
      }
    }

    self.render_if_dirty()?;

    Ok(flow)
  }
}
