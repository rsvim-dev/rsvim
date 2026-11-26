//! Import map, it is passed by users to customize the import specifiers, it is actually key-Value
//! entries representing WICG import-maps.
//!
//! For example, if user provide such a import map:
//!
//! ```javascript
//! const importMap = {'lodash': 'react'};
//! ```
//!
//! Pass the `importMap` to javascript runtime, then when user writes such javacsripts:
//!
//! ```javascript
//! const _ = import "lodash";
//! ```
//!
//! The javascript runtime will actually load the `react` module instead of the `lodash`.
//!
//! See: <https://github.com/WICG/import-maps>.

use crate::prelude::*;

/// A single import mapping (specifier, target).
pub type ImportMapEntry = (String, String);

/// Key-Value entries representing WICG import-maps.
///
/// NOTE: This is just a mock-up which is actually not supported.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ImportMap {
  map: Vec<ImportMapEntry>,
}

impl ImportMap {
  pub fn parse_from_json(_text: &str) -> TheResult<ImportMap> {
    Ok(ImportMap { map: Vec::new() })
  }

  pub fn lookup(&self, _specifier: &str) -> Option<String> {
    None
  }

  // /// Creates an ImportMap from JSON text.
  // pub fn parse_from_json(text: &str) -> AnyResult<ImportMap> {
  //   // Parse JSON string into serde value.
  //   let json: serde_json::Value = serde_json::from_str(text)?;
  //   let imports = json["imports"].to_owned();
  //
  //   if imports.is_null() || !imports.is_object() {
  //     return Err(anyhow::anyhow!("Import map's 'imports' must be an object"));
  //   }
  //
  //   let map: FoldMap<String, String> = serde_json::from_value(imports)?;
  //   let mut map: Vec<ImportMapEntry> = Vec::from_iter(map);
  //
  //   // Note: We're sorting the imports because we need to support "Packages"
  //   // via trailing slashes, so the lengthier mapping should always be selected.
  //   //
  //   // https://github.com/WICG/import-maps#packages-via-trailing-slashes
  //
  //   map.sort_by(|a, b| b.0.cmp(&a.0));
  //
  //   Ok(ImportMap { map })
  // }
  //
  // /// Tries to match a specifier against an import-map entry.
  // pub fn lookup(&self, specifier: &str) -> Option<String> {
  //   // Find a mapping if exists.
  //   let (base, mut target) = match self.map.iter().find(|(k, _)| specifier.starts_with(k)) {
  //     Some(mapping) => mapping.to_owned(),
  //     None => return None,
  //   };
  //
  //   // The following code treats "./" as an alias for the CWD.
  //   if target.starts_with("./") {
  //     let cwd = env::current_dir().unwrap().to_string_lossy().to_string();
  //     target = target.replacen('.', &cwd, 1);
  //   }
  //
  //   // Note: The reason we need this additional check below with the specifier's
  //   // extension (if exists) is to be able to support extension-less imports.
  //   //
  //   // https://github.com/WICG/import-maps#extension-less-imports
  //
  //   match Path::new(specifier).extension() {
  //     Some(ext) => match Path::new(specifier) == Path::new(&base).with_extension(ext) {
  //       false => Some(specifier.replacen(&base, &target, 1)),
  //       _ => None,
  //     },
  //     None => Some(specifier.replacen(&base, &target, 1)),
  //   }
  // }
}
