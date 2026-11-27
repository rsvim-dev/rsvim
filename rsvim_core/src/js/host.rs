//! The javascript host: a synchronous engine behind a single entrance.
//!
//! [`JsHost`] owns the QuickJS runtime and context, and nothing else is allowed
//! to run javascript. Every way in -- evaluating a module, running an `:Ex`
//! command, firing a timer, settling a promise for a finished operation -- goes
//! through [`JsHost::enter`], and `enter` is where the rules that used to be
//! scattered over the call sites live:
//!
//! 1. the call runs;
//! 2. the job queue (microtasks, promise reactions) is drained, so a callback's
//!    microtasks run before the next callback starts -- two timers due at the
//!    same instant produce `t1, m1, t2`, like Node, Deno and browsers;
//! 3. unhandled promise rejections are reported, once each, *after* the drain,
//!    so a `catch` attached in the same turn cancels the report;
//! 4. the host is marked dirty, because javascript may have changed the editor.
//!
//! The engine never schedules anything by itself. Asynchronous work is a tokio
//! task that reports back through [`crate::js::ops`], and timers live in a table
//! the event loop waits on ([`JsHost::poll_timer`]). That is why the loop can
//! see everything javascript does: there is no scheduler inside the engine to
//! make progress behind its back.

use crate::chan::JsMessage;
use crate::chan::MasterMessage;
use crate::evloop::Editor;
use crate::js::binding;
use crate::js::err::JsError;
use crate::js::err::report_js_error;
use crate::js::ops::Completion;
use crate::js::ops::OpTable;
use crate::js::ops::OpTableHandle;
use crate::js::ops::PendingOp;
use crate::js::runtime::JsKernel;
use crate::js::runtime::JsKernelRc;
use crate::js::runtime::JsRuntimeOptions;
use crate::js::runtime::capture_exception;
use crate::js::runtime::get_state;
use crate::js::runtime::set_state;
use crate::js::runtime::to_io_error;
use crate::js::timers::DueTimer;
use crate::prelude::*;
use rquickjs::Context;
use rquickjs::Ctx;
use rquickjs::Exception;
use rquickjs::Module;
use rquickjs::Persistent;
use rquickjs::Runtime;
use rquickjs::Value;
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use std::task::Poll;
use std::time::Duration;
use std::time::Instant;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::task::TaskTracker;

/// A rejected promise that nobody was listening to, yet.
///
/// Only the two values are kept. Turning the reason into a report is work, and
/// most rejections are handled a moment later, so it is done in
/// [`JsHost::flush_rejections`], for the ones that are still here.
struct Rejection {
  /// Identity, so a handler attached later can cancel the report.
  promise: Persistent<Value<'static>>,
  reason: Persistent<Value<'static>>,
}

/// The rejections nobody has handled so far.
#[derive(Default)]
struct Rejections(RefCell<Vec<Rejection>>);

impl Rejections {
  fn record<'js>(
    &self,
    ctx: &Ctx<'js>,
    promise: Value<'js>,
    reason: Value<'js>,
  ) {
    self.0.borrow_mut().push(Rejection {
      promise: Persistent::save(ctx, promise),
      reason: Persistent::save(ctx, reason),
    });
  }

  /// Stops tracking `promise`: somebody handled it, or reports it themselves.
  fn forget<'js>(&self, ctx: &Ctx<'js>, promise: &Value<'js>) {
    self.0.borrow_mut().retain(|r| {
      r.promise
        .clone()
        .restore(ctx)
        .map(|saved| saved != *promise)
        .unwrap_or(true)
    });
  }

  fn take(&self) -> Vec<Rejection> {
    std::mem::take(&mut *self.0.borrow_mut())
  }

  fn clear(&self) {
    self.0.borrow_mut().clear();
  }
}

/// The javascript runtime, its context, and the tables that hang off them.
pub struct JsHost {
  runtime: Runtime,
  context: Context,
  kernel: JsKernelRc,
  editor: Editor,
  ops: Rc<OpTable>,
  rejections: Rc<Rejections>,
  /// Deadline for the code that is running right now, see
  /// [`JsRuntimeOptions::exec_timeout`].
  deadline: Rc<Cell<Option<Instant>>>,
  exec_timeout: Option<Duration>,
  dirty: bool,
}

