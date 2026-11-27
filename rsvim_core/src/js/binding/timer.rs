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

  // `as u64` saturates, and NaN and negative delays become 0; the timer table
  // clamps whatever comes out into `[1ms, MAX_DELAY]`.
  let delay = Duration::from_millis(delay as u64);

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
