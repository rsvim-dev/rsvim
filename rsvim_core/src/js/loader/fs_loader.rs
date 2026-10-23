//! Fs (filesystem) module loader.

use crate::cfg::path_cfg::PATH_CONFIG;
use crate::js::loader::ModuleLoader;
use crate::js::module::ModulePath;
use crate::js::module::ModuleSource;
use crate::prelude::*;
use oxc_resolver::ResolveOptions;
use oxc_resolver::Resolver;

/// Checks if path is a JSON file.
fn is_json_import(path: &Path) -> bool {
  path
    .extension()
    .map(|value| value == "json")
    .unwrap_or(false)
}

/// Wraps JSON data into an ES module (using v8's built in objects).
fn wrap_json(source: &str) -> String {
  format!("export default JSON.parse(`{source}`);")
}

mod sync_load {
  use super::*;

  /// Loads contents from a file.
  pub fn load_source(path: &Path) -> TheResult<ModuleSource> {
    match std::fs::read_to_string(path) {
      Ok(source) => {
        let source = if is_json_import(path) {
          wrap_json(source.as_str())
        } else {
          source
        };
        Ok(source)
      }
      Err(e) => Err(TheErr::LoadModuleFailed(
        path.to_string_lossy().to_string(),
        e,
      )),
    }
  }

  /// Loads import as file.
  pub fn load_as_file(path: &Path) -> TheResult<(PathBuf, ModuleSource)> {
    // If path is a file.
    if path.is_file() {
      return match load_source(path) {
        Ok(source) => Ok((path.to_path_buf(), source)),
        Err(e) => Err(e),
      };
    }

    Err(TheErr::ModuleNotFound(path.to_string_lossy().to_string()))
  }
}

#[allow(dead_code)]
#[derive(Default)]
/// Fs (filesystem) module loader.
pub struct FsModuleLoader {
  resolver: Resolver,
}

fn create_resolve_opts() -> ResolveOptions {
  ResolveOptions {
    extensions: vec![
      ".js".into(),
      ".ts".into(),
      ".mjs".into(),
      ".json".into(),
      ".wasm".into(),
    ],
    extension_alias: vec![
      (".js".into(), vec![".js".into()]),
      (".mjs".into(), vec![".mjs".into()]),
      (".ts".into(), vec![".ts".into()]),
      (".json".into(), vec![".json".into()]),
      (".wasm".into(), vec![".wasm".into()]),
    ],
    modules: vec![
      PATH_CONFIG.config_home().to_string_lossy().to_string(),
      PATH_CONFIG
        .config_home()
        .join("node_modules")
        .to_string_lossy()
        .to_string(),
      "node_modules".to_string(),
    ],
    // builtin_modules: false,
    ..ResolveOptions::default()
  }
}

impl FsModuleLoader {
  pub fn new() -> Self {
    Self {
      resolver: Resolver::new(create_resolve_opts()),
    }
  }
}

impl FsModuleLoader {
  fn resolve_impl(
    &self,
    resolver: &Resolver,
    base: &str,
    specifier: &str,
  ) -> TheResult<ModulePath> {
    let base = Path::new(base).to_path_buf();
    trace!(
      "|FsModuleLoader::resolve| base:{:?}, specifier:{:?}",
      base, specifier
    );

    match resolver.resolve(&base, specifier) {
      Ok(resolution) => Ok(resolution.path().to_string_lossy().to_string()),
      Err(_) => Err(TheErr::ModuleNotFound(specifier.to_string())),
    }
  }
}

impl ModuleLoader for FsModuleLoader {
  #[cfg(not(test))]
  /// Resolve module path by specifier in local filesystem.
  ///
  /// It tries to resolve npm packages, thus we can directly use npm registry to maintain plugins.
  /// But node/npm have quite a history, it requires quite an effort to be fully compatible with,
  /// we only choose to maintain a small subset (at least for now):
  ///
  /// 1. The "common js" standard is not supported.
  /// 2. The `cjs` file extension is not supported.
  /// 3. The `require` keyword is not supported.
  ///
  /// For more details about node/npm package, please see: <https://nodejs.org/api/packages.html>.
  fn resolve(&self, base: &str, specifier: &str) -> TheResult<ModulePath> {
    self.resolve_impl(&self.resolver, base, specifier)
  }

  #[cfg(test)]
  fn resolve(&self, base: &str, specifier: &str) -> TheResult<ModulePath> {
    let resolver = Resolver::new(create_resolve_opts());
    self.resolve_impl(&resolver, base, specifier)
  }

  /// Load module source by its module path, it can be either a file path, or a directory path.
  fn load(&self, specifier: &str) -> TheResult<ModuleSource> {
    // Load source.
    let path = Path::new(specifier);
    let maybe_source = sync_load::load_as_file(path)?;
    Ok(maybe_source.1)
  }
}