impl JsHost {
  /// Builds the engine: one runtime, one context, one kernel.
  ///
  /// `jsrt_tx` is where finished operations are reported; `blocked` and
  /// `detached` are the trackers the event loop waits on / closes on exit.
  pub fn new(
    options: JsRuntimeOptions,
    master_tx: UnboundedSender<MasterMessage>,
    jsrt_tx: UnboundedSender<JsMessage>,
    editor: Editor,
    blocked: TaskTracker,
    detached: TaskTracker,
  ) -> IoResult<Self> {
    let runtime = Runtime::new().map_err(to_io_error)?;
    let context = Context::full(&runtime).map_err(to_io_error)?;

    // The engine walks the import graph itself; these two callbacks are how it
    // asks for a specifier to be resolved and for the source behind it. Core
    // modules (`rsvim:cmd` and friends) come from the tuple's first member, plain
    // files from the second: the tuple impl tries them in order and only falls
    // through on a resolving/loading failure.
    runtime.set_loader(
      (
        crate::js::loader::CoreResolver,
        crate::js::fs_loader::FsModuleLoader::new(),
      ),
      (
        crate::js::loader::CoreLoader,
        crate::js::fs_loader::FsModuleLoader::new(),
      ),
    );

    let exec_timeout = options.exec_timeout;
    let kernel = JsKernel::to_rc(JsKernel {
      options,
      master_tx,
      editor: editor.clone(),
    });
    let ops = Rc::new(OpTable::new(jsrt_tx, blocked, detached));
    let rejections: Rc<Rejections> = Rc::default();

    // QuickJS reports a rejection twice when somebody handles it late: first with
    // `handled == false` when it is rejected with no handler, then with `true`
    // when a handler is attached. So the tracker only keeps a record, and
    // [`JsHost::flush_rejections`] reports whatever is still on it once the job
    // queue has been drained.
    runtime.set_host_promise_rejection_tracker(Some(Box::new({
      let rejections = rejections.clone();
      move |ctx, promise, reason, handled| {
        if handled {
          rejections.forget(&ctx, &promise);
        } else {
          rejections.record(&ctx, promise, reason);
        }
      }
    })));

    // The watchdog: QuickJS calls this regularly while it interprets, and a
    // `true` raises an exception javascript cannot catch. Without a timeout
    // there is nothing for it to do, so it is not installed at all.
    let deadline: Rc<Cell<Option<Instant>>> = Rc::default();
    if exec_timeout.is_some() {
      runtime.set_interrupt_handler(Some(Box::new({
        let deadline = deadline.clone();
        move || deadline.get().is_some_and(|d| Instant::now() > d)
      })));
    }

    let mut host = Self {
      runtime,
      context,
      kernel: kernel.clone(),
      editor,
      ops: ops.clone(),
      rejections,
      deadline,
      exec_timeout,
      dirty: false,
    };

    // Bootstrapping is a javascript entry like any other: the web globals are a
    // module, and it can leave jobs behind.
    host
      .enter(|ctx| -> rquickjs::Result<()> {
        set_state(&ctx, kernel)?;
        ctx
          .store_userdata(OpTableHandle(ops))
          .map_err(|_| Exception::throw_message(&ctx, "op table is taken"))?;
        binding::install_bindings(&ctx)?;
        binding::install_web_globals(&ctx)?;
        Ok(())
      })
      .map_err(to_io_error)?;
    host.dirty = false;
    Ok(host)
  }

  /// The kernel stored in the context.
  pub fn kernel(&self) -> &JsKernelRc {
    &self.kernel
  }

  /// The in-flight operations and armed timers.
  pub fn ops(&self) -> &Rc<OpTable> {
    &self.ops
  }

  /// Whether javascript ran since the last call, i.e. whether the editor may
  /// have changed and the terminal needs a repaint.
  pub fn take_dirty(&mut self) -> bool {
    std::mem::take(&mut self.dirty)
  }

