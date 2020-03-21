//! Insertion and removal for a block container.

use super::super::*;
use super::*;
use crate::style::*;

pub struct BlockInside;

/// For a given insertion point of non-anonymous nodes, find the actual
/// insertion point that we should use, by adjusting the previous sibling in
/// order to "escape" from any anonymous wrapper, or walk outside of our
/// ib-split continuations.
///
/// TODO(emilio): Maybe find a better name for this?
pub fn legalize_insertion_point(tree: &LayoutTree, ip: InsertionPoint) -> InsertionPoint {
    let InsertionPoint {
        parent,
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
        assert_eq!(tree[maybe_parent].style.pseudo, Some(PseudoElement::InlineInsideBlockWrapper));
        prev_sibling = maybe_parent;
    }
    InsertionPoint {
        parent,
        prev_sibling: Some(prev_sibling),
    }
}

impl BlockInside {
    fn inline_wrapper(tree: &mut LayoutTree) -> LayoutNodeId {
        tree.alloc(LayoutNode::new_container(
            ComputedStyle::for_inline_inside_block_wrapper(),
            ContainerKind::block(),
        ))
    }

    fn wrap_inlines_in_anon_blocks(tree: &mut LayoutTree, ip: InsertionPoint) -> InsertionPoint {
        let trailing_anon_block = Self::inline_wrapper(tree);
        // Move all our inline children after ip.prev_sibling to an anonymous
        // block.
        tree.move_children_to(ip.parent, trailing_anon_block, ip.prev_sibling);
        if ip.prev_sibling.is_none() {
            tree.insert_unchecked(trailing_anon_block, ip);
            return ip;
        }

        // Move the rest of the inline kids into another anon block.
        let heading_anon_block = Self::inline_wrapper(tree);
        tree.move_children_to(ip.parent, heading_anon_block, None);
        tree.insert_unchecked(
            heading_anon_block,
            InsertionPoint {
                parent: ip.parent,
                prev_sibling: None,
            },
        );

        tree.insert_unchecked(
            trailing_anon_block,
            InsertionPoint {
                parent: ip.parent,
                prev_sibling: Some(heading_anon_block),
            },
        );

        InsertionPoint {
            parent: ip.parent,
            prev_sibling: Some(heading_anon_block),
        }
    }

    fn create_anon_block_for_single_inline(
        tree: &mut LayoutTree,
        ip: InsertionPoint,
    ) -> InsertionPoint {
        let anon_block = Self::inline_wrapper(tree);
        tree.insert_unchecked(anon_block, ip);
        InsertionPoint {
            parent: anon_block,
            prev_sibling: None,
        }
    }

    fn is_inline_wrapper(tree: &LayoutTree, node: LayoutNodeId) -> bool {
        tree[node].style.pseudo == Some(PseudoElement::InlineInsideBlockWrapper)
    }

    fn find_block_for_inline_insertion(
        tree: &LayoutTree,
        ip: InsertionPoint,
    ) -> Option<InsertionPoint> {
        if let Some(prev_sibling) = ip.prev_sibling {
            // There should be no two contiguous anonymous blocks, so if the
            // previous sibling is inside an anonymous block, then we're good.
            let parent = tree[prev_sibling].parent.unwrap();
            if Self::is_inline_wrapper(tree, parent) {
                assert_eq!(tree[parent].parent, Some(ip.parent), "How?");
                return Some(InsertionPoint {
                    parent,
                    prev_sibling: Some(prev_sibling),
                });
            }
        }

        let ip = legalize_insertion_point(tree, ip);
        let next_sibling = match ip.prev_sibling {
            Some(prev) => tree[prev].next_sibling?,
            None => tree[ip.parent]
                .first_child()
                .expect("How did we determine that we need to wrap an inline kid?"),
        };

        if Self::is_inline_wrapper(tree, next_sibling) {
            return Some(InsertionPoint {
                parent: next_sibling,
                prev_sibling: None,
            });
        }

        None
    }

    /// Removes from a non-anonymous block.
    pub fn detach(
        tree: &mut LayoutTree,
        parent: LayoutNodeId,
        node_to_remove: LayoutNodeId,
    ) -> InsertionPoint {
        assert!(!tree[parent].is_anonymous());

        // TODO(emilio): Merge / remove anon boxes as needed if the node removed
        // is a block that is around two blocks-wrapping-inlines.
        tree.detach_unchecked(node_to_remove).unwrap()
    }

    /// Processes an insertion inside a block-inside container, and returns the
    /// new insertion point. Note that the effective child under this could be
    /// something else than `node` (if it gets wrapped due to it being an
    /// internal table part for example).
    pub fn insertion(
        tree: &mut LayoutTree,
        node: &LayoutNode,
        ip: InsertionPoint,
    ) -> Option<InsertionPoint> {
        assert!(tree[ip.parent].is_block_container());
        let has_children = tree[ip.parent].has_children();
        let inline_formatting_context = has_children && tree[ip.parent].establishes_ifc(tree);

        // Easy case, we just need to insert in the right place, since we either
        // are an inline-formatting-context and we're inserting an inline, or we
        // have only non-inlines and we're inserting an inline.
        if !has_children || inline_formatting_context == node.style.display.is_inline_outside() {
            return Some(legalize_insertion_point(tree, ip));
        }

        if inline_formatting_context {
            return Some(Self::wrap_inlines_in_anon_blocks(tree, ip));
        }

        // We're inserting an inline inside a block wrapper with more blocks.
        // Try to find a pre-existing anonymous block to insert ourselves into,
        // or otherwise wrap ourselves into an anonymous block.
        if let Some(ip) = Self::find_block_for_inline_insertion(tree, ip) {
            return Some(ip);
        }

        // We're an inline, and our previous or next sibling is a block, so
        // gotta wrap ourselves into an anonymous block.
        let ip = legalize_insertion_point(tree, ip);
        Some(Self::create_anon_block_for_single_inline(tree, ip))
    }
}
