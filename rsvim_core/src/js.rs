//! JavaScript runtime.
//!
//! References:
//! - Embed V8: <https://v8.dev/docs/embed>.
//! - Roll your own JavaScript runtime: <https://deno.com/blog/roll-your-own-javascript-runtime>.
//! - Learning V8 Tutorial: <https://github.com/danbev/learning-v8>.
//! - A hobby runtime for javascript/typescript: <https://github.com/aalykiot/dune>.
//! - V8 API Reference: <https://v8docs.nodesource.com/node-24.1/index.html>.

pub mod binding;
pub mod command;
pub mod converter;
pub mod err;
pub mod exception;
pub mod hook;
pub mod loader;
pub mod module;
pub mod pending;
pub mod resource;

#[cfg(test)]
mod command_tests;
#[cfg(test)]
mod converter_tests;
#[cfg(test)]
mod module_tests;

use crate::buf::BufferManagerArc;
use crate::cfg::path_cfg::PATH_CONFIG;
use crate::chan;
use crate::chan::JsMessage;
use crate::chan::MasterMessage;
use crate::cli::CliOptions;
use crate::cmdltext::CmdlineTextArc;
use crate::hl::ColorSchemeManagerArc;
use crate::prelude::*;
use crate::syntax::SyntaxManagerArc;
use crate::ui::tree::TreeArc;
pub use boost::*;
use command::CommandManagerArc;
use err::JsError;
use err::report_js_error;
use exception::ExceptionState;
use exception::PromiseRejectionEntry;
use hook::module_resolve_cb;
use module::ImportMap;
use module::ModuleMap;
use module::fetch_module;
use module::fetch_module_tree;
use module::resolve_import;
use pending::TaskCallback;
use pending::TimerCallback;
use resource::ResourceTableArc;
use std::sync::Once;
use tokio::sync::mpsc::UnboundedReceiver;
use tokio::sync::mpsc::UnboundedSender;
use tokio::time::Instant;

pub fn v8_version() -> &'static str {
  v8::VERSION_STRING
}

// /// A vector with JS callbacks and parameters.
// type NextTickQueue = Vec<(v8::Global<v8::Function>, Vec<v8::Global<v8::Value>>)>;

/// An abstract interface for javascript `Promise` and `async`. Since
/// everything in V8 needs the `&mut v8::PinScope` to operate with, we cannot
/// simply put the async task into tokio `spawn` API.
pub trait JsFuture {
  fn run(&mut self, scope: &mut v8::PinScope);
}

