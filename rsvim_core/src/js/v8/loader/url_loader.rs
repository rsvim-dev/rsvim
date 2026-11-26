//! URL module loader.
//!
//! This module can be loaded in async, i.e. it can be first downloaded in local filesystem async,
//! then loaded into js runtime sync.

// use crate::js::module::CORE_MODULES;
// use crate::js::module::ModulePath;
// use crate::js::module::ModuleSource;
// use crate::js::transpiler::Jsx;
// use crate::js::transpiler::TypeScript;
// use crate::js::transpiler::Wasm;
// use crate::prelude::*;
//
// use anyhow::bail;
// use regex::Regex;
// use sha::sha1::Sha1;
// use sha::utils::Digest;
// use sha::utils::DigestExt;
// use std::fs;
// use url::Url;

// lazy_static! {
//     // Use local cache directory in development.
//     pub static ref CACHE_DIR: PathBuf = if cfg!(debug_assertions) {
//         PathBuf::from(".cache")
//     } else {
//         dirs::home_dir().unwrap().join(".dune/cache")
//     };
// }
//
// #[derive(Default)]
// /// Loader supporting URL imports.
// pub struct UrlModuleLoader {
//   // Ignores the cache and re-downloads the dependency.
//   pub skip_cache: bool,
// }
//
// impl ModuleLoader for UrlModuleLoader {
//   fn resolve(&self, base: Option<&str>, specifier: &str) -> AnyResult<ModulePath> {
//     // 1. Check if specifier is a valid URL.
//     if let Ok(url) = Url::parse(specifier) {
//       return Ok(url.into());
//     }
//
//     // 2. Check if the requester is a valid URL.
//     if let Some(base) = base {
//       if let Ok(base) = Url::parse(base) {
//         let options = Url::options();
//         let url = options.base_url(Some(&base));
//         let url = url.parse(specifier)?;
//
//         return Ok(url.as_str().to_string());
//       }
//     }
//
//     // Possibly unreachable error.
//     bail!("Base is not a valid URL");
//   }
//
//   fn load(&self, specifier: &str) -> AnyResult<ModuleSource> {
//     // Create the cache directory.
//     if fs::create_dir_all(CACHE_DIR.as_path()).is_err() {
//       bail!("Failed to create module caching directory");
//     }
//
//     // Hash URL using sha1.
//     let hash = Sha1::default().digest(specifier.as_bytes()).to_hex();
//     let module_path = CACHE_DIR.join(hash);
//
//     if !self.skip_cache {
//       // Check cache, and load file.
//       if module_path.is_file() {
//         let source = fs::read_to_string(&module_path).unwrap();
//         return Ok(source);
//       }
//     }
//
//     println!("{} {}", "Downloading".green(), specifier);
//
//     // Download file and, save it to cache.
//     let source = match ureq::get(specifier).call()?.into_string() {
//       Ok(source) => source,
//       Err(_) => bail!(format!("Module not found \"{specifier}\"")),
//     };
//
//     // Use a preprocessor if necessary.
//     let source = match (
//       specifier.ends_with(".wasm"),
//       specifier.ends_with(".jsx"),
//       specifier.ends_with(".ts"),
//       specifier.ends_with(".tsx"),
//     ) {
//       (true, _, _, _) => Wasm::parse(&source),
//       (_, true, _, _) => Jsx::compile(Some(specifier), &source)?,
//       (_, _, true, _) => TypeScript::compile(Some(specifier), &source)?,
//       (_, _, _, true) => Jsx::compile(Some(specifier), &source)
//         .and_then(|output| TypeScript::compile(Some(specifier), &output))?,
//       _ => source,
//     };
//
//     fs::write(&module_path, &source)?;
//
//     Ok(source)
//   }
// }
