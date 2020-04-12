pub mod builder;

use self::builder::InsertionPoint;
use crate::allocator;
use crate::fragment_tree::ChildFragment;
use crate::layout_tree::builder::{inline::InlineInside, block::BlockInside};
use crate::layout_algorithms::{AvailableSize, ConstraintSpace, LayoutAlgorithm, LayoutContext};
use crate::layout_algorithms::block::BlockFormattingContext;
use crate::logical_geometry::{LogicalSize, WritingMode};
use crate::misc::print_tree::PrintTree;
use crate::style::{self, ComputedStyle, Display, DisplayInside, PseudoElement};
use app_units::Au;
use euclid::default::Size2D;
use html5ever::tree_builder::QuirksMode;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct LayoutNodeId(usize);

#[derive(Debug)]
pub enum LeafKind {
    Text { text: Box<str> },
    Replaced { intrinsic_size: Size2D<Au> },
}

#[derive(Clone, PartialEq, Eq)]
pub enum ContainerKind {
    Block {
        prev_ib_sibling: Option<LayoutNodeId>,
        next_ib_sibling: Option<LayoutNodeId>,
    },
    Inline {
        prev_ib_sibling: Option<LayoutNodeId>,
        next_ib_sibling: Option<LayoutNodeId>,
    },
}

impl std::fmt::Debug for ContainerKind {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match *self {
            ContainerKind::Block { ref prev_ib_sibling, ref next_ib_sibling } => {
                if prev_ib_sibling.is_none() && next_ib_sibling.is_none() {
                    f.write_str("Block")
                } else {
                    f.debug_struct("Block")
                        .field("prev_ib_sibling", prev_ib_sibling)
                        .field("next_ib_sibling", next_ib_sibling)
                        .finish()
                }
            }
            ContainerKind::Inline { ref prev_ib_sibling, ref next_ib_sibling } => {
                if prev_ib_sibling.is_none() && next_ib_sibling.is_none() {
                    f.write_str("Inline")
                } else {
                    f.debug_struct("Inline")
                        .field("prev_ib_sibling", prev_ib_sibling)
                        .field("next_ib_sibling", next_ib_sibling)
                        .finish()
                }
            }
        }
    }
}

impl ContainerKind {
    pub fn inline() -> Self {
        Self::Inline {
            prev_ib_sibling: None,
            next_ib_sibling: None,
        }
    }

    pub fn block() -> Self {
        Self::Block {
            prev_ib_sibling: None,
            next_ib_sibling: None,
        }
    }

    pub fn is_block(&self) -> bool {
        matches!(*self, Self::Block { .. })
    }

    pub fn is_inline(&self) -> bool {
        matches!(*self, Self::Inline { .. })
    }
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

#[derive(Copy, Clone, PartialEq)]
pub enum PrintId {
    No,
    Yes,
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

    fn prev_ib_sibling(&self) -> Option<LayoutNodeId> {
        match self.kind {
            LayoutNodeKind::Container { ref kind, ..  } => {
                match *kind {
                    ContainerKind::Block { prev_ib_sibling, .. } |
                    ContainerKind::Inline { prev_ib_sibling, .. } => prev_ib_sibling,
                }
            }
            LayoutNodeKind::Leaf { .. } => None,
        }
    }

    fn next_ib_sibling(&self) -> Option<LayoutNodeId> {
        match self.kind {
            LayoutNodeKind::Container { ref kind, ..  } => {
                match *kind {
                    ContainerKind::Block { next_ib_sibling, .. } |
                    ContainerKind::Inline { next_ib_sibling, .. } => next_ib_sibling,
                }
            }
            LayoutNodeKind::Leaf { .. } => None,
        }
    }

    fn is_anonymous(&self) -> bool {
        self.style.pseudo.map_or(false, |p| p.is_anonymous())
    }

    pub fn is_container(&self) -> bool {
        self.container_kind().is_some()
    }

    pub fn is_block_container(&self) -> bool {
        self.container_kind().map_or(false, |k| k.is_block())
    }

    pub fn is_inline(&self) -> bool {
        self.container_kind().map_or(false, |k| k.is_inline())
    }

    pub fn is_inline_continuation(&self, tree: &LayoutTree) -> bool {
        self.is_inline() && self.prev_sibling.map_or(false, |sibling| tree[sibling].style.pseudo == Some(PseudoElement::BlockInsideInlineWrapper))
    }

