//! Js module.
//!
//! There are 3 terms in this section:
//!
//! - Module specifier.
//! - Module path.
//! - Module source.
//!
//! # Module Specifier
//!
//! A module specifier is the module name used in `import`/`require` keywords. For example:
//!
//! ```javascript
//! const _ = import "lodash";
//! ```
//!
//! The `"lodash"` is the module specifier.
//!
//! # Module Path
//!
//! A module path is the local file path where the module stores. For example
//! `/home/users/project/node_modules/lodash/index.js`.
//!
//! # Module Source
//!
//! A module source is the source code of the module, such as javascript source code, and the
//! source can be evaluated by js engine. But in ECMAScript standards, there are also many other
//! kinds of sources: `json`/`json5`, `wasm`, etc.

pub mod import_map;
pub mod module_map;

#[cfg(test)]
mod module_map_tests;

use crate::js::JsRuntime;
use crate::js::binding;
use crate::js::loader::CoreModuleLoader;
use crate::js::loader::FsModuleLoader;
use crate::js::loader::ModuleLoader;
use crate::prelude::*;
use crate::util::paths;
use std::sync::LazyLock;
// use url::Url;
pub use import_map::*;
pub use module_map::*;

/// Module path on local file system.
pub type ModulePath = String;

/// Module source code.
pub type ModuleSource = String;

pub static CORE_MODULES: LazyLock<FoldMap<&'static str, &'static str>> =
  LazyLock::new(|| {
    let modules = vec![
      // ("rsvim:ext/infra", include_str!("./runtime/00__infra.js")),
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

/// Creates v8 script origins, see:
/// - Node V8 API: <https://v8docs.nodesource.com/node-24.1/db/d84/classv8_1_1_script_origin.html>
/// - Rusty V8 API: <https://docs.rs/v8/latest/v8/struct.ScriptOrigin.html>.
/// - MDN script: <https://developer.mozilla.org/en-US/docs/Web/HTML/Reference/Elements/script>.
/// - HTML5 origin: <https://www.w3.org/TR/2011/WD-html5-20110525/origin-0.html>.
pub fn create_origin<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  name: &str,
  is_module: bool,
) -> v8::ScriptOrigin<'s> {
  let name = v8::String::new(scope, name).unwrap();
  let source_map = v8::undefined(scope);

  v8::ScriptOrigin::new(
    scope,
    name.into(),
    0,
    0,
    false,
    0,
    Some(source_map.into()),
    false,
    false,
    is_module,
    None,
  )
}

fn _choose_module_loader(specifier: &str) -> &dyn ModuleLoader {
  static CORE_MODULE_LOADER: CoreModuleLoader = CoreModuleLoader {};
  static FS_MODULE_LOADER: LazyLock<FsModuleLoader> =
    LazyLock::new(FsModuleLoader::new);

  let is_core_module_import = CORE_MODULES.contains_key(specifier);
  if is_core_module_import {
    &CORE_MODULE_LOADER
  } else {
    &*FS_MODULE_LOADER
  }
}

/// Resolves module path by its specifier.
///
/// The `base` parameter is current module's local filesystem path, all its dependent modules'
/// filesystem path should be relatively based on the same directory that contains the root module,
/// i.e. current module.
///
/// The `import_map` is an optional user provided map that overwrite default module loader, see
/// [`ImportMap`].
///
/// # Returns
///
/// It returns full path on local filesystem.
pub fn resolve_import(
  base: &str,
  specifier: &str,
  import_map: Option<ImportMap>,
) -> TheResult<ModulePath> {
  // Use import-maps if available.
  let specifier = match import_map {
    Some(map) => map.lookup(specifier).unwrap_or_else(|| specifier.into()),
    None => specifier.into(),
  };

  // Look the params and choose a loader, then resolve module.
  let resolver: &dyn ModuleLoader = _choose_module_loader(specifier.as_str());

  resolver.resolve(base, &specifier)
}

/// Loads module source by its module path.
pub fn load_import(
  specifier: &str,
  _skip_cache: bool,
) -> TheResult<ModuleSource> {
  // // Look the params and choose a loader.
  // let loader: Box<dyn ModuleLoader> = match (
  //   CORE_MODULES.contains_key(specifier),
  //   WINDOWS_DRIVE_REGEX.is_match(specifier),
  //   Url::parse(specifier).is_ok(),
  // ) {
  //   (true, _, _) => Box::new(CoreModuleLoader),
  //   (_, true, _) => Box::new(FsModuleLoader),
  //   (_, _, true) => Box::new(UrlModuleLoader { skip_cache }),
  //   _ => Box::new(FsModuleLoader),
  // };
  //
  // // Load module.
  // loader.load(specifier)

  // We don't actually have core modules
  let loader: &dyn ModuleLoader = _choose_module_loader(specifier);

  loader.load(specifier)
}

/// Resolves module imports without dependency.
///
/// TODO: Support dependencies resolving for custom snapshot.
pub fn fetch_module<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  filename: &str,
  source: Option<&str>,
) -> Option<v8::Local<'s, v8::Module>> {
  // Create a script origin.
  let origin = create_origin(scope, filename, true);

  // Find appropriate loader if source is empty.
  let source = match source {
    Some(source) => source.into(),
    None => load_import(filename, true).unwrap(),
  };

  let source = v8::String::new(scope, &source).unwrap();
  let mut source = v8::script_compiler::Source::new(source, Some(&origin));

  v8::script_compiler::compile_module(scope, &mut source)
}

/// Resolves module imports synchronously.
/// See: <https://source.chromium.org/chromium/v8/v8.git/+/51e736ca62bd5c7bfd82488a5587fed31dbf45d5:src/d8.cc;l=741>.
pub fn fetch_module_tree<'s>(
  scope: &mut v8::PinScope<'s, '_>,
  filename: &str,
  source: Option<&str>,
) -> Option<v8::Local<'s, v8::Module>> {
  let module = match fetch_module(scope, filename, source) {
    Some(module) => module,
    None => {
      // Early returns `None`
      return None;
    }
  };

  let state_rc = JsRuntime::state(scope);

  // Subscribe module to the module-map.
  let module_id = module.get_identity_hash().get();
  let module_ref = v8::Global::new(scope, module);
  {
    let mut state = state_rc.borrow_mut();
    state.module_map.insert(filename, module_id, module_ref);
  }

  let requests = module.get_module_requests();

  for i in 0..requests.length() {
    // Get import request from the `module_requests` array.
    let request = requests.get(scope, i).unwrap();
    let request = v8::Local::<v8::ModuleRequest>::try_from(request).unwrap();

    // Transform v8's ModuleRequest into Rust string.
    let base = paths::maybe_parent(&filename).to_string_lossy();
    let specifier = request.get_specifier().to_rust_string_lossy(scope);

    let specifier = match resolve_import(&base, &specifier, None) {
      Ok(specifier) => specifier,
      Err(e) => {
        binding::throw_exception(scope, &e);
        return None;
      }
    };

    // Resolve subtree of modules
    // If any dependency failed fetching, early returns `None`.
    if !state_rc.borrow().module_map.contains(&specifier) {
      fetch_module_tree(scope, &specifier, None)?;
    }
  }

  Some(module)
}
