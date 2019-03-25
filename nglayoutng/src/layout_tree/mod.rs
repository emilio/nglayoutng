pub mod builder;

use self::builder::InsertionPoint;
use allocator;
use app_units::Au;
use euclid::Size2D;
use layout_algorithms::{ConstraintSpace, GenericLayoutResult, LayoutContext};
use logical_geometry;
use misc::print_tree::PrintTree;
use style::{self, ComputedStyle, Display};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct LayoutNodeId(usize);

#[derive(Debug)]
pub enum LeafKind {
    Text { text: Box<str> },
    Replaced { intrinsic_size: Size2D<Au> },
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum ContainerKind {
    /// The top-level viewport box.
    Viewport,
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

    pub fn is_container(&self) -> bool {
        self.container_kind().is_some()
    }

    fn is_inline(&self) -> bool {
        self.container_kind()
            .map_or(false, |k| k == ContainerKind::Inline)
    }

    fn is_ib_split_wrapper(&self) -> bool {
        let is_split = self.style.is_ib_split_wrapper();
        if is_split {
            assert_eq!(self.container_kind(), Some(ContainerKind::Block));
        }
        is_split
    }

    fn container_kind(&self) -> Option<ContainerKind> {
        match self.kind {
            LayoutNodeKind::Container { kind, .. } => Some(kind),
            LayoutNodeKind::Leaf { .. } => None,
        }
    }

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
        use style::Overflow;

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
        if display.is_block_outside() && self.style.overflow_x != Overflow::Visible {
            return true;
        }

        // TODO(emilio): Columns and such, step by step...
        false
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

        if self.is_ib_split_wrapper() {
            label.push_str(" (ib-split-wrapper)");
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

    pub fn next_sibling(&self) -> Option<LayoutNodeId> {
        self.next_sibling
    }

    pub fn prev_sibling(&self) -> Option<LayoutNodeId> {
        self.prev_sibling
    }

    pub fn parent(&self) -> Option<LayoutNodeId> {
        self.parent
    }

    fn children_and_id<'tree>(
        &self,
        tree: &'tree LayoutTree,
    ) -> impl Iterator<Item = (LayoutNodeId, &'tree LayoutNode)> {
        Children {
            current: self.first_child(),
            tree,
            get_next: |node| node.next_sibling(),
        }
    }

    fn rev_children_and_id<'tree>(
        &self,
        tree: &'tree LayoutTree,
    ) -> impl Iterator<Item = (LayoutNodeId, &'tree LayoutNode)> {
        Children {
            current: self.last_child(),
            tree,
            get_next: |node| node.prev_sibling(),
        }
    }

