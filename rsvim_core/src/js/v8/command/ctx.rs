//! Ex command runtime context.

use crate::buf::BufferId;
use crate::ui::tree::NodeId;
use compact_str::CompactString;

#[derive(
  Debug, Clone, PartialEq, Eq, derive_builder::Builder, rsvim_macro::ToV8,
)]
pub struct CommandContext {
  #[builder(default = false)]
  // bang
  pub bang: bool,

  #[builder(default = vec![])]
  pub args: Vec<CompactString>,

  #[builder(default = BufferId::negative_one())]
  pub current_buffer_id: BufferId,

  #[builder(default = NodeId::negative_one())]
  pub current_window_id: NodeId,
}
