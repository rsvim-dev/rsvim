//! Js runtime bindings for the QuickJS-ng backend.
//!
//! The shape mirrors `crate::js::v8::binding`: a context is created, one
//! internal object (`__InternalRsvimGlobalObject`) is hung off the global, and
//! every built-in function is a plain function on that object. The javascript
//! modules in `rsvim_core/js/*.min.js` never touch anything else, which is why
//! the same sources work for both engines.

pub mod buf;
pub mod cmd;
pub mod opt;
pub mod process;
pub mod syn;

use rquickjs::Ctx;
use rquickjs::Function;
use rquickjs::Module;
use rquickjs::Object;
use rquickjs::Result as JsResult;
use rquickjs::function::IntoJsFunc;

/// The name of the object every js-callable built-in hangs off.
///
/// Kept identical to the v8 backend on purpose: `rsvim_core/js/*.min.js` call
/// `__InternalRsvimGlobalObject.cmd_echo(...)` and friends, and those sources
/// are shared.
pub const INTERNAL_OBJECT_NAME: &str = "__InternalRsvimGlobalObject";

/// Registers `f` as a function named `name` on `target`.
///
/// `P` is spelled out rather than defaulted because it is the parameter list
/// `F` implements [`IntoJsFunc`] for; leaving it to inference would make the
/// call site unnameable whenever `F` is a closure over borrowed state.
pub fn set_function_to<'js, F, P>(
  ctx: &Ctx<'js>,
  target: &Object<'js>,
  name: &str,
  f: F,
) -> JsResult<()>
where
  F: IntoJsFunc<'js, P> + 'js,
  P: 'js,
{
  let func = Function::new(ctx.clone(), f)?;
  target.set(name, func)?;
  Ok(())
}

/// Creates the object the built-in functions live on and returns it.
pub fn create_internal_object<'js>(ctx: &Ctx<'js>) -> JsResult<Object<'js>> {
  let global = ctx.globals();
  let object = Object::new(ctx.clone())?;
  global.set(INTERNAL_OBJECT_NAME, object.clone())?;
  Ok(object)
}

/// Installs every js-callable built-in on a fresh context.
///
/// This is the rqjs counterpart of `crate::js::v8::binding::create_new_context`:
/// it does not create the context (the caller already has one), it only fills it
/// in. Callers are expected to run it exactly once per context.
pub fn install_bindings<'js>(ctx: &Ctx<'js>) -> JsResult<()> {
  let internal = create_internal_object(ctx)?;
  buf::install(ctx, &internal)?;
  cmd::install(ctx, &internal)?;
  opt::install(ctx, &internal)?;
  process::install(ctx, &internal)?;
  syn::install(ctx, &internal)?;
  Ok(())
}

/// Loads the bootstrap module that installs `setTimeout`, `TextEncoder` and the
/// rest of the web-shaped globals.
///
/// The v8 backend feeds the same file through `fetch_module_tree` + `evaluate`;
/// QuickJS walks the dependency graph through the loader instead, so this only
/// has to evaluate the entry module and drive the promise it returns.
pub fn install_web_globals<'js>(ctx: &Ctx<'js>) -> JsResult<()> {
  let promise = Module::evaluate(
    ctx.clone(),
    "00__web.js",
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/js/00__web.min.js")),
  )?;
  promise.finish::<()>()?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::js::rqjs::loader::CoreLoader;
  use crate::js::rqjs::loader::CoreResolver;
  use crate::tests::log::init as test_log_init;
  use rquickjs::AsyncContext;
  use rquickjs::AsyncRuntime;

  #[test]
  #[cfg_attr(miri, ignore)]
  fn web_globals_install() {
    test_log_init();
    let rt = AsyncRuntime::new().unwrap();
    let ctx = futures::executor::block_on(AsyncContext::full(&rt)).unwrap();
    futures::executor::block_on(rt.set_loader(CoreResolver, CoreLoader));
    futures::executor::block_on(ctx.with(|ctx| {
      install_web_globals(&ctx).unwrap();
      // The bootstrap is what makes these globals exist at all; without it the
      // `rsvim:*` modules would call into thin air.
      let kind: String = ctx.eval("typeof globalThis.setTimeout").unwrap();
      assert_eq!(kind, "function");
      let kind: String = ctx.eval("typeof globalThis.TextEncoder").unwrap();
      assert_eq!(kind, "function");
    }));
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn internal_object_is_reachable_from_javascript() {
    test_log_init();
    let rt = AsyncRuntime::new().unwrap();
    let ctx = futures::executor::block_on(AsyncContext::full(&rt)).unwrap();
    futures::executor::block_on(ctx.with(|ctx| {
      let internal = create_internal_object(&ctx).unwrap();
      set_function_to(&ctx, &internal, "answer", || 42i32).unwrap();
      let answer: i32 =
        ctx.eval("__InternalRsvimGlobalObject.answer()").unwrap();
      assert_eq!(answer, 42);
    }));
  }
}