// TimerId start from 1.
#[derive(
  Copy, Clone, rsvim_macro::IncrementalId, serde::Serialize, serde::Deserialize,
)]
pub struct TimerId(#[start_from(1)] i32);

// TaskId start from 1.
#[derive(
  Copy, Clone, rsvim_macro::IncrementalId, serde::Serialize, serde::Deserialize,
)]
pub struct TaskId(#[start_from(1)] usize);

pub fn init_v8_platform() {
  static V8_INIT: Once = Once::new();

  V8_INIT.call_once(move || {
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
  });
}

fn init_v8_isolate(isolate: &mut v8::OwnedIsolate) {
  // NOTE: Set microtasks policy to explicit, this requires we invoke `perform_microtask_checkpoint` API on each tick.
  // See: [`run_next_tick_callbacks`].
  isolate.set_microtasks_policy(v8::MicrotasksPolicy::Explicit);
  isolate.set_capture_stack_trace_for_uncaught_exceptions(true, 10);
  isolate.set_promise_reject_callback(hook::promise_reject_cb);
  isolate.set_host_initialize_import_meta_object_callback(
    hook::host_initialize_import_meta_object_cb,
  );

  // WARNING: dynamic import is not supported, at least for now.
  // isolate.set_host_import_module_dynamically_callback(
  //   hook::host_import_module_dynamically_cb,
  // );
}

fn init_builtin_modules(scope: &mut v8::PinScope) {
  static BUILTIN_MODULES: [(/* filename */ &str, /* source */ &str); 2] = [
    (
      "00__web.js",
      include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/js/00__web.min.js")),
    ),
    (
      "01__rsvim.js",
      include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/js/01__rsvim.min.js")),
    ),
  ];

  for module in BUILTIN_MODULES.iter() {
    let filename = module.0;
    let source = module.1;

    v8::tc_scope!(let tc_scope, scope);

    let module = fetch_module(tc_scope, filename, Some(source)).unwrap();
    let _ = module
      .instantiate_module(tc_scope, module_resolve_cb)
      .unwrap();
    let _ = module.evaluate(tc_scope);
    trace!(
      "|init_builtin_modules| ModuleMap evaluated {:?}, status {:?}",
      filename,
      module.get_status()
    );

    if module.get_status() == v8::ModuleStatus::Errored {
      let exception = module.get_exception();
      let exception = JsError::from_v8_exception(tc_scope, exception, None);
      error!(
        "Failed to evaluate builtin modules: {filename}, error: {exception:?}"
      );
      // Exit process!
      std::process::exit(1);
    }
  }
}

/// Javascript runtime.
pub mod boost {
  use super::*;

  #[derive(Debug, Default, Clone)]
  pub struct JsRuntimeOptions {
    // // The seed used in Math.random() method.
    // pub seed: Option<i64>,
    // // Reloads every URL import.
    // pub reload: bool,
    // The main entry point for the program.
    pub root: Option<String>,
    // Holds user defined import maps for module loading.
    pub import_map: Option<ImportMap>,
    // // The numbers of threads used by the thread-pool.
    // pub num_threads: Option<usize>,
    // Indicates if we're running JavaScript tests.
    pub test_mode: bool,
  }

  #[derive(rsvim_macro::RcRefCellPtr)]
  /// The state of js runtime.
  pub struct JsRuntimeState {
    /// A sand-boxed execution context with its own set of built-in objects and functions.
    pub context: v8::Global<v8::Context>,
    /// Holds information about resolved ES modules.
    pub module_map: ModuleMap,
    /// Pending timers.
    pub pending_timers: FoldMap<TimerId, TimerCallback>,
    /// Pending tasks.
    pub pending_tasks: FoldMap<TaskId, TaskCallback>,
    /// Holds JS pending futures scheduled by the event-loop.
    pub pending_futures: Vec<Box<dyn JsFuture>>,
    /// Indicates the start time of the process.
    pub startup_moment: Instant,
    /// Specifies the timestamp which the current process began in Unix time.
    pub time_origin: u128,
    // /// Holds callbacks scheduled by nextTick.
    // pub next_tick_queue: NextTickQueue,
    /// Stores and manages uncaught exceptions.
    pub exceptions: ExceptionState,
    /// Runtime options.
    pub options: JsRuntimeOptions,
    // /// Tracks wake event for current loop iteration.
    // pub wake_event_queued: bool,

    // Data Access for RSVIM {
    pub master_tx: UnboundedSender<MasterMessage>,
    pub jsrt_rx: UnboundedReceiver<JsMessage>,
    pub cli_opts: CliOptions,
    pub tree: TreeArc,
    pub buffer_manager: BufferManagerArc,
    pub cmdline_text: CmdlineTextArc,
    pub syntax_manager: SyntaxManagerArc,
    pub colorscheme_manager: ColorSchemeManagerArc,
    pub command_manager: CommandManagerArc,
    pub resource_table: ResourceTableArc,
    // Data Access for RSVIM }
  }

  /// Javascript runtime.
  ///
  /// There are 3 most important concepts:
  ///
  /// - Isolate
  /// - Context
  /// - Handle Scope
  ///
  /// For more details, please see: <https://v8.dev/docs/embed>.
  pub struct JsRuntime {
    /// V8 isolate.
    pub isolate: v8::OwnedIsolate,

    /// The state of the runtime.
    #[allow(unused)]
    pub state: JsRuntimeStateRc,
  }

  impl std::fmt::Debug for JsRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
      write!(f, "JsRuntime")
    }
  }

  impl JsRuntime {
    /// Creates a new JsRuntime.
    pub fn new(
      options: JsRuntimeOptions,
      startup_moment: Instant,
      time_origin: u128,
      master_tx: UnboundedSender<MasterMessage>,
      jsrt_rx: UnboundedReceiver<JsMessage>,
      cli_opts: CliOptions,
      tree: TreeArc,
      buffer_manager: BufferManagerArc,
      cmdline_text: CmdlineTextArc,
      syntax_manager: SyntaxManagerArc,
      colorscheme_manager: ColorSchemeManagerArc,
      command_manager: CommandManagerArc,
      resource_table: ResourceTableArc,
    ) -> Self {
      // Fire up the v8 engine.
      init_v8_platform();

      let mut isolate = v8::Isolate::new(v8::CreateParams::default());

      init_v8_isolate(&mut isolate);

      let context: v8::Global<v8::Context> = {
        v8::scope!(scope, &mut isolate);
        let context = binding::create_new_context(scope);

        // let module_handles = get_context_data(scope, context);
        v8::Global::new(scope, context)
      };

      // Store state inside the v8 isolate slot.
      // https://v8docs.nodesource.com/node-4.8/d5/dda/classv8_1_1_isolate.html#a7acadfe7965997e9c386a05f098fbe36
      let state = JsRuntimeState::to_rc(JsRuntimeState {
        context,
        module_map: ModuleMap::new(),
        pending_timers: FoldMap::new(),
        pending_tasks: FoldMap::new(),
        pending_futures: vec![],
        startup_moment,
        time_origin,
        // next_tick_queue: Vec::new(),
        exceptions: ExceptionState::new(),
        options,
        // wake_event_queued: false,
        master_tx,
        jsrt_rx,
        cli_opts,
        tree,
        buffer_manager,
        cmdline_text,
        syntax_manager,
        colorscheme_manager,
        command_manager,
        resource_table,
      });

      isolate.set_slot(state.clone());

      let mut runtime = JsRuntime {
        isolate,
        // event_loop,
        state,
        // inspector,
      };

      // Load, compile and evaluate all built-in modules.
      runtime.with_scope(init_builtin_modules);

      runtime
    }

    fn with_scope<F>(&mut self, func: F)
    where
      F: FnOnce(&mut v8::PinScope),
    {
      let context = self.context();
      v8::scope_with_context!(scope, &mut self.isolate, context);
      func(scope);
    }

    /// Executes javascript source code as ES module, i.e. ECMA standard.
    pub fn execute_module(&mut self, filename: &str, source: Option<&str>) {
      // Get a reference to v8's scope.
      self.with_scope(|scope| execute_module(scope, filename, source));
    }

    /// Runs a single tick of the event-loop.
    pub fn tick_event_loop(&mut self) {
      self.with_scope(run_next_tick_callbacks);
      // self.event_loop.tick();
      self.run_pending_futures();

      trace!(
        "|JsRuntime::tick_event_loop| has_promise_rejections:{:?}, has_pending_background_tasks:{:?}",
        self.has_promise_rejections(),
        self.isolate.has_pending_background_tasks(),
      );
      if self.has_promise_rejections()
        || self.isolate.has_pending_background_tasks()
      {
        chan::send_to_master(
          self.get_state().borrow().master_tx.clone(),
          MasterMessage::TickAgainReq,
        );
      }
    }

    // /// Polls the inspector for new devtools messages.
    // pub fn poll_inspect_session(&mut self) {
    //   if let Some(inspector) = self.inspector.as_mut() {
    //     inspector.borrow_mut().poll_session();
    //   }
    // }

    /// Runs pending javascript tasks which have received results from master.
    fn run_pending_futures(&mut self) {
      // Get a handle-scope and a reference to the runtime's state.
      let context = self.context();
      v8::scope_with_context!(scope, &mut self.isolate, context);
      let state_rc = Self::state(scope);

      // Drain all pending messages
      let mut messages: Vec<JsMessage> = vec![];
      {
        let mut state = state_rc.borrow_mut();
        while let Ok(msg) = state.jsrt_rx.try_recv() {
          messages.push(msg);
        }
        // Drop(state);
      }

      macro_rules! process_message {
        ($variant:ident, $resp:expr) => {{
          trace!("Recv {}:{:?}", stringify!($variant), $resp.task_id);
          debug_assert!(
            state_rc.borrow().pending_tasks.contains_key(&$resp.task_id)
          );
          let mut cb = state_rc
            .borrow_mut()
            .pending_tasks
            .remove(&$resp.task_id)
            .unwrap();
          cb($resp.maybe_result);
        }};
      }

      for msg in messages {
        match msg {
          JsMessage::TimeoutResp(resp) => {
            trace!("Recv TimeResp:{:?}", resp.timer_id);
            let maybe_timer_cb =
              state_rc.borrow_mut().pending_timers.remove(&resp.timer_id);
            if let Some(mut timer_cb) = maybe_timer_cb {
              timer_cb();
              if resp.repeated {
                let mut state = state_rc.borrow_mut();
                pending::create_timer(
                  &mut state,
                  resp.timer_id,
                  resp.delay,
                  resp.repeated,
                  timer_cb,
                );
              }
            }
            // Otherwise the 'timer_cb' is already been removed by the
            // `clear_timeout` API.
          }
          JsMessage::ExCommandReq(req) => {
            trace!("Recv ExCommandReq:{:?}", req.payload);
            let mut state = state_rc.borrow_mut();
            let command_manager = state.command_manager.clone();
            let command_manager = lock!(command_manager);
            if let Some(command_cb) = command_manager.parse(&req) {
              state.pending_futures.push(Box::new(command_cb));
            } else {
              // Print error message
              report_js_error(&state, TheErr::CommandNotFound(req.payload));
            }
          }
          JsMessage::TickAgainResp => trace!("Recv TickAgainResp"),
          JsMessage::FsOpenResp(resp) => process_message!(FsOpenResp, resp),
          JsMessage::FsReadResp(resp) => process_message!(FsReadResp, resp),
          JsMessage::FsWriteResp(resp) => process_message!(FsWriteResp, resp),
          JsMessage::FsReadDirResp(resp) => {
            process_message!(FsReadDirResp, resp)
          }
          JsMessage::FsReadDirNextResp(resp) => {
            process_message!(FsReadDirNextResp, resp)
          }
          JsMessage::FsReadFileResp(resp) => {
            process_message!(FsReadFileResp, resp)
          }
          JsMessage::FsReadTextFileResp(resp) => {
            process_message!(FsReadTextFileResp, resp)
          }
          JsMessage::FsStatResp(resp) => process_message!(FsStatResp, resp),
          JsMessage::FsSymlinkResp(resp) => {
            process_message!(FsSymlinkResp, resp)
          }
          JsMessage::FsLinkResp(resp) => process_message!(FsLinkResp, resp),
          JsMessage::FsMkdirResp(resp) => process_message!(FsMkdirResp, resp),
          JsMessage::LoadTreeSitterParserResp(resp) => {
            process_message!(LoadTreeSitterParserResp, resp)
          }
          JsMessage::ReadTextFromChildProcessStdioResp(resp) => {
            process_message!(ReadTextFromChildProcessStdioResp, resp)
          }
          JsMessage::WaitChildProcessResp(resp) => {
            process_message!(WaitChildProcessResp, resp)
          }
        }
      }

      let futures: Vec<Box<dyn JsFuture>> =
        std::mem::take(&mut state_rc.borrow_mut().pending_futures);
      for mut fut in futures {
        fut.run(scope);
        if let Some(exception) = check_exceptions(scope) {
          trace!("Got exceptions when running pending futures: {exception:?}");
          let state = state_rc.borrow();
          report_js_error(&state, TheErr::JsError(Box::new(exception)));
        }
        run_next_tick_callbacks(scope);
      }
    }
  }

  impl JsRuntime {
    /// Returns if unhandled promise rejections where caught.
    pub fn has_promise_rejections(&mut self) -> bool {
      self.get_state().borrow().exceptions.has_promise_rejection()
    }

    // /// Returns if we have scheduled any next-tick callbacks.
    // pub fn has_next_tick_callbacks(&mut self) -> bool {
    //   !self.get_state().borrow().next_tick_queue.is_empty()
    // }
  }

  // State management specific methods.
  // https://github.com/lmt-swallow/puppy-browser/blob/main/src/javascript/runtime.rs
  impl JsRuntime {
    /// Returns the runtime state stored in the given isolate.
    pub fn state(isolate: &v8::Isolate) -> JsRuntimeStateRc {
      isolate.get_slot::<JsRuntimeStateRc>().unwrap().clone()
    }

    /// Returns the runtime's state.
    pub fn get_state(&self) -> JsRuntimeStateRc {
      Self::state(&self.isolate)
    }

    /// Returns a global context created for the runtime.
    /// See: <https://v8docs.nodesource.com/node-0.8/df/d69/classv8_1_1_context.html>.
    pub fn context(&mut self) -> v8::Global<v8::Context> {
      let state_rc = self.get_state();
      let state = state_rc.borrow();
      state.context.clone()
    }

    // /// Returns the inspector created for the runtime.
    // pub fn inspector(&mut self) -> Option<Rc<RefCell<JsRuntimeInspector>>> {
    //   self.inspector.as_ref().cloned()
    // }
  }
}