    fn container_kind(&self) -> Option<ContainerKind> {
        match self.kind {
            LayoutNodeKind::Container { ref kind, .. } => Some(kind.clone()),
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
        if display.inside() == DisplayInside::FlowRoot {
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

    /// Returns whether a node establishes an inline formatting context.
    ///
    /// This is, effectively, whether we're a block-of-inlines, and thus whether
    /// any in-flow child is inline.
    pub fn establishes_ifc(&self, tree: &LayoutTree) -> bool {
        if !self.is_block_container() {
            return false;
        }
        match self.in_flow_children(tree).next() {
            Some(c) => c.style.display.is_inline_outside(),
            None => false,
        }
    }

    fn ancestors<'tree>(&self, tree: &'tree LayoutTree) -> AncestorIterator<'tree> {
        AncestorIterator {
            tree,
            current: self.parent,
        }
    }

    fn is_absolute_containing_block(&self) -> bool {
        if self.is_fixed_containing_block() {
            return true;
        }

        self.position() != style::Position::Static
    }

    fn is_fixed_containing_block(&self) -> bool {
        if self.parent.is_none() {
            return true;
        }
        // TODO(emilio): transform / will-change: transform /  filters, etc.
        false
    }

    fn is_containing_block_for(&self, child: &Self) -> bool {
        // ICB contains everything.
        if self.parent.is_none() {
            return true;
        }

        if self.is_fixed_containing_block() {
            debug_assert!(self.is_absolute_containing_block());
            return true;
        }

        if child.position() == style::Position::Fixed {
            return false;
        }

        if child.is_out_of_flow_positioned() {
            return self.is_absolute_containing_block();
        }

        // TODO(emilio): Gecko avoids returning true for ib-split wrappers,
        // table rows and other junk.
        self.is_block_container()
    }

    pub fn expect_containing_block<'tree>(&'tree self, tree: &'tree LayoutTree) -> &'tree Self {
        self.containing_block(tree).expect("Called expect_containing_block on the root?")
    }

    pub fn containing_block<'tree>(&'tree self, tree: &'tree LayoutTree) -> Option<&'tree Self> {
        self.containing_block_chain(tree).next()
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

    fn print_label(&self, id: LayoutNodeId, print_id: PrintId) -> String {
        let mut label = match self.kind {
            LayoutNodeKind::Container { ref kind, .. } => format!("{:?}", kind),
            LayoutNodeKind::Leaf { ref kind } => format!("{:?}", kind),
        };

        if print_id == PrintId::Yes {
            label.push_str(&format!(" - {:?}", id));
        }

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

    fn print(&self, tree: &LayoutTree, id: LayoutNodeId, printer: &mut PrintTree, print_id: PrintId) {
        printer.new_level(self.print_label(id, print_id));
        for (id, child) in self.children_and_id(tree) {
            child.print(tree, id, printer, print_id);
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

    pub fn writing_mode(&self) -> WritingMode {
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

    pub fn prev_sibling<'tree>(&self, tree: &'tree LayoutTree) -> Option<&'tree LayoutNode> {
        Some(&tree[self.prev_sibling?])
    }

    pub fn children_and_id<'tree>(
        &self,
        tree: &'tree LayoutTree,
    ) -> impl Iterator<Item = (LayoutNodeId, &'tree LayoutNode)> {
        Children {
            current: self.first_child(),
            tree,
            get_next: |node| node.next_sibling,
        }
    }

    pub fn rev_children_and_id<'tree>(
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

    pub fn prev_siblings_and_id<'tree>(
        &self,
        tree: &'tree LayoutTree,
    ) -> impl Iterator<Item = (LayoutNodeId, &'tree LayoutNode)> {
        Children {
            current: self.prev_sibling,
            tree,
            get_next: |node| node.prev_sibling,
        }
    }

    pub fn following_siblings_and_id<'tree>(
        &self,
        tree: &'tree LayoutTree,
    ) -> impl Iterator<Item = (LayoutNodeId, &'tree LayoutNode)> {
        Children {
            current: self.next_sibling,
            tree,
            get_next: |node| node.next_sibling,
        }
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
        let root = LayoutNode::new_container(ComputedStyle::for_viewport(), ContainerKind::block());

        let mut nodes = allocator::Allocator::default();
        let root = LayoutNodeId(nodes.allocate(root));

        Self { nodes, root }
    }

    fn non_anonymous_ancestor(&self, mut id: LayoutNodeId) -> Option<LayoutNodeId> {
        loop {
            id = self[id].parent?;
            if !self[id].is_anonymous() {
                return Some(id);
            }
        }
    }

    fn register_ib_split(&mut self, prev: LayoutNodeId, next: LayoutNodeId) {
        assert!(self[prev].is_block_container() || self[prev].is_inline());
        assert!(self[next].is_block_container() || self[next].is_inline());
        assert!(self[prev].is_block_container() != self[next].is_block_container());

        match self[prev].kind {
            LayoutNodeKind::Container { ref mut kind, ..  } => {
                match *kind {
                    ContainerKind::Block { ref mut next_ib_sibling, ..  } |
                    ContainerKind::Inline { ref mut next_ib_sibling, ..  } => {
                        *next_ib_sibling = Some(next);
                    }
                }
            }
            _ => unreachable!(),
        }

        match self[next].kind {
            LayoutNodeKind::Container { ref mut kind, ..  } => {
                match *kind {
                    ContainerKind::Block { ref mut prev_ib_sibling, ..  } |
                    ContainerKind::Inline { ref mut prev_ib_sibling, ..  } => {
                        *prev_ib_sibling = Some(prev);
                    }
                }
            }
            _ => unreachable!(),
        }
    }

    pub fn assert_consistent(&self) {
        self.assert_subtree_consistent(self.root);
    }

    fn last_inline_continuation(&self, inline: LayoutNodeId) -> LayoutNodeId {
        // IB splits have a structure like:
        // Containing block
        //   InlineInsideBlockWrapper
        //     Inline
        //       some content...
        //   BlockInsideInlineWrapper
        //     Block content...
        //   InlineInsideBlockWrapper
        //     Inline
        //       some more...
        assert!(self[inline].is_inline());
        let mut current = inline;
        while let Some(next) = self[current].next_ib_sibling() {
            current = next;
        }
        current
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
            Some(ContainerKind::Block { .. }) => {
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
            Some(ContainerKind::Inline { .. }) => {
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

    pub fn insert(&mut self, node: LayoutNode, mut ip: InsertionPoint) -> Option<LayoutNodeId> {
        if let Some(ref mut prev_sibling) = ip.prev_sibling {
            if self[*prev_sibling].is_inline() {
                *prev_sibling = self.last_inline_continuation(*prev_sibling);
            }
        }

        let container_kind = self[ip.parent].container_kind()?;
        let ip = match container_kind {
            ContainerKind::Inline { .. } => InlineInside::insertion(self, &node, ip)?,
            ContainerKind::Block { .. } => BlockInside::insertion(self, &node, ip)?,
        };
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

    /// Detaches a node from the tree, without detaching its children or
    /// deallocating the node.
    ///
    /// NOTE: This leaks if the node is not re-inserted or de-allocated, and the
    /// node doesn't lose its identity.
    ///
    /// Returns the right insertion point position for the removed node, in
    /// order to be in the same position if re-inserted there via insert().
    pub fn detach(&mut self, node_to_remove: LayoutNodeId) -> InsertionPoint {
        let parent = self[node_to_remove].parent.expect("Detaching the root not supported");
        if self[parent].is_anonymous() {
            let pseudo = self[parent].style.pseudo.unwrap();
            return match pseudo {
                PseudoElement::Viewport => self.detach_unchecked(node_to_remove).unwrap(),
                PseudoElement::BlockInsideInlineWrapper => {
                    InlineInside::detach_from_ib_split_block_wrapper(self, parent, node_to_remove)
                }
                PseudoElement::InlineContinuation => {
                    // InlineInside::remove_from_inline_continuation(self, parent, node_to_remove);
                    unimplemented!()
                },
                PseudoElement::InlineInsideBlockWrapper => {
                    // BlockInside::detach_from_inline_wrapper(self, parent, node_to_remove)
                    unimplemented!()
                }
                PseudoElement::Before | PseudoElement::After => {
                    unreachable!("These are not anonymous boxes")
                }
            };
        }

        match self[parent].container_kind().unwrap() {
            ContainerKind::Block { .. } => BlockInside::detach(self, parent, node_to_remove),
            ContainerKind::Inline { .. } => InlineInside::detach(self, parent, node_to_remove),
        }
    }

    pub fn detach_unchecked(&mut self, node_to_remove: LayoutNodeId) -> Option<InsertionPoint> {
        let (prev_sibling, parent, next_sibling) = {
            let node = &mut self[node_to_remove];
            (node.prev_sibling.take(), node.parent.take(), node.next_sibling.take())
        };

        // Fix up the tree.
        if let Some(prev_sibling) = prev_sibling {
            let prev_sibling = &mut self[prev_sibling];
            assert_eq!(prev_sibling.next_sibling, Some(node_to_remove));
            prev_sibling.next_sibling = next_sibling;
        } else if let Some(parent) = parent {
            let parent = &mut self[parent];
            assert_eq!(parent.first_child(), Some(node_to_remove));
            match parent.kind {
                LayoutNodeKind::Container {
                    ref mut first_child,
                    ..
                } => {
                    *first_child = next_sibling;
                },
                LayoutNodeKind::Leaf { .. } => unreachable!(),
            }
        }

        if let Some(next_sibling) = next_sibling {
            let next_sibling = &mut self[next_sibling];
            assert_eq!(next_sibling.prev_sibling, Some(node_to_remove));
            next_sibling.prev_sibling = prev_sibling;
        } else if let Some(parent) = parent {
            let parent = &mut self[parent];
            assert_eq!(parent.last_child(), Some(node_to_remove));
            match parent.kind {
                LayoutNodeKind::Container {
                    ref mut last_child, ..
                } => {
                    *last_child = prev_sibling;
                },
                LayoutNodeKind::Leaf { .. } => unreachable!(),
            }
        }

        parent.map(|parent| {
            InsertionPoint {
                parent,
                prev_sibling,
            }
        })
    }

    pub fn destroy(&mut self, node_to_remove: LayoutNodeId) {
        // Recursively tear down the children.
        let mut child = self[node_to_remove].first_child();
        while let Some(child_to_remove) = child.take() {
            child = self[child_to_remove].next_sibling;
            self.destroy(child_to_remove);
        }

        // Detach the node from the tree.
        self.detach_unchecked(node_to_remove);

        // And de-allocate the node.
        let removed_node = self.nodes.deallocate(node_to_remove.0);
        assert_eq!(removed_node.next_sibling, None);
        assert_eq!(removed_node.prev_sibling, None);
        assert_eq!(removed_node.parent, None);
        assert_eq!(removed_node.first_child(), None);
        assert_eq!(removed_node.last_child(), None);
    }

    /// Prints the layout tree to stdout with ids.
    pub fn print_with_ids(&self) {
        self.print_to(&mut std::io::stdout(), PrintId::Yes);
    }

    /// Prints the layout tree to stdout.
    pub fn print(&self) {
        self.print_to(&mut std::io::stdout(), PrintId::No);
    }

    /// Prints the layout tree to a particular output.
    pub fn print_to(&self, dest: &mut dyn std::io::Write, print_id: PrintId) {
        let mut printer = PrintTree::new("Layout tree", dest);
        self[self.root].print(self, self.root, &mut printer, print_id);
    }

    /// Actually runs layout on the tree!
    pub fn layout(&self, quirks_mode: QuirksMode, viewport_size: Size2D<Au>) -> ChildFragment {
        let context = LayoutContext {
            quirks_mode,
            layout_tree: self,
        };

        let root = self.root_node();
        let wm = root.writing_mode();
        let available_inline_size = if wm.is_vertical() {
            viewport_size.height
        } else {
            viewport_size.width
        };

        let percentage_resolution_size = LogicalSize::from_physical(wm, viewport_size);
        let constraints = ConstraintSpace {
            available_size: AvailableSize::unconstrained_block(wm, available_inline_size),
            percentage_resolution_size: AvailableSize::definite(wm, percentage_resolution_size),
            containing_block_writing_mode: wm,
        };

        let result = BlockFormattingContext::new(&context, root).layout(&constraints);
        // assert!(result.break_token.is_none(), "How did we fragment with unconstrained block size?");
        result.root_fragment
    }
}

impl Drop for LayoutTree {
    fn drop(&mut self) {
        self.destroy(self.root);
        assert!(self.nodes.is_empty(), "Leaked detached nodes");
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
