//! `setTimeout` / `setInterval` backends.
//!
//! The v8 backend parks a callback in `pending_timers` and asks the event loop
//! to wake it through `MasterMessage::TimeoutReq`; the tick then pops the
//! callback out of `pending_futures`. QuickJS needs neither hop: a timer is a
//! future spawned into the engine's own scheduler (`Ctx::spawn`), and the
//! scheduler is driven by `AsyncRuntime::drive`, which the event loop polls in
//! its waiting phase.
//!
//! That means the registry here only has to answer one question: is this timer
//! still armed? The callback itself lives in the spawned future, so cancelling
//! is a flag flip and the future exits on its next wake-up.

use crate::js::TimerId;
use crate::js::rqjs::binding::set_function_to;
use crate::js::rqjs::get_state;
use crate::prelude::*;
use rquickjs::Ctx;
use rquickjs::Exception;
use rquickjs::Function;
use rquickjs::JsLifetime;
use rquickjs::Persistent;
use rquickjs::Result as JsResult;
use rquickjs::Value;
use std::cell::Cell;
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Duration;

/// The timers that are still armed, keyed by the id javascript holds.
///
/// Stored as context userdata rather than as a field of [`crate::js::rqjs::JsKernel`]
/// so that adding timers does not force every kernel construction site to grow
/// an argument for state only this module reads.
pub struct TimerRegistry {
  timers: RefCell<FoldMap<TimerId, Rc<Cell<bool>>>>,
}

// The registry holds no `'js` types.
unsafe impl<'js> JsLifetime<'js> for TimerRegistry {
  type Changed<'to> = TimerRegistry;
}

impl TimerRegistry {
  fn new() -> Self {
    Self {
      timers: RefCell::new(FoldMap::default()),
    }
  }
}

/// Installs an empty registry if the context does not have one yet.
pub fn init_registry<'js>(ctx: &Ctx<'js>) -> JsResult<()> {
  if ctx.userdata::<TimerRegistry>().is_some() {
    return Ok(());
  }
  ctx
    .store_userdata(TimerRegistry::new())
    .map(|_| ())
    .map_err(|_| Exception::throw_message(ctx, "timer registry is taken"))
}

/// Reports a callback failure the same way `reportError` does.
fn report_failure<'js>(ctx: &Ctx<'js>, error: Value<'js>) {
  let message = super::global::describe_exception(ctx, error);
  let Some(kernel) = get_state(ctx) else {
    return;
  };
  let editor = kernel.borrow().editor.clone();
  crate::js::cmdline::send_cmdline_message(&editor, message);
}

/// `setTimeout(callback, delayMs, repeated)` -> timer id.
///
/// The third argument is `true` for `setInterval`; `00__web.min.js` normalises
/// the delay into `[1, 2^31-1]` before calling, so anything out of range here
/// is a caller that skipped the wrapper and is clamped rather than rejected.
pub fn create_timer<'js>(
  ctx: Ctx<'js>,
  callback: Function<'js>,
  delay: f64,
  repeated: bool,
) -> JsResult<i32> {
  init_registry(&ctx)?;

  let timer_id = TimerId::next();
  let cancelled = Rc::new(Cell::new(false));

  {
    let registry = ctx.userdata::<TimerRegistry>().ok_or_else(|| {
      Exception::throw_message(&ctx, "timer registry is not initialised")
    })?;
    registry
      .timers
      .borrow_mut()
      .insert(timer_id, cancelled.clone());
  }

  let millis = if delay.is_finite() && delay >= 1.0 {
    delay as u64
  } else {
    1
  };
  trace!(
    "|create_timer| timer_id:{:?}, delay:{:?}, repeated:{:?}",
    timer_id, millis, repeated
  );

  let callback = Persistent::save(&ctx, callback);
  let task_ctx = ctx.clone();
  ctx.spawn(async move {
    loop {
      tokio::time::sleep(Duration::from_millis(millis)).await;
      if cancelled.get() {
        break;
      }

      // The `Persistent` outlives no runtime: this future is owned by the
      // engine's own scheduler, which is dropped with the runtime.
      let Ok(func) = callback.clone().restore(&task_ctx) else {
        break;
      };
      if func.call::<_, ()>(()).is_err() {
        let error = task_ctx.catch();
        report_failure(&task_ctx, error);
      }

      if !repeated {
        break;
      }
    }

    if let Some(registry) = task_ctx.userdata::<TimerRegistry>() {
      registry.timers.borrow_mut().remove(&timer_id);
    }
  });

  Ok(timer_id.into())
}