pub fn execute_module<'s, 'b>(
  scope: &mut v8::PinScope<'s, 'b>,
  filename: &str,
  source: Option<&str>,
) {
  // trace!("Execute module, filename:{filename:?}, source:{source:?}");

  let state_rc = JsRuntime::state(scope);

  // The following code allows the runtime to execute code with no valid
  // location passed as parameter as an ES module.
  let path = if source.is_some() {
    filename.to_string()
  } else {
    let base = PATH_CONFIG.config_home().to_path_buf();
    match resolve_import(&base.to_string_lossy(), filename, None) {
      Ok(specifier) => specifier,
      Err(e) => {
        // Returns the error directly.
        // trace!("Failed to resolve module path, filename:{filename:?}");
        let state = state_rc.borrow_mut();
        report_js_error(&state, e);
        return;
      }
    }
  };
  // trace!("Module path resolved, filename:{filename:?}({path:?})");

  v8::tc_scope!(let tc_scope, scope);

  let module = match fetch_module_tree(tc_scope, filename, source) {
    Some(module) => module,
    None => {
      // trace!(
      //   "Failed to fetch module, filename:{filename:?}({path:?}), exception:{exception:?}"
      // );
      assert!(tc_scope.has_caught());
      let exception = tc_scope.exception().unwrap();
      let exception = JsError::from_v8_exception(tc_scope, exception, None);
      let state = state_rc.borrow_mut();
      report_js_error(&state, TheErr::JsError(Box::new(exception)));
      return;
    }
  };

  if module
    .instantiate_module(tc_scope, module_resolve_cb)
    .is_none()
  {
    // trace!(
    //   "Failed to initialize module, filename:{filename:?}({path:?}), exception:{exception:?}"
    // );
    assert!(tc_scope.has_caught());
    let exception = tc_scope.exception().unwrap();
    let exception = JsError::from_v8_exception(tc_scope, exception, None);
    let state = state_rc.borrow_mut();
    report_js_error(&state, TheErr::JsError(Box::new(exception)));
    return;
  }

  let _ = module.evaluate(tc_scope);
  trace!(
    "|execute_module| ModuleMap evaluated {:?}, status {:?}",
    path,
    module.get_status()
  );

  if module.get_status() == v8::ModuleStatus::Errored {
    let exception = module.get_exception();
    let exception = v8::Global::new(tc_scope, exception);

    let state_rc = JsRuntime::state(tc_scope);
    let mut state = state_rc.borrow_mut();

    state.exceptions.capture_exception(exception.clone());
    state.exceptions.remove_promise_rejection_entry(&exception);

    drop(state);

    if let Some(error) = check_exceptions(tc_scope) {
      let state = state_rc.borrow();
      report_js_error(&state, TheErr::JsError(Box::new(error)));
    }

    // trace!(
    //   "Failed to evaluate module, filename:{filename:?}({path:?}), exception:{exception:?}"
    // );
  }
}

