//! Insertions and removals into an inline-inside element.

#![allow(unused)]

use super::super::*;
use super::*;
use crate::style::*;

pub struct InlineInside;

impl InlineInside {
    fn create_ib_split_anonymous_block(tree: &mut LayoutTree) -> LayoutNodeId {
        tree.alloc(LayoutNode::new_container(
            ComputedStyle::for_ib_split_wrapper(),
            ContainerKind::Block,
        ))
    }

    fn is_already_split(tree: &LayoutTree, inline: LayoutNodeId) -> bool {
        let inline = &tree[inline];
        assert!(inline.is_inline());
        assert!(!inline.is_anonymous());
        let parent = inline.parent(tree).unwrap();
        if !parent.is_anonymous() {
            return false;
        }
        let sibling = match parent.next_sibling(tree) {
            Some(s) => s,
            None => return false,
        };
        sibling.style.pseudo == Some(PseudoElement::BlockInsideInlineWrapper)
    }

    /// Inserts a node inside an inline-inside container.
    pub fn insertion(
        tree: &mut LayoutTree,
        node: &LayoutNode,
        ip: InsertionPoint,
    ) -> Option<InsertionPoint> {
        assert!(tree[ip.parent].is_inline());
        // Easy case: we're not inserting a block inside an inline.
        if !node.style.display.is_block_outside() {
            return Some(tree.legalize_insertion_point(ip));
        }

        // unimplemented!("IB split fun");
        None
    }
}
