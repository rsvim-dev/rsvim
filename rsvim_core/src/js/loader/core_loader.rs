//! Core module loader.

use crate::js::loader::ModuleLoader;
use crate::js::module::CORE_MODULES;
use crate::js::module::ModulePath;
use crate::js::module::ModuleSource;
use crate::prelude::*;

#[derive(Default)]
/// For core module loader, its module path is the same with the specifier.
///
/// To indicate core modules, they use a special pattern 'rsvim:ext' as its prefix. For example:
///
/// ```javascript
/// import { cmd } from "rsvim:ext/cmd";
/// import { fs } from "rsvim:ext/fs";
/// import { opt } from "rsvim:ext/opt";
/// ```
///
/// See [`CORE_MODULES`] for the builtin modules actually provided.
pub struct CoreModuleLoader;

impl ModuleLoader for CoreModuleLoader {
  /// Resolve module path by its specifier.
  fn resolve(&self, _base: &str, specifier: &str) -> TheResult<ModulePath> {
    assert!(CORE_MODULES.contains_key(specifier));
    Ok(specifier.to_string())
  }

  /// Load module source by its module path.
  fn load(&self, module_path: &str) -> TheResult<ModuleSource> {
    assert!(CORE_MODULES.contains_key(module_path));
    Ok(CORE_MODULES.get(module_path).unwrap().to_string())
  }
}
