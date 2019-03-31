//! Insertion and removal for a block container.

use super::super::*;
use super::*;
use style::*;

pub struct BlockInside;

impl BlockInside {
    /// Whether this node has any inline children.
    ///
    /// This is cheap to test since if we have any inline children, then all our
    /// children have to be inline.
    fn children_inline(tree: &LayoutTree, block: LayoutNodeId) -> bool {
        let child = match tree[block].first_child() {
            Some(c) => c,
            None => return false,
        };
        tree[child].style.display.is_inline_outside()
    }

    fn inline_wrapper(tree: &mut LayoutTree) -> LayoutNodeId {
        tree.alloc(LayoutNode::new_container(
            ComputedStyle::for_inline_inside_block_wrapper(),
            ContainerKind::Block,
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

        let ip = tree.legalize_insertion_point(ip);
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
        let children_inline = has_children && Self::children_inline(tree, ip.parent);

        // Easy case, we just need to insert in the right place, since we either
        // are an inline-formatting-context and we're inserting an inline, or we
        // have only non-inlines and we're inserting an inline.
        if !has_children || children_inline == node.style.display.is_inline_outside() {
            return Some(tree.legalize_insertion_point(ip));
        }

        if children_inline {
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
        let ip = tree.legalize_insertion_point(ip);
        Some(Self::create_anon_block_for_single_inline(tree, ip))
    }
}
