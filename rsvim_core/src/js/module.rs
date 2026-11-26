//! Module path and source types shared by the loader and the error type.
//!
//! QuickJS does the graph walk itself, so these are the only two things the
//! rest of the crate still has to name: a resolved module identity and the
//! source text handed back to the engine.

/// The identity of a loaded module.
pub type ModulePath = String;

/// The source text of a loaded module.
pub type ModuleSource = String;

/// Loads module sources by specifier.
///
/// QuickJS calls back into this through [`rquickjs::loader::Loader`]; the trait
/// stays engine-shaped (plain `TheResult`) so [`crate::js::fs_loader`] and its
/// tests do not have to know about `rquickjs` error types.
pub trait ModuleLoader {
  /// Resolves `specifier` relative to `base` into a module path.
  fn resolve(
    &self,
    base: &str,
    specifier: &str,
  ) -> crate::err::TheResult<ModulePath>;

  /// Loads the source text of `module_path`.
  fn load(&self, module_path: &str) -> crate::err::TheResult<ModuleSource>;
}
