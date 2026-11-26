//! `setTimeout` / `setInterval` backends.
//!
//! A timer is an entry in the host's timer table ([`crate::js::timers::Timers`]),
//! not a future: the event loop waits on the table and runs the callback through
//! [`crate::js::JsHost::run_timer`]. `clearTimeout` therefore just removes the
//! entry, which releases the callback at once.

use crate::js::TimerId;
use crate::js::binding::set_function_to;
use crate::js::ops::op_table;
use crate::js::ops::try_op_table;
use crate::prelude::*;
use rquickjs::Ctx;
use rquickjs::Function;
use rquickjs::Persistent;
use rquickjs::Result as JsResult;
use std::time::Duration;

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
  let table = op_table(&ctx)?;

  let delay = if delay.is_finite() && delay >= 1.0 {
    Duration::from_millis(delay as u64)
  } else {
    Duration::from_millis(1)
  };

  let timer_id = table.timers.borrow_mut().insert(
    Persistent::save(&ctx, callback),
    delay,
    repeated,
  );
  trace!(
    "|create_timer| timer_id:{:?}, delay:{:?}, repeated:{:?}",
    timer_id, delay, repeated
  );
  Ok(timer_id.into())
}

/// `clearTimeout(timerId)` / `clearInterval(timerId)`.
pub fn clear_timer<'js>(ctx: Ctx<'js>, timer_id: i32) -> JsResult<()> {
  let timer_id = TimerId::from(timer_id);
  trace!("|clear_timer| timer_id:{:?}", timer_id);
  if let Some(table) = try_op_table(&ctx) {
    table.timers.borrow_mut().remove(timer_id);
  }
  Ok(())
}

/// Installs `global_create_timer` / `global_clear_timer` on `internal`.
pub fn install<'js>(
  ctx: &Ctx<'js>,
  internal: &rquickjs::Object<'js>,
) -> JsResult<()> {
  set_function_to(ctx, internal, "global_create_timer", create_timer)?;
  set_function_to(ctx, internal, "global_clear_timer", clear_timer)?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use crate::tests::js::make_js_host;
  use crate::tests::log::init as test_log_init;
  use std::time::Duration;

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn timer_fires_once_after_the_delay() {
    test_log_init();
    let mut js = make_js_host();

    js.enter(|ctx| {
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
    });

    let done = js
      .run_until(Duration::from_secs(5), |js| {
        js.enter(|ctx| ctx.eval::<i32, _>("globalThis.fired").unwrap()) == 1
      })
      .await;
    assert!(done, "the timer must fire");

    // A one-shot timer must not come back.
    js.run_for(Duration::from_millis(30)).await;
    let fired: i32 = js.enter(|ctx| ctx.eval("globalThis.fired").unwrap());
    assert_eq!(fired, 1, "a one-shot timer must not fire twice");
    assert_eq!(
      js.host.ops().timers_len(),
      0,
      "a fired timer leaves the table"
    );
  }

  #[tokio::test]
  #[cfg_attr(miri, ignore)]
  async fn cleared_timer_never_fires() {
    test_log_init();
    let mut js = make_js_host();

    js.enter(|ctx| {
      ctx
        .eval::<(), _>(
          r#"
          globalThis.fired = 0;
          globalThis.id = __InternalRsvimGlobalObject.global_create_timer(() => {
            globalThis.fired += 1;
          }, 1, false);
          __InternalRsvimGlobalObject.global_clear_timer(globalThis.id);
          "#,
        )
        .unwrap();
    });
    assert_eq!(js.host.ops().timers_len(), 0, "clearing removes the entry");

    js.run_for(Duration::from_millis(30)).await;
    let fired: i32 = js.enter(|ctx| ctx.eval("globalThis.fired").unwrap());
    assert_eq!(fired, 0, "a cleared timer must not run its callback");
  }
}
