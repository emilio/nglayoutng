pub mod builder;

use self::builder::InsertionPoint;
use crate::allocator;
use crate::layout_algorithms::{ConstraintSpace, GenericLayoutResult, LayoutContext};
use crate::logical_geometry;
use crate::misc::print_tree::PrintTree;
use crate::style::{self, ComputedStyle, Display, PseudoElement};
use app_units::Au;
use euclid::Size2D;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct LayoutNodeId(usize);

#[derive(Debug)]
pub enum LeafKind {
    Text { text: Box<str> },
    Replaced { intrinsic_size: Size2D<Au> },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ContainerKind {
    Block,
    Inline,
}

#[derive(Debug)]
pub enum LayoutNodeKind {
    Leaf {
        kind: LeafKind,
    },
    Container {
        first_child: Option<LayoutNodeId>,
        last_child: Option<LayoutNodeId>,
        // TODO(emilio): Put OOFs parented to me in here? Or collect them during
        // layout?
        kind: ContainerKind,
    },
}

/// A display node is a node in the display tree, which contains the primary box
/// of each element.
///
/// A display node is the primary box of an element, but contains no layout
/// information, that's left to fragment.
///
/// This is the CSS2 concept of "box", minus dimensions.
#[derive(Debug)]
pub struct LayoutNode {
    pub style: ComputedStyle,
    pub parent: Option<LayoutNodeId>,
    pub next_sibling: Option<LayoutNodeId>,
    pub prev_sibling: Option<LayoutNodeId>,
    pub kind: LayoutNodeKind,
}

impl LayoutNode {
    fn new(style: ComputedStyle, kind: LayoutNodeKind) -> Self {
        Self {
            style,
            parent: None,
            prev_sibling: None,
            next_sibling: None,
            kind,
        }
    }

    pub fn layout(
        &self,
        _context: &LayoutContext,
        _constraints: &ConstraintSpace,
    ) -> GenericLayoutResult {
        // TODO
        unimplemented!()
    }

    fn is_anonymous(&self) -> bool {
        self.style.pseudo.map_or(false, |p| p.is_anonymous())
    }

    pub fn is_container(&self) -> bool {
        self.container_kind().is_some()
    }

    pub fn is_block_container(&self) -> bool {
        self.container_kind()
            .map_or(false, |k| k == ContainerKind::Block)
    }

    pub fn is_inline(&self) -> bool {
        self.container_kind()
            .map_or(false, |k| k == ContainerKind::Inline)
    }

    fn container_kind(&self) -> Option<ContainerKind> {
        match self.kind {
            LayoutNodeKind::Container { kind, .. } => Some(kind),
            LayoutNodeKind::Leaf { .. } => None,
        }
    }

    // 9.3 Positioning schemes

    pub fn is_floating(&self) -> bool {
        self.style.is_floating()
    }

    pub fn is_out_of_flow_positioned(&self) -> bool {
        self.style.is_out_of_flow_positioned()
    }

    pub fn is_out_of_flow(&self) -> bool {
        self.style.is_out_of_flow()
    }

    pub fn is_in_flow(&self) -> bool {
        !self.is_out_of_flow()
    }

    /// https://drafts.csswg.org/css2/visuren.html#block-formatting
    ///
    /// > Floats, absolutely positioned elements, block containers (such as
    /// > inline-blocks, table-cells, and table-captions) that are not block
    /// > boxes, and block boxes with 'overflow' other than 'visible' (except
    /// > when that value has been propagated to the viewport) establish new
    /// > block formatting contexts for their contents.
    pub fn establishes_bfc(&self) -> bool {
        use crate::style::Overflow;

        // The root always establishes an (initial) BFC.
        if self.parent.is_none() {
            return true;
        }

        if self.style.is_floating() || self.style.is_out_of_flow_positioned() {
            return true;
        }

        // These always stablish a new bfc.
        let display = self.display();
        if display == Display::InlineBlock || display == Display::FlowRoot {
            return true;
        }

        // Style guarantees that for the Visible case, overflow-x is equal to
        // overflow-y.
        //
        // TODO: Maybe implement the 'has been propagated to the viewport'
        // thing.
        //
        // TODO: Overflow::Clip doesn't cause a bfc either afaict.
        if display.is_block_outside() && self.style.overflow_x != Overflow::Visible {
            return true;
        }

        // TODO(emilio): Columns and such, step by step...
        false
    }

