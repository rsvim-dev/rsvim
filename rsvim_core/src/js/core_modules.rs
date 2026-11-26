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
      // ("rsvim:infra", include_str!("./runtime/00__infra.js")),
      // ("console", include_str!("./js/console.js")),
      // ("events", include_str!("./js/events.js")),
      // ("process", include_str!("./js/process.js")),
      // ("timers", include_str!("./js/timers.js")),
      // ("assert", include_str!("./js/assert.js")),
      // ("util", include_str!("./js/util.js")),
      // ("fs", include_str!("./module/fs.js")),
      // ("perf_hooks", include_str!("./js/perf-hooks.js")),
      // ("colors", include_str!("./js/colors.js")),
      // ("dns", include_str!("./js/dns.js")),
      // ("net", include_str!("./js/net.js")),
      // ("test", include_str!("./js/test.js")),
      // ("stream", include_str!("./js/stream.js")),
      // ("http", include_str!("./js/http.js")),
      // ("@web/abort", include_str!("./js/abort-controller.js")),
      // ("@web/text_encoding", include_str!("./js/text-encoding.js")),
      // ("@web/clone", include_str!("./js/structured-clone.js")),
      // ("@web/fetch", include_str!("./js/fetch.js")),
    ];
    FoldMap::from_iter(modules)
  });