/// `clearTimeout(timerId)` / `clearInterval(timerId)`.
pub fn clear_timer<'js>(ctx: Ctx<'js>, timer_id: i32) -> JsResult<()> {
  let timer_id = TimerId::from(timer_id);
  trace!("|clear_timer| timer_id:{:?}", timer_id);
  let Some(registry) = ctx.userdata::<TimerRegistry>() else {
    return Ok(());
  };
  if let Some(cancelled) = registry.timers.borrow_mut().remove(&timer_id) {
    cancelled.set(true);
  }
  Ok(())
}

/// Installs `global_create_timer` / `global_clear_timer` on `internal`.
pub fn install<'js>(
  ctx: &Ctx<'js>,
  internal: &rquickjs::Object<'js>,
) -> JsResult<()> {
  init_registry(ctx)?;
  set_function_to(ctx, internal, "global_create_timer", create_timer)?;
  set_function_to(ctx, internal, "global_clear_timer", clear_timer)?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use crate::js::rqjs::binding::install_bindings;
  use crate::js::rqjs::runtime::JsKernel;
  use crate::js::rqjs::runtime::JsRuntimeOptions;
  use crate::js::rqjs::runtime::set_state;
  use crate::tests::js::make_editor;
  use crate::tests::log::init as test_log_init;
  use rquickjs::AsyncContext;
  use rquickjs::AsyncRuntime;
  use std::task::Poll;
  use std::time::Duration;

  /// Builds a context with the bindings installed.
  ///
  /// The driver is deliberately *not* polled here: `AsyncRuntime::drive` takes
  /// the same inner lock that `AsyncContext::with` holds, so driving from
  /// inside a `with` block would deadlock. Callers poll it between `with`
  /// blocks, which is exactly how the event loop uses it.
  async fn make_context() -> (AsyncContext, AsyncRuntime) {
    let editor = make_editor();
    let (master_tx, _master_rx) = tokio::sync::mpsc::unbounded_channel();
    let kernel = JsKernel::to_rc(JsKernel {
      options: JsRuntimeOptions::default(),
      master_tx,
      editor,
      blocked_tracker: tokio_util::task::TaskTracker::new(),
    });
    let rt = AsyncRuntime::new().unwrap();
    let ctx = AsyncContext::full(&rt).await.unwrap();
    ctx
      .with(move |ctx| {
        set_state(&ctx, kernel).unwrap();
        install_bindings(&ctx).unwrap();
      })
      .await;
    (ctx, rt)
  }

  /// Polls the driver once and yields to tokio so timers can make progress.
  async fn pump(
    drive: &mut std::pin::Pin<&mut rquickjs::runtime::DriveFuture>,
  ) {
    futures::future::poll_fn(|cx| {
      let _ = drive.as_mut().poll(cx);
      Poll::Ready(())
    })
    .await;
    tokio::time::sleep(Duration::from_millis(2)).await;
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn timer_fires_once_after_the_delay() {
    test_log_init();
    let (ctx, rt) = make_context().await;

    ctx
      .with(|ctx| {
        ctx
          .eval::<(), _>(
            r#"
            globalThis.fired = 0;
            __InternalRsvimGlobalObject.global_create_timer(() => {
              globalThis.fired += 1;
            }, 1, false);
            "#,
          )
          .unwrap();
      })
      .await;

    let drive = rt.drive();
    futures::pin_mut!(drive);

    let mut fired = 0;
    for _ in 0..100 {
      pump(&mut drive).await;
      fired = ctx.with(|ctx| ctx.eval("globalThis.fired").unwrap()).await;
      if fired == 1 {
        break;
      }
    }
    assert_eq!(fired, 1, "the timer must fire exactly once");

    // A one-shot timer must not come back.
    for _ in 0..10 {
      pump(&mut drive).await;
    }
    let fired: i32 =
      ctx.with(|ctx| ctx.eval("globalThis.fired").unwrap()).await;
    assert_eq!(fired, 1, "a one-shot timer must not fire twice");
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn cleared_timer_never_fires() {
    test_log_init();
    let (ctx, rt) = make_context().await;

    ctx
      .with(|ctx| {
        ctx
          .eval::<(), _>(
            r#"
            globalThis.fired = 0;
            globalThis.id = __InternalRsvimGlobalObject.global_create_timer(() => {
              globalThis.fired += 1;
            }, 1, false);
            "#,
          )
          .unwrap();
        ctx
          .eval::<(), _>("__InternalRsvimGlobalObject.global_clear_timer(globalThis.id)")
          .unwrap();
      })
      .await;

    let drive = rt.drive();
    futures::pin_mut!(drive);
    for _ in 0..20 {
      pump(&mut drive).await;
    }

    let fired: i32 =
      ctx.with(|ctx| ctx.eval("globalThis.fired").unwrap()).await;
    assert_eq!(fired, 0, "a cleared timer must not run its callback");
  }
}
