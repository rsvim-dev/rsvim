//! Ex command options.

use compact_str::CompactString;

#[derive(Debug, Clone, PartialEq, Eq, derive_builder::Builder)]
pub struct CommandOptions {
  #[builder(default = true)]
  pub force: bool,

  #[builder(default = None)]
  pub alias: Option<CompactString>,
}
