//! `queueMicrotask` and `reportError` — the remaining web globals that do not
//! need the event loop.
//!
//! `TextEncoder`/`TextDecoder` live in the [`text_encoder`] submodule. The
//! timer half of the web globals (`setTimeout`/`setInterval`) belongs to the
//! async step: a timer has to park a callback and come back to it later, which
//! needs the completion channel that step introduces.

use crate::js::binding::set_function_to;
use crate::js::err::JsError;
use crate::js::runtime::kernel;
use rquickjs::Ctx;
use rquickjs::Function;
use rquickjs::Object;
use rquickjs::Result as JsResult;
use rquickjs::Value;

mod text_encoder;

#[cfg(test)]
mod microtask_tests;

/// `queueMicrotask(callback)`.
///
/// `Function::defer` is exactly this: it pushes the call onto QuickJS's job
/// queue, which is the microtask queue. v8 needed the context's
/// `MicrotaskQueue` handle instead.
pub fn queue_microtask<'js>(
  _ctx: Ctx<'js>,
  callback: Function<'js>,
) -> JsResult<()> {
  callback.defer(())
}

/// `reportError(exception)`.
///
/// There is no rejection tracker in play here — this is the explicit path
/// `00__web.min.js` uses when a callback throws. The error is shown the way
/// every other uncaught error is, see [`JsError::from_exception`].
pub fn report_error<'js>(ctx: Ctx<'js>, error: Value<'js>) -> JsResult<()> {
  let kernel = kernel(&ctx)?;
  let error = JsError::from_js_value(&ctx, error);
  crate::js::err::report_js_error(&kernel.borrow().editor, error);
  Ok(())
}

/// Installs the web globals that do not need the event loop.
pub fn install<'js>(ctx: &Ctx<'js>, internal: &Object<'js>) -> JsResult<()> {
  text_encoder::install(ctx, internal)?;
  set_function_to(ctx, internal, "global_queue_microtask", queue_microtask)?;
  set_function_to(ctx, internal, "global_report_error", report_error)?;
  Ok(())
}
