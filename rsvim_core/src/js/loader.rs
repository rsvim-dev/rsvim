//! Module resolution and loading for the QuickJS-ng backend.
//!
//! QuickJS walks the dependency graph itself: `Module::eval` calls back into the
//! loader for every `import` it meets, so the whole `fetch_module_tree` /
//! `ModuleMap` machinery the v8 backend needs has no counterpart here. All this
//! needs to do is hand back source for the modules baked into the binary.

use crate::js::CORE_MODULES;
use rquickjs::Ctx;
use rquickjs::Error;
use rquickjs::Module;
use rquickjs::loader::ImportAttributes;
use rquickjs::loader::Loader;
use rquickjs::loader::Resolver;
use rquickjs::module::Declared;

/// Resolves the built-in `rsvim:*` modules.
///
/// Everything else is passed through untouched so a later resolver in the tuple
/// (a filesystem resolver, the import map) can claim it. Returning an error here
/// would abort the chain instead of letting the next resolver try.
pub struct CoreResolver;

impl Resolver for CoreResolver {
  fn resolve<'js>(
    &mut self,
    _ctx: &Ctx<'js>,
    base: &str,
    name: &str,
    _attributes: Option<ImportAttributes<'js>>,
  ) -> rquickjs::Result<String> {
    if CORE_MODULES.contains_key(name) {
      Ok(name.to_string())
    } else {
      Err(Error::new_resolving(base, name))
    }
  }
}

/// Loads the built-in `rsvim:*` modules from the copy compiled into the binary.
pub struct CoreLoader;

impl Loader for CoreLoader {
  fn load<'js>(
    &mut self,
    ctx: &Ctx<'js>,
    name: &str,
    _attributes: Option<ImportAttributes<'js>>,
  ) -> rquickjs::Result<Module<'js, Declared>> {
    let source = CORE_MODULES
      .get(name)
      .ok_or_else(|| Error::new_loading(name))?;
    Module::declare(ctx.clone(), name, source.as_bytes())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::tests::log::init as test_log_init;
  use rquickjs::Context;
  use rquickjs::Runtime;

  #[test]
  #[cfg_attr(miri, ignore)]
  fn core_modules_resolve_and_load() {
    test_log_init();
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();
    rt.set_loader(CoreResolver, CoreLoader);
    ctx.with(|ctx| {
      let declared = CoreLoader.load(&ctx, "rsvim:cmd", None).unwrap();
      let (evaluated, promise) = declared.eval().unwrap();
      drop(evaluated);
      promise.finish::<()>().unwrap();
      // A module that is not in `CORE_MODULES` must be rejected, not silently
      // turned into an empty module.
      assert!(CoreLoader.load(&ctx, "rsvim:nope", None).is_err());
    });
  }

  #[test]
  #[cfg_attr(miri, ignore)]
  fn every_core_module_parses() {
    test_log_init();
    let rt = Runtime::new().unwrap();
    let ctx = Context::full(&rt).unwrap();
    rt.set_loader(CoreResolver, CoreLoader);
    ctx.with(|ctx| {
      for name in CORE_MODULES.keys() {
        let declared = CoreLoader
          .load(&ctx, name, None)
          .unwrap_or_else(|e| panic!("{name} failed to load: {e:?}"));
        // `eval` returns before the module body runs, so a syntax error would
        // have surfaced in `declare` already; driving the promise is what runs
        // the top-level statements.
        let (_, promise) = declared.eval().unwrap();
        promise
          .finish::<()>()
          .unwrap_or_else(|e| panic!("{name} failed to evaluate: {e:?}"));
      }
    });
  }
}