    fn ancestors<'tree>(&self, tree: &'tree LayoutTree) -> AncestorIterator<'tree> {
        AncestorIterator {
            tree,
            current: self.parent,
        }
    }

    fn is_containing_block_for(&self, _for_child: &Self) -> bool {
        unimplemented!()
    }

    pub fn containing_block_chain<'tree>(
        &'tree self,
        tree: &'tree LayoutTree,
    ) -> ContainingBlockIterator<'tree> {
        ContainingBlockIterator {
            ancestors: self.ancestors(tree),
            current: self,
        }
    }

    fn print_label(&self, _id: LayoutNodeId) -> String {
        let mut label = match self.kind {
            LayoutNodeKind::Container { ref kind, .. } => format!("{:?}", kind),
            LayoutNodeKind::Leaf { ref kind } => format!("{:?}", kind),
        };

        // label.push_str(&format!(" - {:?}", _id));

        if self.is_out_of_flow() {
            label.push_str(" (oof)");
        }

        if self.establishes_bfc() {
            label.push_str(" (bfc)");
        }

        if let Some(pseudo) = self.style.pseudo {
            label.push_str(&format!(" ({:?})", pseudo));
        }

        label
    }

    fn print(&self, tree: &LayoutTree, id: LayoutNodeId, printer: &mut PrintTree) {
        printer.new_level(self.print_label(id));
        for (id, child) in self.children_and_id(tree) {
            child.print(tree, id, printer);
        }
        printer.end_level();
    }

    pub fn new_leaf(style: ComputedStyle, kind: LeafKind) -> Self {
        Self::new(style, LayoutNodeKind::Leaf { kind })
    }

    pub fn new_container(style: ComputedStyle, kind: ContainerKind) -> Self {
        Self::new(
            style,
            LayoutNodeKind::Container {
                first_child: None,
                last_child: None,
                kind,
            },
        )
    }

    pub fn display(&self) -> Display {
        self.style.display
    }

    pub fn position(&self) -> style::Position {
        self.style.position
    }

    pub fn writing_mode(&self) -> logical_geometry::WritingMode {
        self.style.writing_mode
    }

    pub fn has_children(&self) -> bool {
        debug_assert_eq!(self.first_child().is_some(), self.last_child().is_some());
        self.first_child().is_some()
    }

    pub fn first_child(&self) -> Option<LayoutNodeId> {
        match self.kind {
            LayoutNodeKind::Container { first_child, .. } => first_child,
            LayoutNodeKind::Leaf { .. } => None,
        }
    }

    pub fn last_child(&self) -> Option<LayoutNodeId> {
        match self.kind {
            LayoutNodeKind::Container { last_child, .. } => last_child,
            LayoutNodeKind::Leaf { .. } => None,
        }
    }

    pub fn next_sibling<'tree>(&self, tree: &'tree LayoutTree) -> Option<&'tree LayoutNode> {
        Some(&tree[self.next_sibling?])
    }

    pub fn prev_sibling_id<'tree>(&self, tree: &'tree LayoutTree) -> Option<&'tree LayoutNode> {
        Some(&tree[self.prev_sibling?])
    }

    fn children_and_id<'tree>(
        &self,
        tree: &'tree LayoutTree,
    ) -> impl Iterator<Item = (LayoutNodeId, &'tree LayoutNode)> {
        Children {
            current: self.first_child(),
            tree,
            get_next: |node| node.next_sibling,
        }
    }

    fn rev_children_and_id<'tree>(
        &self,
        tree: &'tree LayoutTree,
    ) -> impl Iterator<Item = (LayoutNodeId, &'tree LayoutNode)> {
        Children {
            current: self.last_child(),
            tree,
            get_next: |node| node.prev_sibling,
        }
    }

    pub fn children<'tree>(
        &self,
        tree: &'tree LayoutTree,
    ) -> impl Iterator<Item = &'tree LayoutNode> {
        self.children_and_id(tree).map(|(_id, child)| child)
    }

    pub fn parent<'tree>(&self, tree: &'tree LayoutTree) -> Option<&'tree LayoutNode> {
        Some(&tree[self.parent?])
    }

    pub fn rev_children<'tree>(
        &self,
        tree: &'tree LayoutTree,
    ) -> impl Iterator<Item = &'tree LayoutNode> {
        self.rev_children_and_id(tree).map(|(_id, child)| child)
    }

    pub fn in_flow_children<'tree>(
        &self,
        tree: &'tree LayoutTree,
    ) -> impl Iterator<Item = &'tree LayoutNode> {
        self.children(tree).filter(|c| c.is_in_flow())
    }
}

