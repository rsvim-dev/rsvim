//! Engine-neutral module sources.
//!
//! These are the javascript sources embedded into the binary, loaded through
//! [`crate::js::loader::CoreLoader`]. The sources themselves never had anything
//! to do with an engine, so the table lives outside any backend.

use crate::prelude::*;
use std::sync::LazyLock;

/// Builtin modules, maps from the module specifier to its source code.
///
/// These modules are compiled by `tsc` and bundled by `rollup`, see
/// `rollup.config.js`, then minified by `terser` into `./js/*.min.js`, see the
/// `bundle` recipe in `justfile`. They are embedded into the binary at compile
/// time, thus no file is needed at runtime.
///
/// The `rsvim:` prefix indicates a core module, see [`CoreModuleLoader`].
///
/// [`CoreModuleLoader`]: crate::js::loader::core_loader::CoreModuleLoader
pub static CORE_MODULES: LazyLock<FoldMap<&'static str, &'static str>> =
  LazyLock::new(|| {
    let modules = vec![
      (
        "rsvim:buf",
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/js/buf.min.js")),
      ),
      (
        "rsvim:cmd",
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/js/cmd.min.js")),
      ),
      (
        "rsvim:fs",
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/js/fs.min.js")),
      ),
      (
        "rsvim:opt",
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/js/opt.min.js")),
      ),
      (
        "rsvim:process",
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/js/process.min.js")),
      ),
      (
        "rsvim:syn",
        include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/js/syn.min.js")),
      ),
    ];
    FoldMap::from_iter(modules)
  });
