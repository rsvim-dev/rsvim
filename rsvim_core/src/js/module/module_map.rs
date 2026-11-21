//! Module map.
//!
//! # Terms
//!
//! - Module map: it maintains all the dependencies loaded into the js runtime.
//!
//! # Module
//!
//! In node.js, a module is loaded by keyword `require` (Common JS) or `import`
//! (ECMAScript Module). Rsvim only supports the static import, for example:
//!
//! ```javascript
//! // ES Module
//! import _ from "lodash";
//!
//! // Common JS
//! const _ = require("lodash");
//! ```
//!
//! Static import runs synchronously.

use crate::js::module::ModulePath;
use crate::prelude::*;
use std::collections::hash_map::Entry;
use std::fmt::Debug;

/// Module map.
/// It maintains all the modules inside js runtime.
pub struct ModuleMap {
  // Entry point of runtime execution, this is the `rsvim.js`
  // configuration entry point for Rsvim.
  main: Option<ModulePath>,

  // Maps from "Module Path" to "v8 Module".
  by_path: FoldMap<ModulePath, v8::Global<v8::Module>>,

  // Maps from "Module ID" to "v8 Module" and its path.
  by_id: FoldMap<i32, Vec<(ModulePath, v8::Global<v8::Module>)>>,
}

impl Debug for ModuleMap {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("ModuleMap")
      .field("main", &self.main)
      .field(
        "by_path",
        &self
          .by_path
          .keys()
          .map(|k| (k.clone(), "v8::Module".to_string()))
          .collect::<FoldMap<String, String>>(),
      )
      .field(
        "by_id",
        &self
          .by_id
          .iter()
          .map(|(k, v)| (*k, v.iter().map(|e| e.0.clone()).collect()))
          .collect::<FoldMap<i32, Vec<String>>>(),
      )
      .finish()
  }
}

impl ModuleMap {
  /// Creates a global module map.
  pub fn new() -> ModuleMap {
    Self {
      main: None,
      by_path: FoldMap::new(),
      by_id: FoldMap::new(),
    }
  }

  pub fn main(&self) -> &Option<ModulePath> {
    &self.main
  }

  /// Add a compiled v8 module to the cache.
  pub fn insert(
    &mut self,
    path: &str,
    module_id: i32,
    module: v8::Global<v8::Module>,
  ) {
    // No main module has been set, so let's update the value.
    if self.main.is_none() && std::fs::metadata(path).is_ok() {
      self.main = Some(path.into());
    }
    self.by_path.insert(path.into(), module.clone());
    let entry = (path.into(), module.clone());
    match self.by_id.entry(module_id) {
      Entry::Vacant(value) => {
        value.insert(vec![entry]);
      }
      Entry::Occupied(mut value) => {
        value.get_mut().push(entry);
      }
    }
  }

  /// Get v8 module by its path.
  pub fn get(&self, path: &str) -> Option<v8::Global<v8::Module>> {
    self.by_path.get(path).cloned()
  }

  /// Whether a v8 module already resolved.
  pub fn contains(&self, path: &str) -> bool {
    self.by_path.contains_key(path)
  }

  #[cfg(test)]
  /// Whether a v8 module already resolved, compare by key suffix.
  pub fn get_by_suffix(
    &self,
    path: &str,
  ) -> Option<(&ModulePath, &v8::Global<v8::Module>)> {
    self.by_path.iter().find(|(k, _v)| {
      let kp = Path::new(k).canonicalize().unwrap();
      let pp = Path::new(path).canonicalize().unwrap();
      kp.ends_with(&pp) || pp.ends_with(kp)
    })
  }

  /// Get module path by its ID.
  ///
  /// NOTE: v8 Module does not guarantee its ID is unique, thus here we use
  /// `Vec<v8::Module>` to store all the modules with the same ID, to solve the
  /// conflicted hash IDs.
  pub fn get_path(
    &self,
    module_id: i32,
    module: v8::Global<v8::Module>,
  ) -> Option<ModulePath> {
    match self.by_id.get(&module_id) {
      None => None,
      Some(entries) => {
        if entries.len() <= 1 {
          debug_assert!(!entries.is_empty());
          Some(entries[0].0.clone())
        } else {
          entries
            .iter()
            .find(|(_, m)| *m == module)
            .map(|(p, _)| p.clone())
        }
      }
    }
  }
}
