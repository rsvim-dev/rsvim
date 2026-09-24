//! The node structure of the internal tree.

use crate::prelude::*;
use crate::ui::tree::internal::context::TreeContextWk;
use crate::ui::tree::internal::context::TruncatePolicy;
use std::fmt::Debug;

// NodeId starts from 1001
#[derive(
  Copy, Clone, rsvim_macro::IncrementalId, serde::Serialize, serde::Deserialize,
)]
pub struct NodeId(#[start_from(1001)] i32);

pub trait Inodify: Sized + Clone + Debug {
  fn id(&self) -> NodeId;

  fn shape(&self) -> IRect;

  fn actual_shape(&self) -> U16Rect;

  fn zindex(&self) -> usize;

  fn enabled(&self) -> bool;

  fn truncate_policy(&self) -> TruncatePolicy;
}

#[derive(Debug, Clone)]
pub struct InodeBase {
  id: NodeId,
  ctx: TreeContextWk,
}

impl InodeBase {
  pub fn new(id: NodeId, ctx: TreeContextWk) -> Self {
    Self { id, ctx }
  }

  pub fn context(&self) -> TreeContextWk {
    self.ctx.clone()
  }
}

impl Inodify for InodeBase {
  fn id(&self) -> NodeId {
    self.id
  }

  fn shape(&self) -> IRect {
    self
      .ctx
      .upgrade()
      .unwrap()
      .borrow()
      .shape(self.id)
      .copied()
      .unwrap()
  }

  fn actual_shape(&self) -> U16Rect {
    self
      .ctx
      .upgrade()
      .unwrap()
      .borrow()
      .actual_shape(self.id)
      .copied()
      .unwrap()
  }

  fn zindex(&self) -> usize {
    self
      .ctx
      .upgrade()
      .unwrap()
      .borrow()
      .zindex(self.id)
      .unwrap()
  }

  fn enabled(&self) -> bool {
    self
      .ctx
      .upgrade()
      .unwrap()
      .borrow()
      .enabled(self.id)
      .unwrap()
  }

  fn truncate_policy(&self) -> TruncatePolicy {
    self
      .ctx
      .upgrade()
      .unwrap()
      .borrow()
      .truncate_policy(self.id)
      .unwrap()
  }
}
