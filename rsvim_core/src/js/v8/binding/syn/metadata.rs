//! Tree-sitter parser metadata

use compact_str::CompactString;
use compact_str::ToCompactString;

#[derive(
  Debug, Clone, PartialEq, Eq, derive_builder::Builder, rsvim_macro::ToV8,
)]
pub struct SynTreeSitterParserMetadata {
  #[builder(default = "".to_compact_string())]
  pub name: CompactString,

  #[builder(default = "".to_compact_string())]
  pub camelcase: CompactString,

  #[builder(default = "".to_compact_string())]
  pub scope: CompactString,

  #[builder(default = "".to_compact_string())]
  pub path: CompactString,

  #[builder(default = vec![])]
  pub file_types: Vec<CompactString>,

  #[builder(default = None)]
  pub highlights_path: Option<CompactString>,

  #[builder(default = None)]
  pub highlights_query: Option<String>,

  #[builder(default = None)]
  pub tags_path: Option<CompactString>,

  #[builder(default = None)]
  pub tags_query: Option<String>,

  #[builder(default = None)]
  pub injections_path: Option<CompactString>,

  #[builder(default = None)]
  pub injections_query: Option<String>,

  #[builder(default = None)]
  pub injection_regex: Option<String>,
}