    pub fn children<'tree>(
        &self,
        tree: &'tree LayoutTree,
    ) -> impl Iterator<Item = &'tree LayoutNode> {
        self.children_and_id(tree).map(|(_id, child)| child)
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
        let root =
            LayoutNode::new_container(ComputedStyle::for_viewport(), ContainerKind::Viewport);

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
    }

    pub fn root(&self) -> LayoutNodeId {
        self.root
    }

    pub fn root_node(&self) -> &LayoutNode {
        &self[self.root]
    }

    pub fn insert(&mut self, node: LayoutNode, mut ip: InsertionPoint) -> LayoutNodeId {
        // TODO(emilio): Also need to handle table anonymous wrappers and
        // company.
        ip = self.create_split_if_needed(&node, ip);
        let id = LayoutNodeId(self.nodes.allocate(node));
        self.insert_unchecked(id, ip);
        id
    }

    fn creates_ib_split(&self, node_style: &ComputedStyle, container: LayoutNodeId) -> bool {
        // If the parent is not an inline, then it's definitely not an IB-split.
        if !self[container].is_inline() {
            return false;
        }
        if !node_style.display.is_block_outside() {
            return false;
        }
        if node_style.is_out_of_flow() {
            return false;
        }
        return true;
    }

    fn unchecked_move_children_to(
        &mut self,
        from_node: LayoutNodeId,
        to_node: LayoutNodeId,
        from_sibling: Option<LayoutNodeId>,
    ) {
        trace!(
            "unchecked_move_children_to({:?}, {:?}, {:?})",
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

    fn create_ib_split_anonymous_block(&mut self) -> LayoutNodeId {
        LayoutNodeId(self.nodes.allocate(LayoutNode::new_container(
            ComputedStyle::for_ib_split_wrapper(),
            ContainerKind::Block,
        )))
    }

    fn create_inline_continuation(&mut self, inline: LayoutNodeId) -> LayoutNodeId {
        assert_eq!(self[inline].style.display, Display::Inline);
        assert_eq!(self[inline].container_kind(), Some(ContainerKind::Inline));
        let style = self[inline].style.clone();
        let node = LayoutNode::new_container(style, ContainerKind::Inline);
        LayoutNodeId(self.nodes.allocate(node))
    }

    /// Ensures that a valid ib-split block wrapper is created right after the
    /// previous sibling (along with corresponding inline next-siblings if
    /// needed, so that there's always a trailing inline).
    ///
    /// Returns the id of the new anonymous block parent.
    fn ensure_ib_sibling_is_created_for(&mut self, ip: &InsertionPoint) -> LayoutNodeId {
        trace!("ensure_ib_sibling_is_created_for({:?})", ip);
        // Whatever happens, we're going to need to create an anonymous block
        // for the new block, and at least a new inline.
        let block = self.create_ib_split_anonymous_block();
        let inline = self.create_inline_continuation(ip.parent);
        let grandparent = self[ip.parent]
            .parent
            .expect("There should be no un-parented inlines");

        {
            let ip = InsertionPoint {
                parent: grandparent,
                prev_sibling: Some(ip.parent),
            };

            self.insert_unchecked(block, ip);

            let ip = InsertionPoint {
                parent: grandparent,
                prev_sibling: Some(block),
            };
            self.insert_unchecked(inline, ip);
        }

        // Move all children after the prev sibiling to the inline-continuation.
        self.unchecked_move_children_to(ip.parent, inline, ip.prev_sibling);

        // If the prev sibling is an block wrapper, move it to our block as
        // well.
        if let Some(prev) = ip.prev_sibling {
            if self[prev].is_ib_split_wrapper() {
                let prev_sibling = self[prev].prev_sibling;
                self.unchecked_move_children_to(ip.parent, block, prev_sibling);
            }
        }

        block
    }

    fn next_ib_wrapper_for_inline(&self, id: LayoutNodeId) -> Option<LayoutNodeId> {
        let mut depth = 0;
        let mut current_inline = Some(id);
        while let Some(inline) = current_inline {
            assert!(self[inline].is_inline());
            if let Some(mut sibling) = self[inline].next_sibling {
                if !self[sibling].is_ib_split_wrapper() {
                    return None;
                }
                for _ in 0..depth {
                    sibling = self[sibling].first_child().unwrap();
                }
                assert!(self[sibling].is_ib_split_wrapper());
                return Some(sibling);
            }
            let parent = self[inline].parent?;
            if !self[parent].is_inline() {
                return None;
            }
            depth += 1;
            current_inline = Some(parent);
        }
        unreachable!("How did we exit the loop?");
    }

    fn next_inline_for_ib_wrapper(&self, id: LayoutNodeId) -> LayoutNodeId {
        assert!(self[id].is_ib_split_wrapper());
        let mut current_ib_sibling = Some(id);
        let mut depth = 0;
        while let Some(ib_wrapper) = current_ib_sibling {
            assert!(self[ib_wrapper].is_ib_split_wrapper());
            if let Some(mut next_inline) = self[ib_wrapper].next_sibling {
                // Go back to our level.
                for _ in 0..depth {
                    next_inline = self[next_inline].first_child().unwrap();
                }
                return next_inline;
            }
            current_ib_sibling = self[ib_wrapper].parent;
            depth += 1;
        }
        unreachable!("Should always have a trailing inline");
    }

    fn last_inline_continuation(&self, id: LayoutNodeId) -> LayoutNodeId {
        if !self[id].is_inline() {
            return id;
        }

        let mut current_inline = Some(id);
        while let Some(inline_id) = current_inline {
            let inline = &self[inline_id];
            assert!(inline.is_inline());

            let wrapper = match self.next_ib_wrapper_for_inline(inline_id) {
                Some(next_sibling) => next_sibling,
                None => return inline_id,
            };

            current_inline = Some(self.next_inline_for_ib_wrapper(wrapper));
        }

        unreachable!("should always have a trailing inline");
    }

    fn adjusted_insertion_point_for_ib_split(&self, ip: InsertionPoint) -> InsertionPoint {
        let parent = &self[ip.parent];
        if !parent.is_inline() {
            return InsertionPoint {
                parent: ip.parent,
                prev_sibling: ip.prev_sibling.map(|p| self.last_inline_continuation(p)),
            };
        }

        let prev_sibling_id = match ip.prev_sibling {
            Some(prev_sibling) => prev_sibling,
            None => return ip,
        };
        let prev_sibling = &self[prev_sibling_id];
        assert!(
            !prev_sibling.is_ib_split_wrapper(),
            "Shouldn't get here with an ib-split wrapper"
        );
        if self.creates_ib_split(&prev_sibling.style, ip.parent) {
            assert!(
                prev_sibling
                    .next_sibling
                    .map_or(true, |next_sibling| !self[next_sibling]
                        .is_ib_split_wrapper()),
                "Non-inlines shouldn't have ib-wrapping siblings"
            );

            // If we're inserting after a block wrapped in an ib split wrapper,
            // then this is not a block itself (otherwise we'd create a wrapper
            // for this node). In this case, where we really want to insert
            // ourselves is in the following inline continuation.
            let parent = prev_sibling.parent.unwrap();
            return InsertionPoint {
                parent: self.next_inline_for_ib_wrapper(parent),
                prev_sibling: None,
            };
        }

        // We're inserting inside an inline, and next to something that isn't a
        // block. We want to insert using the right IB-split continuation, which
        // is the one the inline is in.
        let parent = self[prev_sibling_id].parent.unwrap();
        InsertionPoint {
            parent,
            prev_sibling: ip.prev_sibling,
        }
    }

    /// Handles IB-split, table-anon-boxes creation, and such.
    ///
    /// TODO(emilio): We need to dynamically remove splits as well when style
    /// changes.
    fn create_split_if_needed(
        &mut self,
        for_node: &LayoutNode,
        ip: InsertionPoint,
    ) -> InsertionPoint {
        trace!("create_split_if_needed({:?})", ip);
        if !self.creates_ib_split(&for_node.style, ip.parent) {
            let new_ip = self.adjusted_insertion_point_for_ib_split(ip);
            trace!(
                "adjusted_insertion_point_for_ib_split: {:?} -> {:?}",
                ip,
                new_ip
            );
            return new_ip;
        }

        let parent_block = self.ensure_ib_sibling_is_created_for(&ip);
        // We need to also split arbitrary inline ancestors.
        {
            let mut current_parent = self[ip.parent].parent;
            let mut current_block = parent_block;
            while let Some(parent) = current_parent {
                if !self[parent].is_inline() {
                    break;
                }

                let ip = InsertionPoint {
                    parent,
                    prev_sibling: Some(current_block),
                };

                current_block = self.ensure_ib_sibling_is_created_for(&ip);
                current_parent = self[parent].parent;
            }
        }
        InsertionPoint {
            parent: parent_block,
            prev_sibling: None,
        }
    }

    fn insert_unchecked(&mut self, node_id: LayoutNodeId, ip: InsertionPoint) {
        trace!("Inserting {:?} into {:?}", node_id, ip);
        self.assert_subtree_consistent(ip.parent);

        {
            let node = &self[node_id];
            assert!(node.parent.is_none());
            assert!(node.prev_sibling.is_none());
            assert!(node.next_sibling.is_none());
            match node.kind {
                LayoutNodeKind::Container {
                    first_child,
                    last_child,
                    ..
                } => {
                    assert!(first_child.is_none());
                    assert!(last_child.is_none());
                },
                LayoutNodeKind::Leaf { .. } => {},
            }

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
            child = self[child_to_remove].next_sibling();
            self.destroy(child_to_remove);
        }
    }

    /// Prints the layout tree to stdout.
    pub fn print(&self) {
        self.print_to(&mut ::std::io::stdout());
    }

    /// Prints the layout tree to a particular output.
    pub fn print_to(&self, dest: &mut ::std::io::Write) {
        let mut printer = PrintTree::new("Layout tree", dest);
        self[self.root].print(self, self.root, &mut printer);
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
