//! Asynchronous work for javascript: the host-owned completion queue.
//!
//! An asynchronous binding does not hand a future to the engine. It creates a
//! promise, parks the two functions that settle it in [`OpTable`] under an
//! [`OpId`], and runs the work as a plain `Send` tokio task. When the task is
//! done it sends one [`Completion`] -- plain data, no javascript value -- down
//! the same channel the editor already listens on, and the event loop settles
//! the promise from the main thread, through [`crate::js::JsHost::enter`].
//!
//! This is the shape Deno (`PromiseId` + `completed_ops`) and Dune (`JsFuture`)
//! use, and it is what makes the three things that used to be easy to get wrong
//! impossible to forget:
//!
//! - the loop *sees* every completion, so it knows the editor may have changed;
//! - every completion is its own javascript entry, so the microtask queue is
//!   drained between two completions;
//! - an in-flight operation is a table entry, so it can be counted, dropped and
//!   (for the tokio side) aborted.
//!
//! Everything javascript-shaped (`Persistent`, `Ctx`) stays on the main thread.
//! Only the result of the work -- `T: Send` -- crosses.

use crate::chan::JsMessage;
use crate::js::timers::Timers;
use crate::prelude::*;
use rquickjs::Ctx;
use rquickjs::Exception;
use rquickjs::Function;
use rquickjs::JsLifetime;
use rquickjs::Persistent;
use rquickjs::Promise;
use rquickjs::Result as JsResult;
use rquickjs::Value;
use std::any::Any;
use std::cell::Cell;
use std::cell::RefCell;
use std::future::Future;
use std::rc::Rc;
use tokio::sync::mpsc::UnboundedSender;
use tokio_util::task::TaskTracker;

/// Identifies one in-flight operation.
pub type OpId = u64;

/// What the editor does with the operation when it quits.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum OpKind {
  /// Nobody would see the result once the loop is gone (a read, a `stat`).
  /// The task is tracked so it can be closed, but exit does not wait for it.
  Detached,
  /// A change to the outside world the user was already promised (a write, a
  /// `mkdir`, a child process the script is waiting on). Exit waits for it.
  Blocked,
}

/// The result of one operation, on its way back to the main thread.
///
/// The payload is type-erased because the channel carries every operation's
/// result; [`spawn_op`] is the only place that knows the real type, and the
/// closure it parks in the table is the only place that downcasts it.
pub struct Completion {
  pub(crate) id: OpId,
  result: Box<dyn Any + Send>,
}

impl std::fmt::Debug for Completion {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("Completion")
      .field("id", &self.id)
      .finish_non_exhaustive()
  }
}

/// Turns the `Send` result into a javascript value, on the main thread.
///
/// `aux` is a javascript value the binding wanted to keep alive across the
/// await: `fs.read` parks the caller's `ArrayBuffer` here, so the bytes are
/// copied in *after* the read, against the buffer's length at that moment.
type Reply = Box<
  dyn for<'js> FnOnce(
    &Ctx<'js>,
    Option<Value<'js>>,
    Box<dyn Any + Send>,
  ) -> JsResult<Value<'js>>,
>;

/// Lets the `for<'js>` bound drive the closure's signature inference.
fn erase<R, T>(reply: R) -> Reply
where
  T: Send + 'static,
  R: for<'js> FnOnce(&Ctx<'js>, Option<Value<'js>>, T) -> JsResult<Value<'js>>
    + 'static,
{
  Box::new(move |ctx, aux, boxed| {
    match *boxed
      .downcast::<Result<T, String>>()
      .expect("an operation completes with the type it was spawned with")
    {
      Ok(value) => reply(ctx, aux, value),
      Err(message) => Err(Exception::throw_message(ctx, &message)),
    }
  })
}

/// One operation waiting for its completion.
pub(crate) struct PendingOp {
  #[allow(dead_code)]
  pub(crate) name: &'static str,
  pub(crate) resolve: Persistent<Function<'static>>,
  pub(crate) reject: Persistent<Function<'static>>,
  pub(crate) aux: Option<Persistent<Value<'static>>>,
  pub(crate) reply: Reply,
}

/// The in-flight operations and the armed timers.
///
/// One per runtime, owned by [`crate::js::JsHost`] and reachable from a binding
/// through [`op_table`]. Everything in it is `!Send` on purpose: it is only
/// touched from the thread that owns the engine.
pub struct OpTable {
  next: Cell<OpId>,
  pending: RefCell<FoldMap<OpId, PendingOp>>,
  tx: UnboundedSender<JsMessage>,
  blocked: TaskTracker,
  detached: TaskTracker,
  pub(crate) timers: RefCell<Timers>,
}

