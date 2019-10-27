//! Insertions and removals into an inline-inside element.

#![allow(unused)]

use super::super::*;
use super::*;
use crate::style::*;

pub struct InlineInside;

impl InlineInside {
    fn block_wrapper(tree: &mut LayoutTree) -> LayoutNodeId {
        tree.alloc(LayoutNode::new_container(
            ComputedStyle::for_ib_split_block_wrapper(),
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

    pub fn detach_from_ib_split_block_wrapper(
        tree: &mut LayoutTree,
        block_wrapper: LayoutNodeId,
        node: LayoutNodeId,
    ) -> InsertionPoint {
        let ip = tree.detach_unchecked(node).unwrap();

        if tree[block_wrapper].has_children() {
            return ip;
        }

        // TODO: Remove wrapper and merge following inline into previous inline.
        //
        // If there are no more inlines, then remove ib-split wrapper entirely.

        unimplemented!()
    }

    /// Removes from a non-anonymous inline.
    pub fn detach(
        tree: &mut LayoutTree,
        parent: LayoutNodeId,
        node_to_remove: LayoutNodeId,
    ) -> InsertionPoint {
        assert!(!tree[parent].is_anonymous());

        // NOTE(emilio): I think there's nothing special to do here if we're
        // split. In that case, we have a block sibling and arbitrary
        // continuations, but the original inline box needs to remain (even if
        // empty).
        tree.detach_unchecked(node_to_remove).unwrap()
    }

    /// Ensures that an inline is set up so that we can contain blocks, that is:
    ///
    ///  * Wraps the inline itself inside an ano

    /// Inserts a node inside an inline-inside container.
    pub fn insertion(
        tree: &mut LayoutTree,
        node: &LayoutNode,
        ip: InsertionPoint,
    ) -> Option<InsertionPoint> {
        assert!(tree[ip.parent].is_inline());
        assert!(!tree[ip.parent].is_anonymous());
        // Easy case: we're not inserting a block inside an inline.
        if !node.style.display.is_block_outside() {
            return Some(tree.legalize_insertion_point(ip));
        }

        if let Some(prev_sibling) = ip.prev_sibling {
            // If our previous sibling is also block-outside, then we're good
            // too, as we have the right container necessarily.
            if tree[prev_sibling].style.display.is_block_outside() {
                let parent = tree[prev_sibling].parent.unwrap();
                return Some(InsertionPoint {
                    parent,
                    prev_sibling: Some(prev_sibling),
                });
            }



            // Otherwise we need to find the wrapper of our next sibling or
            // create it if any.
            // Self::find_or_insert_block_wrapper_after(tree, prev_sibling)
        }

        // unimplemented!("IB Split fun")

        None
    }
}
