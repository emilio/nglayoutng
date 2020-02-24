//! Insertions and removals into an inline-inside element.

#![allow(unused)]

use super::super::*;
use super::*;
use crate::style::*;

pub struct InlineInside;

impl InlineInside {
    fn inline_continuation(tree: &LayoutTree, of: LayoutNodeId) -> LayoutNode {
        // TODO(emilio): Should we somehow tag it as anonymous? Can we
        // otherwise know when to tear it down?
        let style = tree[of].style.clone();
        LayoutNode::new_container(style, ContainerKind::Inline)
    }

    fn block_wrapper() -> LayoutNode {
        LayoutNode::new_container(
            ComputedStyle::for_ib_split_block_wrapper(),
            ContainerKind::Block,
        )
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

    fn legalize_insertion_point(tree: &LayoutTree, ip: InsertionPoint) -> InsertionPoint {
        assert!(tree[ip.parent].is_inline());

        let InsertionPoint {
            mut parent,
            prev_sibling,
        } = ip;
        let mut prev_sibling = match prev_sibling {
            Some(s) => s,
            None => return ip,
        };

        loop {
            let maybe_parent = tree[prev_sibling].parent.unwrap();
            if maybe_parent == parent {
                break;
            }

            if tree[maybe_parent].is_inline() {
                parent = maybe_parent;
                break;
            }

            let pseudo = tree[maybe_parent].style.pseudo.expect("Expected an anonymous box");
            assert_eq!(pseudo, PseudoElement::BlockInsideInlineWrapper);

            // Insert in the following inline, which must be inside an
            // InlineInsideBlock wrapper managed by the containing block.
            let next_sibling =
                tree[maybe_parent].next_sibling.expect("There should always be a trailing inline in a block-inside wrapper");

            assert_eq!(tree[next_sibling].style.pseudo, Some(PseudoElement::InlineInsideBlockWrapper));

            let next_sibling_first_child = tree[next_sibling].first_child().expect("There should always be a trailing inline-inside-inline");
            assert!(tree[next_sibling_first_child].is_inline());

            return InsertionPoint {
                parent: next_sibling_first_child,
                prev_sibling: None,
            };
        }


        InsertionPoint {
            parent,
            prev_sibling: Some(prev_sibling),
        }
    }

    /// Inserts a node inside an inline-inside container.
    pub fn insertion(
        tree: &mut LayoutTree,
        node: &LayoutNode,
        ip: InsertionPoint,
    ) -> Option<InsertionPoint> {
        assert!(tree[ip.parent].is_inline());
        assert!(!tree[ip.parent].is_anonymous());
        // Easy case: we're not inserting a block inside an inline, we just need
        // to find the right continuation to append to, if any.
        if !node.style.display.is_block_outside() {
            return Some(Self::legalize_insertion_point(tree, ip));
        }

        let continuation_to_split = match ip.prev_sibling {
            Some(prev_sibling) => tree[prev_sibling].parent.unwrap(),
            None => ip.parent,
        };

        // Assert the thing is actually a continuation of ours.
        if cfg!(debug_assertions) {
            let mut current = Some(continuation_to_split);
            let mut found = false;
            while let Some(cur) = current {
                assert!(tree[cur].is_inline() || tree[cur].style.pseudo == Some(PseudoElement::BlockInsideInlineWrapper));
                if cur == ip.parent {
                    found = true;
                    break;
                }
                current = tree[cur].prev_sibling;
            }
            assert!(found, "Didn't found continuation of ib-split");
        }

        // We're going to need a block wrapper and an inline continuation for
        // this. Behold.
        let grandparent = tree[ip.parent].parent.unwrap();
        let block_wrapper = {
            let wrapper = Self::block_wrapper();
            let insertion_point = InsertionPoint {
                parent: grandparent,
                prev_sibling: Some(continuation_to_split),
            };
            // Note that this takes care of splitting ancestor inlines as
            // needed.
            tree.insert(wrapper, insertion_point).unwrap()
        };

        println!("Created block wrapper for {:?}", ip.parent);
        tree.print_with_ids();

        let continuation = {
            let continuation = Self::inline_continuation(tree, ip.parent);
            let insertion_point = InsertionPoint {
                parent: grandparent,
                prev_sibling: Some(block_wrapper),
            };
            tree.insert(continuation, insertion_point).unwrap()
        };

        println!("Created block wrapper and continuation for {:?}", ip.parent);
        tree.print_with_ids();


        tree.move_children_to(continuation_to_split, continuation, ip.prev_sibling);

        Some(InsertionPoint {
            parent: block_wrapper,
            prev_sibling: None,
        })
    }
}