  /// **The** entrance to javascript.
  ///
  /// Runs `f`, drains the job queue, reports the rejections nobody handled, and
  /// marks the host dirty. See the module documentation for why each of those
  /// belongs here and not at the call sites.
  pub fn enter<R>(&mut self, f: impl for<'js> FnOnce(Ctx<'js>) -> R) -> R {
    if let Some(limit) = self.exec_timeout {
      self.deadline.set(Some(Instant::now() + limit));
    }
    let result = self.context.with(f);
    self.drain_jobs();
    self.deadline.set(None);
    self.flush_rejections();
    self.dirty = true;
    result
  }

  /// Evaluates `filename` as an ES module.
  ///
  /// QuickJS resolves the whole import graph itself: [`Module::evaluate`] calls
  /// back into the loader for every `import`, so there is no tree to fetch
  /// beforehand.
  ///
  /// A module parked on a top-level await leaves its promise pending forever;
  /// that is not an error, it is the "still waiting" case the caller has to live
  /// with (rquickjs reports it as [`rquickjs::Error::WouldBlock`]).
  pub fn execute_module(
    &mut self,
    filename: &str,
    source: Option<&str>,
  ) -> TheResult<()> {
    let source: String = match source {
      Some(source) => source.to_string(),
      None => std::fs::read_to_string(filename)
        .map_err(|e| TheErr::LoadModuleFailed(filename.to_string(), e))?,
    };

    let rejections = self.rejections.clone();
    self.enter(|ctx| -> TheResult<()> {
      let promise = Module::evaluate(ctx.clone(), filename.to_string(), source)
        .map_err(|e| JsError::from_error(&ctx, e))?;

      match promise.finish::<()>() {
        Ok(()) => Ok(()),
        // The module is parked on a top-level await: not an error, just a
        // promise that will never settle.
        Err(rquickjs::Error::WouldBlock) => Ok(()),
        Err(e) => {
          // The caller reports this failure, so the rejection the failed module
          // left on the tracker would report it a second time.
          rejections.forget(&ctx, &promise.into_value());
          Err(JsError::from_error(&ctx, e).into())
        }
      }
    })
  }

  /// Settles the promise of a finished operation.
  pub fn complete(&mut self, completion: Completion) {
    let id = completion.id;
    let Some(PendingOp {
      resolve,
      reject,
      aux,
      reply,
      ..
    }) = self.ops.take(id)
    else {
      // The operation was dropped (cleared on the way out): nobody is waiting.
      trace!("|complete| op {:?} is gone", id);
      return;
    };
    let result = completion.result;

    self.enter(move |ctx| {
      let aux = aux.and_then(|saved| saved.restore(&ctx).ok());
      let (settle, value) = match reply(&ctx, aux, result) {
        Ok(value) => (resolve, value),
        Err(e) => (reject, rejection_value(&ctx, e)),
      };
      if let Ok(settle) = settle.restore(&ctx)
        && settle.call::<_, ()>((value,)).is_err()
      {
        ctx.catch();
      }
    });
  }

  /// Waits for the earliest armed timer, see [`crate::js::timers::Timers`].
  ///
  /// `Ready(None)` means no timer is armed.
  pub fn poll_timer(
    &self,
    cx: &mut std::task::Context<'_>,
  ) -> Poll<Option<DueTimer>> {
    self.ops.timers.borrow_mut().poll_due(cx)
  }

  /// Takes the earliest timer if it is due right now, without waiting.
  pub fn pop_due_timer(&self) -> Option<DueTimer> {
    self
      .ops
      .timers
      .borrow_mut()
      .pop_due(tokio::time::Instant::now())
  }

  /// Runs the callback of a timer that came due.
  pub fn run_timer(&mut self, timer: DueTimer) {
    trace!("|run_timer| timer_id:{:?}", timer.id);
    self.enter(move |ctx| {
      let Ok(callback) = timer.callback.restore(&ctx) else {
        return;
      };
      if callback.call::<_, ()>(()).is_ok() {
        return;
      }
      if let Some(err) = capture_exception(&ctx)
        && let Some(kernel) = get_state(&ctx)
      {
        report_js_error(&kernel.borrow().editor, err);
      }
    });
  }

  /// Runs queued jobs until the queue is empty.
  ///
  /// This is the checkpoint: it uses [`Runtime::execute_pending_job`], which
  /// hands back a job's exception instead of swallowing it like
  /// [`Ctx::execute_pending_job`] does.
  fn drain_jobs(&mut self) {
    loop {
      match self.runtime.execute_pending_job() {
        Ok(true) => {}
        Ok(false) => break,
        Err(exception) => {
          if let Some(err) = exception.0.with(|ctx| capture_exception(&ctx)) {
            report_js_error(&self.editor, err);
          }
        }
      }
    }
  }

  /// Reports the rejections that are still unhandled, once each.
  fn flush_rejections(&mut self) {
    let unhandled = self.rejections.take();
    if unhandled.is_empty() {
      return;
    }
    let reports: Vec<String> = self.context.with(|ctx| {
      unhandled
        .iter()
        .filter_map(|r| r.reason.clone().restore(&ctx).ok())
        .map(|reason| JsError::from_js_value(&ctx, reason).to_report(true))
        .collect()
    });
    for report in reports {
      crate::js::cmdline::send_cmdline_message(&self.editor, report);
    }
  }
}

/// What a promise is rejected with when building its value failed.
fn rejection_value<'js>(ctx: &Ctx<'js>, error: rquickjs::Error) -> Value<'js> {
  if !matches!(error, rquickjs::Error::Exception) {
    let _ = Exception::throw_message(ctx, &error.to_string());
  }
  ctx.catch()
}

impl Drop for JsHost {
  /// Ties the cleanup to ownership instead of to a code path.
  ///
  /// Every `Persistent` handle has to die before the runtime does, otherwise
  /// QuickJS aborts the process in `JS_FreeRuntime`. The handles live in three
  /// places -- the operation/timer tables, the rejection list, and the registered
  /// `:Ex` commands -- and none of them is reachable through a field the
  /// compiler could order for us, so this does it explicitly. It runs on every
  /// path out, including the early `?` returns that skip an orderly shutdown.
  fn drop(&mut self) {
    self.ops.clear();
    self.rejections.clear();
    lock!(self.editor.command_manager).clear();
  }
}
