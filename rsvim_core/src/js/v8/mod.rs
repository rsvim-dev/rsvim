//! JavaScript runtime.
//!
//! References:
//! - Embed V8: <https://v8.dev/docs/embed>.
//! - Roll your own JavaScript runtime: <https://deno.com/blog/roll-your-own-javascript-runtime>.
//! - Learning V8 Tutorial: <https://github.com/danbev/learning-v8>.
//! - A hobby runtime for javascript/typescript: <https://github.com/aalykiot/dune>.
//! - V8 API Reference: <https://v8docs.nodesource.com/node-24.1/index.html>.

pub mod binding;
pub mod converter;
pub mod err;
pub mod exception;
pub mod hook;
pub mod loader;
pub mod module;
pub mod pending;
pub mod resource;

#[cfg(test)]
mod converter_tests;
#[cfg(test)]
mod module_tests;

use crate::cfg::path_cfg::PATH_CONFIG;
use crate::chan::JsMessage;
use crate::chan::MasterMessage;
use crate::evloop::Editor;
use crate::js::TaskId;
use crate::js::TimerId;
use crate::prelude::*;
pub use boost::*;
use err::JsError;
use err::report_js_error;
use exception::ExceptionState;
use exception::PromiseRejectionEntry;
use hook::module_resolve_cb;
use module::ImportMap;
use module::ModuleMap;
use module::fetch_module_tree;
use module::resolve_import;
use pending::TaskCallback;
use pending::TimerCallback;
use std::sync::Once;
use tokio::sync::mpsc::UnboundedSender;

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
  // See: [`checkpoint`].
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