/// Runs callbacks stored in the next-tick queue.
fn run_next_tick_callbacks(scope: &mut v8::PinScope) {
  // let state_rc = JsRuntime::state(scope);
  // let callbacks: NextTickQueue = state_rc.borrow_mut().next_tick_queue.drain(..).collect();

  // let undefined = v8::undefined(scope);
  v8::tc_scope!(let tc_scope, scope);
  //
  // for (cb, params) in callbacks {
  //   // Create a local handle for the callback and its parameters.
  //   let cb = v8::Local::new(tc_scope, cb);
  //   let args: Vec<v8::Local<v8::Value>> = params
  //     .iter()
  //     .map(|arg| v8::Local::new(tc_scope, arg))
  //     .collect();
  //
  //   cb.call(tc_scope, undefined.into(), &args);
  //
  //   // On exception, report it and handle the error.
  //   if tc_scope.has_caught() {
  //     let exception = tc_scope.exception().unwrap();
  //     let exception = v8::Global::new(tc_scope, exception);
  //     let mut state = state_rc.borrow_mut();
  //     state.exceptions.capture_exception(exception);
  //
  //     drop(state);
  //
  //     // Check for uncaught errors (capture callbacks might be in place).
  //     if let Some(error) = check_exceptions(tc_scope) {
  //       report_and_exit(error);
  //     }
  //   }
  // }

  tc_scope.perform_microtask_checkpoint();
}

