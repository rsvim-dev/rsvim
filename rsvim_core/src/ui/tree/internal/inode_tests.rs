use super::inode::*;
use crate::ui::tree::*;
// use crate::tests::log::init as test_log_init;
use taffy::Style;

// Test node
#[derive(Clone, Debug, rsvim_macro::Inodify)]
struct TestNode {
  #[inode_base]
  pub __node: InodeBase,
}

impl TestNode {
  pub fn new(id: NodeId, ctx: TreeContextWk) -> Self {
    Self {
      __node: InodeBase::new(id, ctx),
    }
  }
}

#[test]
fn new() {
  // test_log_init();

  let mut ctx = TreeContext::new();
  let style = Style {
    size: taffy::Size {
      width: taffy::prelude::length(1.0_f32),
      height: taffy::prelude::length(1.0_f32),
    },
    ..Default::default()
  };

  // Root
  let nid1 = ctx.new_leaf_default(style.clone(), "n1").unwrap();

  // Non-root
  let nid2 = ctx.new_leaf_default(style.clone(), "n2").unwrap();

  ctx.compute_layout(nid1).unwrap();

  let ctx = TreeContext::to_rc(ctx);

  let n1 = TestNode::new(nid1, Rc::downgrade(&ctx));
  let n2 = TestNode::new(nid2, Rc::downgrade(&ctx));

  assert_eq!(nid1, ctx.borrow().root());
  assert!(n1.id() < n2.id());
  assert_eq!(n1.id(), nid1);
  assert_eq!(n2.id(), nid2);
  assert!(n1.enabled());
  assert!(!n2.enabled());
}
