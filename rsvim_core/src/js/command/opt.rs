//! Ex command options.

use compact_str::CompactString;

#[derive(Debug, Clone, PartialEq, Eq, derive_builder::Builder)]
#[cfg_attr(feature = "js-v8", derive(rsvim_macro::ToV8, rsvim_macro::FromV8))]
pub struct CommandOptions {
  #[builder(default = true)]
  pub force: bool,

  #[builder(default = None)]
  pub alias: Option<CompactString>,
}
