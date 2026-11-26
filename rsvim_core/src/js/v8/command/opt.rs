//! Ex command options.

use compact_str::CompactString;

#[derive(
  Debug,
  Clone,
  PartialEq,
  Eq,
  derive_builder::Builder,
  rsvim_macro::ToV8,
  rsvim_macro::FromV8,
)]
pub struct CommandOptions {
  #[builder(default = true)]
  pub force: bool,

  #[builder(default = None)]
  pub alias: Option<CompactString>,
}