impl OpTable {
  pub fn new(
    tx: UnboundedSender<JsMessage>,
    blocked: TaskTracker,
    detached: TaskTracker,
  ) -> Self {
    Self {
      next: Cell::new(1),
      pending: RefCell::new(FoldMap::default()),
      tx,
      blocked,
      detached,
      timers: RefCell::new(Timers::default()),
    }
  }

  /// How many operations are still waiting for their completion.
  pub fn pending_len(&self) -> usize {
    self.pending.borrow().len()
  }

  /// How many timers are armed.
  pub fn timers_len(&self) -> usize {
    self.timers.borrow().len()
  }

  pub(crate) fn take(&self, id: OpId) -> Option<PendingOp> {
    self.pending.borrow_mut().remove(&id)
  }

  /// Drops every `Persistent` the table holds.
  ///
  /// A `Persistent` that outlives its runtime makes QuickJS abort the process
  /// (`JS_FreeRuntime: Assertion 'list_empty(&rt->gc_obj_list)' failed`), so the
  /// owner calls this before the runtime goes away.
  pub fn clear(&self) {
    self.pending.borrow_mut().clear();
    self.timers.borrow_mut().clear();
  }
}

pub(crate) struct OpTableHandle(pub(crate) Rc<OpTable>);

// The table holds no `'js` types, so its lifetime is trivially unchanged.
unsafe impl<'js> JsLifetime<'js> for OpTableHandle {
  type Changed<'to> = OpTableHandle;
}

/// The operation table of the runtime `ctx` belongs to, if there is one.
///
/// Unlike [`op_table`] this raises nothing, so it is the one for code that can
/// quietly do nothing without a host (`clearTimeout` on a bare context).
pub fn try_op_table(ctx: &Ctx<'_>) -> Option<Rc<OpTable>> {
  ctx
    .userdata::<OpTableHandle>()
    .map(|handle| handle.0.clone())
}

/// The operation table of the runtime `ctx` belongs to.
pub fn op_table(ctx: &Ctx<'_>) -> JsResult<Rc<OpTable>> {
  try_op_table(ctx)
    .ok_or_else(|| Exception::throw_message(ctx, "js host is not initialised"))
}

/// Runs `fut` as a tokio task and returns the promise its result settles.
///
/// - `fut` is plain Rust: `Send + 'static`, it cannot capture a javascript
///   value. Anything the reply needs from the javascript side goes in `aux`.
/// - `reply` runs on the main thread, after the task is done, and builds the
///   value the promise resolves with. An `Err(String)` from `fut` rejects the
///   promise with an `Error` carrying that message instead.
pub fn spawn_op<'js, T, F, R>(
  ctx: &Ctx<'js>,
  name: &'static str,
  kind: OpKind,
  aux: Option<Value<'js>>,
  fut: F,
  reply: R,
) -> JsResult<Promise<'js>>
where
  T: Send + 'static,
  F: Future<Output = Result<T, String>> + Send + 'static,
  R: for<'a> FnOnce(&Ctx<'a>, Option<Value<'a>>, T) -> JsResult<Value<'a>>
    + 'static,
{
  let table = op_table(ctx)?;
  let (promise, resolve, reject) = Promise::new(ctx)?;

  let id = table.next.get();
  table.next.set(id + 1);
  trace!("|spawn_op| id:{:?}, name:{:?}, kind:{:?}", id, name, kind);

  table.pending.borrow_mut().insert(
    id,
    PendingOp {
      name,
      resolve: Persistent::save(ctx, resolve),
      reject: Persistent::save(ctx, reject),
      aux: aux.map(|value| Persistent::save(ctx, value)),
      reply: erase::<R, T>(reply),
    },
  );

  let tx = table.tx.clone();
  let task = async move {
    let result = fut.await;
    // The receiver only goes away with the loop, and then nobody is left to
    // settle the promise anyway.
    let _ = tx.send(JsMessage::OpDone(Completion {
      id,
      result: Box::new(result),
    }));
  };
  match kind {
    OpKind::Blocked => table.blocked.spawn(task),
    OpKind::Detached => table.detached.spawn(task),
  };

  Ok(promise)
}

/// [`spawn_op`] for the common case: the result converts to javascript by
/// itself and there is nothing to keep alive.
pub fn spawn_simple_op<'js, T, F>(
  ctx: &Ctx<'js>,
  name: &'static str,
  kind: OpKind,
  fut: F,
) -> JsResult<Promise<'js>>
where
  T: for<'a> rquickjs::IntoJs<'a> + Send + 'static,
  F: Future<Output = Result<T, String>> + Send + 'static,
{
  spawn_op(ctx, name, kind, None, fut, |ctx, _aux, value: T| {
    rquickjs::IntoJs::into_js(value, ctx)
  })
}

impl Completion {
  /// Hands the payload to the reply closure parked in the table.
  pub(crate) fn into_result(self) -> Box<dyn Any + Send> {
    self.result
  }
}