// Returns an error if an uncaught exception or unhandled rejection has been captured.
pub fn check_exceptions(scope: &mut v8::PinScope) -> Option<JsError> {
  let state_rc = JsRuntime::state(scope);
  let maybe_exception = state_rc.borrow_mut().exceptions.exception.take();

  // Check for uncaught exceptions first.
  if let Some(exception) = maybe_exception {
    let state = state_rc.borrow();
    let exception = v8::Local::new(scope, exception);
    if let Some(callback) = state.exceptions.uncaught_exception_cb.as_ref() {
      let callback = v8::Local::new(scope, callback);
      let undefined = v8::undefined(scope).into();
      let origin = v8::String::new(scope, "uncaughtException").unwrap();
      v8::tc_scope!(let tc_scope, scope);
      drop(state);

      callback.call(tc_scope, undefined, &[exception, origin.into()]);

      // Note: To avoid infinite recursion with these hooks, if this
      // function throws, return it as error.
      if tc_scope.has_caught() {
        let exception = tc_scope.exception().unwrap();
        let exception = v8::Local::new(tc_scope, exception);
        let error = JsError::from_v8_exception(tc_scope, exception, None);
        return Some(error);
      }

      return None;
    }

    let error = JsError::from_v8_exception(scope, exception, None);
    return Some(error);
  }

  let promise_rejections: Vec<PromiseRejectionEntry> =
    std::mem::take(&mut state_rc.borrow_mut().exceptions.promise_rejections);

  // Then, check for unhandled rejections.
  for (promise, exception) in promise_rejections.iter() {
    let state = state_rc.borrow_mut();
    let promise = v8::Local::new(scope, promise);
    let exception = v8::Local::new(scope, exception);

    // If the `unhandled_rejection_cb` is set, invoke it to handle the promise rejection.
    if let Some(callback) = state.exceptions.unhandled_rejection_cb.as_ref() {
      let callback = v8::Local::new(scope, callback);
      let undefined = v8::undefined(scope).into();
      v8::tc_scope!(let tc_scope, scope);
      drop(state);

      callback.call(tc_scope, undefined, &[exception, promise.into()]);

      // Note: To avoid infinite recursion with these hooks, if this
      // function throws, return it as error.
      if tc_scope.has_caught() {
        let exception = tc_scope.exception().unwrap();
        let exception = v8::Local::new(tc_scope, exception);
        let error = JsError::from_v8_exception(tc_scope, exception, None);
        return Some(error);
      }

      continue;
    }

    // If the `uncaught_exception_cb` is set, invoke it to handle the promise rejection.
    if let Some(callback) = state.exceptions.uncaught_exception_cb.as_ref() {
      let callback = v8::Local::new(scope, callback);
      let undefined = v8::undefined(scope).into();
      let origin = v8::String::new(scope, "unhandledRejection").unwrap();
      v8::tc_scope!(let tc_scope, scope);
      drop(state);

      callback.call(tc_scope, undefined, &[exception, origin.into()]);

      // Note: To avoid infinite recursion with these hooks, if this
      // function throws, return it as error.
      if tc_scope.has_caught() {
        let exception = tc_scope.exception().unwrap();
        let exception = v8::Local::new(tc_scope, exception);
        let error = JsError::from_v8_exception(tc_scope, exception, None);
        return Some(error);
      }

      continue;
    }

    let prefix = Some("(in promise) ");
    let error = JsError::from_v8_exception(scope, exception, prefix);

    return Some(error);
  }

  None
}
