//! Engine-neutral pieces of the syntax API.

use compact_str::CompactString;
use compact_str::ToCompactString;

/// The argument `syn.loadParser` / `syn.loadParserSync` takes.
///
/// Grammar loading itself is pure Rust — it walks a `path` and asks the syntax
/// manager to build a tree-sitter grammar. Only the shape of the javascript
/// argument differed between the engines, so it lives here.
#[derive(Debug, Clone, PartialEq, Eq, derive_builder::Builder)]
#[cfg_attr(feature = "js-v8", derive(rsvim_macro::ToV8, rsvim_macro::FromV8))]
pub struct SynLoadTreeSitterParserOptions {
  #[builder(default = "".to_compact_string())]
  pub grammar_path: CompactString,
}