/// An iterator over all the children of a node.
pub struct Children<'a, F>
where
    F: Fn(&LayoutNode) -> Option<LayoutNodeId>,
{
    tree: &'a LayoutTree,
    current: Option<LayoutNodeId>,
    get_next: F,
}

impl<'a, F> Iterator for Children<'a, F>
where
    F: Fn(&LayoutNode) -> Option<LayoutNodeId>,
{
    type Item = (LayoutNodeId, &'a LayoutNode);

    fn next(&mut self) -> Option<Self::Item> {
        let current_id = self.current.take()?;
        let current = &self.tree[current_id];
        let next = (self.get_next)(current);
        self.current = next;
        Some((current_id, current))
    }
}

#[derive(Debug)]
pub struct LayoutTree {
    nodes: allocator::Allocator<LayoutNode>,
    root: LayoutNodeId,
}

impl LayoutTree {
    pub fn new() -> Self {
        let root = LayoutNode::new_container(ComputedStyle::for_viewport(), ContainerKind::Block);

        let mut nodes = allocator::Allocator::default();
        let root = LayoutNodeId(nodes.allocate(root));

        Self { nodes, root }
    }

    pub fn assert_consistent(&self) {
        self.assert_subtree_consistent(self.root);
    }

    fn assert_subtree_consistent(&self, root: LayoutNodeId) {
        let mut expected_count = 0;

        let mut expected_prev_sibling = None;
        for (id, child) in self[root].children_and_id(self) {
            self.assert_subtree_consistent(id);
            assert_eq!(
                child.parent,
                Some(root),
                "Unexpected parent, child {:?} - {:?}",
                id,
                child
            );
            assert_eq!(
                child.prev_sibling, expected_prev_sibling,
                "Unexpected prev_sibling, child {:?} - {:#?} - {:#?}",
                id, child, self[root]
            );
            expected_prev_sibling = Some(id);
            expected_count += 1;
        }

        let mut expected_next_sibling = None;
        let mut reverse_count = 0;
        for (id, child) in self[root].rev_children_and_id(self) {
            assert_eq!(
                child.parent,
                Some(root),
                "Unexpected parent, child {:?} - {:?}",
                id,
                child
            );
            assert_eq!(
                child.next_sibling, expected_next_sibling,
                "Unexpected next_sibling, child {:?} - {:#?} - {:#?}",
                id, child, self[root]
            );
            expected_next_sibling = Some(id);
            reverse_count += 1;
        }

        assert_eq!(reverse_count, expected_count);

        match self[root].container_kind() {
            None => {},
            Some(ContainerKind::Block) => {
                let mut saw_inline = false;
                let mut saw_non_inline = false;
                for child in self[root].children(self) {
                    let inline = child.style.display.is_inline_outside();
                    if inline {
                        assert!(!saw_non_inline, "Mixed non-inlines and inlines in a block");
                    } else {
                        assert!(!saw_inline, "Mixed inlines and non-inlines in a block");
                    }

                    saw_inline |= inline;
                    saw_non_inline |= !inline;
                }
            },
            Some(ContainerKind::Inline) => {
                for child in self[root].children(self) {
                    assert!(
                        !child.style.display.is_block_outside(),
                        "Saw block inside an inline"
                    );
                }
            },
        }
    }

    pub fn root(&self) -> LayoutNodeId {
        self.root
    }

    pub fn root_node(&self) -> &LayoutNode {
        &self[self.root]
    }

    /// Allocates a node inside the tree. This node _must_ be inserted in the
    /// layout tree.
    #[must_use]
    pub fn alloc(&mut self, node: LayoutNode) -> LayoutNodeId {
        LayoutNodeId(self.nodes.allocate(node))
    }

    pub fn insert(&mut self, node: LayoutNode, ip: InsertionPoint) -> Option<LayoutNodeId> {
        debug_assert!(!node.is_anonymous());
        debug_assert!(
            !self[ip.parent].is_anonymous() ||
                self[ip.parent].style.pseudo == Some(PseudoElement::Viewport),
        );
        let container_kind = self[ip.parent].container_kind()?;
        let ip = match container_kind {
            ContainerKind::Inline => builder::inline::InlineInside::insertion(self, &node, ip)?,
            ContainerKind::Block => builder::block::BlockInside::insertion(self, &node, ip)?,
        };
        // TODO: add table wrappers as needed.
        let id = self.alloc(node);
        self.insert_unchecked(id, ip);
        Some(id)
    }

    pub fn move_children_to(
        &mut self,
        from_node: LayoutNodeId,
        to_node: LayoutNodeId,
        from_sibling: Option<LayoutNodeId>,
    ) {
        trace!(
            "move_children_to({:?}, {:?}, {:?})",
            from_node,
            to_node,
            from_sibling
        );
        let first_sibling_to_move = {
            let mut first_sibling_to_move = match from_sibling {
                Some(from_sibling) => self[from_sibling].next_sibling.take(),
                None => None,
            };

            match self[from_node].kind {
                LayoutNodeKind::Container {
                    ref mut first_child,
                    ref mut last_child,
                    ..
                } => {
                    *last_child = from_sibling;
                    if from_sibling.is_none() {
                        first_sibling_to_move = first_child.take();
                    }
                },
                LayoutNodeKind::Leaf { .. } => unreachable!(),
            };

            first_sibling_to_move
        };

        let mut current = first_sibling_to_move;
        while let Some(child) = current {
            // Un-parent the child, save next sibling so that we can
            // continue the loop.
            let child_prev_sibling = {
                let mut child = &mut self[child];
                assert_eq!(child.parent, Some(from_node));
                child.parent = None;

                current = child.next_sibling.take();
                child.prev_sibling.take()
            };

            let ip = InsertionPoint {
                parent: to_node,
                prev_sibling: if child_prev_sibling == from_sibling {
                    None
                } else {
                    child_prev_sibling
                },
            };
            self.insert_unchecked(child, ip);
        }
    }

    /// For a given insertion point of non-anonymous nodes, find the actual
    /// insertion point that we should use, by adjusting the previous sibling in
    /// order to "escape" from any anonymous wrapper, or walk outside of our
    /// ib-split continuations.
    ///
    /// TODO(emilio): Maybe find a better name for this?
    pub fn legalize_insertion_point(&self, ip: InsertionPoint) -> InsertionPoint {
        let InsertionPoint {
            parent,
            prev_sibling,
        } = ip;
        let mut prev_sibling = match prev_sibling {
            Some(s) => s,
            None => return ip,
        };
        loop {
            let maybe_parent = self[prev_sibling].parent.unwrap();
            if maybe_parent == parent {
                break;
            }
            assert!(self[maybe_parent].is_anonymous());
            prev_sibling = maybe_parent;
        }
        InsertionPoint {
            parent,
            prev_sibling: Some(prev_sibling),
        }
    }

    pub fn insert_unchecked(&mut self, node_id: LayoutNodeId, ip: InsertionPoint) {
        trace!("Inserting {:?} into {:?}", node_id, ip);
        self.assert_subtree_consistent(ip.parent);

        {
            let node = &self[node_id];
            assert!(node.parent.is_none());
            assert!(node.prev_sibling.is_none());
            assert!(node.next_sibling.is_none());
            if let Some(prev_sibling) = ip.prev_sibling {
                assert_eq!(self[prev_sibling].parent, Some(ip.parent));
            }
        }

        let new_next_sibling = match ip.prev_sibling {
            Some(prev_sibling) => self[prev_sibling].next_sibling,
            None => self[ip.parent].first_child(),
        };

        {
            let mut node = &mut self[node_id];
            node.parent = Some(ip.parent);
            node.prev_sibling = ip.prev_sibling;
            node.next_sibling = new_next_sibling;
        }

        if let Some(prev_sibling) = ip.prev_sibling {
            self[prev_sibling].next_sibling = Some(node_id);
        }

        if let Some(next_sibling) = new_next_sibling {
            self[next_sibling].prev_sibling = Some(node_id);
        }

        let parent = &mut self[ip.parent];
        match parent.kind {
            LayoutNodeKind::Container {
                ref mut first_child,
                ref mut last_child,
                ..
            } => {
                if ip.prev_sibling.is_none() {
                    *first_child = Some(node_id);
                }
                if *last_child == ip.prev_sibling {
                    *last_child = Some(node_id);
                }
            },
            LayoutNodeKind::Leaf { .. } => unreachable!(),
        }

        self.assert_subtree_consistent(ip.parent);
    }

    pub fn destroy(&mut self, node_to_remove: LayoutNodeId) {
        // TODO(emilio): This would have to clean up fragments and such from
        // other places.
        let mut removed_node = self.nodes.deallocate(node_to_remove.0);

        // Fix up the tree.
        if let Some(prev_sibling) = removed_node.prev_sibling {
            let prev_sibling = &mut self[prev_sibling];
            assert_eq!(prev_sibling.next_sibling, Some(node_to_remove));
            prev_sibling.next_sibling = removed_node.next_sibling;
        } else if let Some(parent) = removed_node.parent {
            let parent = &mut self[parent];
            assert_eq!(parent.first_child(), Some(node_to_remove));
            match parent.kind {
                LayoutNodeKind::Container {
                    ref mut first_child,
                    ..
                } => {
                    *first_child = removed_node.next_sibling;
                },
                LayoutNodeKind::Leaf { .. } => unreachable!(),
            }
        }

        if let Some(next_sibling) = removed_node.next_sibling {
            let next_sibling = &mut self[next_sibling];
            assert_eq!(next_sibling.prev_sibling, Some(node_to_remove));
            next_sibling.prev_sibling = removed_node.prev_sibling;
        } else if let Some(parent) = removed_node.parent {
            let parent = &mut self[parent];
            assert_eq!(parent.last_child(), Some(node_to_remove));
            match parent.kind {
                LayoutNodeKind::Container {
                    ref mut last_child, ..
                } => {
                    *last_child = removed_node.prev_sibling;
                },
                LayoutNodeKind::Leaf { .. } => unreachable!(),
            }
        }

        // TODO(emilio): We may want / need the children to have access to the
        // parent chain, when we come up with something for OOFs, in which case
        // we should clean it up at the end of this function.
        removed_node.next_sibling = None;
        removed_node.prev_sibling = None;
        removed_node.parent = None;

        // Now recursively tear down the children.
        let mut child = removed_node.first_child();
        while let Some(child_to_remove) = child.take() {
            child = self[child_to_remove].next_sibling;
            self.destroy(child_to_remove);
        }
    }

    /// Prints the layout tree to stdout.
    pub fn print(&self) {
        self.print_to(&mut ::std::io::stdout());
    }

    /// Prints the layout tree to a particular output.
    pub fn print_to(&self, dest: &mut dyn (::std::io::Write)) {
        let mut printer = PrintTree::new("Layout tree", dest);
        self[self.root].print(self, self.root, &mut printer);
    }
}

/// A simple iterator for the in-flow ancestors of a layout node.
pub struct AncestorIterator<'tree> {
    tree: &'tree LayoutTree,
    current: Option<LayoutNodeId>,
}

impl<'tree> Iterator for AncestorIterator<'tree> {
    type Item = &'tree LayoutNode;

    fn next(&mut self) -> Option<Self::Item> {
        let next = &self.tree[self.current?];
        self.current = next.parent;
        Some(next)
    }
}

pub struct ContainingBlockIterator<'tree> {
    ancestors: AncestorIterator<'tree>,
    current: &'tree LayoutNode,
}

impl<'tree> Iterator for ContainingBlockIterator<'tree> {
    type Item = &'tree LayoutNode;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let next = self.ancestors.next()?;
            if !next.is_containing_block_for(self.current) {
                continue;
            }
            self.current = next;
            return Some(next);
        }
    }
}

impl ::std::ops::Index<LayoutNodeId> for LayoutTree {
    type Output = LayoutNode;

    fn index(&self, id: LayoutNodeId) -> &LayoutNode {
        &self.nodes[id.0]
    }
}

impl ::std::ops::IndexMut<LayoutNodeId> for LayoutTree {
    fn index_mut(&mut self, id: LayoutNodeId) -> &mut LayoutNode {
        &mut self.nodes[id.0]
    }
}