/// Loads, compiles and evaluates all builtin modules.
///
/// The entry modules are embedded here, and their dependencies are the core
/// modules embedded in [`CORE_MODULES`], which are resolved and loaded by the
/// [`CoreModuleLoader`]. Note that [`fetch_module_tree`] takes care of the
/// dependencies, i.e. fetching them recursively, thus only the entry modules
/// are listed here.
///
/// [`CORE_MODULES`]: module::CORE_MODULES
/// [`CoreModuleLoader`]: loader::CoreModuleLoader
fn init_builtin_modules(scope: &mut v8::PinScope) {
  static BUILTIN_MODULES: [(/* filename */ &str, /* source */ &str); 1] = [(
    "00__web.js",
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/js/00__web.min.js")),
  )];

  for module in BUILTIN_MODULES.iter() {
    let filename = module.0;
    let source = module.1;

    v8::tc_scope!(let tc_scope, scope);

    let module = fetch_module_tree(tc_scope, filename, Some(source)).unwrap();
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
  /// The v8 isolate's in-process state, i.e. the kernel of the js runtime.
  ///
  /// Only things that v8 callbacks can and only can reach from here belong
  /// here. `Rsvim`-level fields (writer / canvas / state machine / the receive
  /// ends of the channels) are **forbidden** to be put in.
  pub struct JsKernel {
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
    // /// Holds callbacks scheduled by nextTick.
    // pub next_tick_queue: NextTickQueue,
    /// Stores and manages uncaught exceptions.
    pub exceptions: ExceptionState,
    /// Runtime options.
    pub options: JsRuntimeOptions,

    // Data Access for RSVIM {
    pub master_tx: UnboundedSender<MasterMessage>,
    /// Editor's shared data (the same `Arc`s as `Rsvim.editor`).
    pub editor: Editor,
    // Data Access for RSVIM }
  }

  /// Creates a new v8 isolate, together with its [`JsKernel`].
  ///
  /// The kernel is stored inside the isolate's slot, so that v8 callbacks can
  /// reach it with [`get_state`]. The caller (i.e.
  /// [`Rsvim`](crate::evloop::Rsvim)) is responsible for keeping both alive.
  pub fn new_isolate(
    options: JsRuntimeOptions,
    master_tx: UnboundedSender<MasterMessage>,
    editor: Editor,
  ) -> (v8::OwnedIsolate, JsKernelRc) {
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
    let kernel = JsKernel::to_rc(JsKernel {
      context,
      module_map: ModuleMap::new(),
      pending_timers: FoldMap::new(),
      pending_tasks: FoldMap::new(),
      pending_futures: vec![],
      // next_tick_queue: Vec::new(),
      exceptions: ExceptionState::new(),
      options,
      master_tx,
      editor,
    });

    isolate.set_slot(kernel.clone());

    // Load, compile and evaluate all built-in modules.
    {
      let context = kernel.borrow().context.clone();
      v8::scope_with_context!(scope, &mut isolate, context);
      init_builtin_modules(scope);
    }

    (isolate, kernel)
  }

  /// Returns the runtime state stored in the given isolate's slot.
  ///
  /// Every callback runs with a scope but no other way back to the kernel, so
  /// this is the single entry point from v8 into rsvim's state. The isolate is
  /// owned by [`Rsvim`](crate::evloop::Rsvim) and the slot is set in
  /// [`new_isolate`], long before any callback can run.
  pub fn get_state(isolate: &v8::Isolate) -> JsKernelRc {
    isolate
      .get_slot::<JsKernelRc>()
      .expect("v8 isolate slot not initialized")
      .clone()
  }

  /// Handles a single message that has come back from the master (i.e. the
  /// completion of a task that was previously spawned into the tokio runtime).
  ///
  /// The `pending_tasks` entry is looked up with `if let Some` rather than
  /// `debug_assert!` + `unwrap()`: the message can come from a task whose
  /// future was already dropped, and a `panic!` in release builds is not an
  /// acceptable answer to that.
  pub(crate) fn process_js_message(
    kernel: &JsKernelRc,
    _scope: &mut v8::PinScope,
    msg: JsMessage,
  ) {
    macro_rules! process_message {
      ($variant:ident, $resp:expr) => {{
        trace!("Recv {}:{:?}", stringify!($variant), $resp.task_id);
        let maybe_cb = kernel.borrow_mut().pending_tasks.remove(&$resp.task_id);
        match maybe_cb {
          Some(mut cb) => cb($resp.maybe_result),
          None => {
            warn!(
              "Received {} for an unknown task id:{:?}, the task future is \
               probably already dropped, ignore it",
              stringify!($variant),
              $resp.task_id,
            );
          }
        }
      }};
    }

    match msg {
      JsMessage::TimeoutResp(resp) => {
        trace!("Recv TimeResp:{:?}", resp.timer_id);
        let maybe_timer_cb =
          kernel.borrow_mut().pending_timers.remove(&resp.timer_id);
        if let Some(mut timer_cb) = maybe_timer_cb {
          timer_cb();
          if resp.repeated {
            let mut state = kernel.borrow_mut();
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
        let mut state = kernel.borrow_mut();
        let command_manager = state.editor.command_manager.clone();
        let command_manager = lock!(command_manager);
        if let Some(command_cb) = command_manager.parse(&req) {
          state.pending_futures.push(Box::new(command_cb));
        } else {
          // Print error message
          report_js_error(&state, TheErr::CommandNotFound(req.payload));
        }
      }
      JsMessage::FsOpenResp(resp) => process_message!(FsOpenResp, resp),
      JsMessage::FsReadResp(resp) => process_message!(FsReadResp, resp),
      JsMessage::FsWriteResp(resp) => process_message!(FsWriteResp, resp),
      JsMessage::FsReadDirResp(resp) => process_message!(FsReadDirResp, resp),
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
      JsMessage::FsSymlinkResp(resp) => process_message!(FsSymlinkResp, resp),
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
}

pub fn execute_module<'s, 'b>(
  scope: &mut v8::PinScope<'s, 'b>,
  filename: &str,
  source: Option<&str>,
) {
  // trace!("Execute module, filename:{filename:?}, source:{source:?}");

  let state_rc = get_state(scope);

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

  let maybe_evaluated = module.evaluate(tc_scope);
  let status = module.get_status();
  trace!(
    "|execute_module| ModuleMap evaluated {:?}, status {:?}",
    path, status
  );

  // A module that is still `Evaluating` after `evaluate()` returned is waiting
  // on a top-level await. Give it an explicit state instead of leaving it
  // silently stuck: the pending promise will be driven by the microtask queue,
  // and if it never settles we report it at shutdown.
  if maybe_evaluated.is_none() && status == v8::ModuleStatus::Evaluating {
    trace!(
      "|execute_module| Module {:?} is pending on a top-level await",
      path
    );
  }

  if status == v8::ModuleStatus::Errored {
    let exception = module.get_exception();
    let exception = v8::Global::new(tc_scope, exception);

    let state_rc = get_state(tc_scope);
    let mut state = state_rc.borrow_mut();

    state.exceptions.capture_exception(exception.clone());
    state.exceptions.remove_promise_rejection_entry(&exception);

    drop(state);

    for error in check_exceptions(tc_scope) {
      let state = state_rc.borrow();
      report_js_error(&state, TheErr::JsError(Box::new(error)));
    }

    // trace!(
    //   "Failed to evaluate module, filename:{filename:?}({path:?}), exception:{exception:?}"
    // );
  }
}

/// Performs a microtask checkpoint.
///
/// The isolate is created with the `explicit` microtasks policy
/// (`js.rs:85`), so the embedder is responsible for draining the microtask
/// queue: this must be called after every step that may have queued jobs.
pub(crate) fn checkpoint(scope: &mut v8::PinScope) {
  // NOTE: the next-tick queue was never wired up, `run_next_tick_callbacks`
  // only ever performed the microtask checkpoint. The dead code was removed;
  // if a next-tick queue is ever added back, it belongs here, before the
  // checkpoint.
  v8::tc_scope!(let tc_scope, scope);
  tc_scope.perform_microtask_checkpoint();
}

/// Returns the errors of every uncaught exception and unhandled rejection that
/// has been captured.
///
/// Note that this returns **all** of them: the previous `Option<JsError>`
/// signature took the whole `promise_rejections` vector but then returned only
/// the first entry, silently dropping the rest.
pub fn check_exceptions(scope: &mut v8::PinScope) -> Vec<JsError> {
  let mut errors: Vec<JsError> = vec![];

  let state_rc = get_state(scope);
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
        errors.push(error);
      }

      return errors;
    }

    let error = JsError::from_v8_exception(scope, exception, None);
    errors.push(error);
    return errors;
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
        errors.push(error);
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
        errors.push(error);
      }

      continue;
    }

    let prefix = Some("(in promise) ");
    let error = JsError::from_v8_exception(scope, exception, prefix);

    errors.push(error);
  }

  errors
}
