pub mod builder;

use self::builder::InsertionPoint;
use allocator;
use app_units::Au;
use euclid::Size2D;
use logical_geometry;
use misc::print_tree::PrintTree;
use style::{self, ComputedStyle, Display};
use layout_algorithms::{LayoutContext, GenericLayoutResult, ConstraintSpace};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct LayoutNodeId(usize);

#[derive(Debug)]
pub enum LeafKind {
    Text { text: Box<str> },
    Replaced { intrinsic_size: Size2D<Au> },
}

#[derive(Debug)]
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
        match self.kind {
            LayoutNodeKind::Container { .. } => true,
            LayoutNodeKind::Leaf { .. } => false,
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

    fn print_label(&self) -> String {
        let mut label = match self.kind {
            LayoutNodeKind::Container { ref kind, .. } => format!("{:?}", kind),
            LayoutNodeKind::Leaf { ref kind } => format!("{:?}", kind),
        };

        if self.is_out_of_flow() {
            label.push_str(" (oof)");
        }

        if self.establishes_bfc() {
            label.push_str(" (bfc)");
        }

        label
    }

    fn print(&self, tree: &LayoutTree, printer: &mut PrintTree) {
        printer.new_level(self.print_label());
        for child in self.children(tree) {
            child.print(tree, printer);
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

    pub fn children<'tree>(
        &self,
        tree: &'tree LayoutTree,
    ) -> impl Iterator<Item = &'tree LayoutNode> {
        Children {
            current: self.first_child(),
            tree,
            get_next: |node| node.next_sibling(),
        }
    }

    pub fn rev_children<'tree>(
        &self,
        tree: &'tree LayoutTree,
    ) -> impl Iterator<Item = &'tree LayoutNode> {
        Children {
            current: self.first_child(),
            tree,
            get_next: |node| node.prev_sibling(),
        }
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
    type Item = &'a LayoutNode;

    fn next(&mut self) -> Option<Self::Item> {
        let current = self.current.take()?;
        let current = &self.tree[current];
        let next = (self.get_next)(current);
        self.current = next;
        Some(current)
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

    fn creates_ib_split(&self, node_style: &ComputedStyle, ip: &InsertionPoint) -> bool {
        // If the parent is not an inline, then it's definitely not an IB-split.
        if self[ip.parent].style.display != Display::Inline {
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

    #[allow(unused)]
    fn unchecked_move_all_children_to(
        &mut self,
        from_node: LayoutNodeId,
        to_node: LayoutNodeId,
        prev_sibling: Option<LayoutNodeId>,
    ) {
        let (first_child, _last_child) = match self[from_node].kind {
            LayoutNodeKind::Container { ref mut first_child, ref mut last_child, .. } => {
                (first_child.take(), last_child.take())
            }
            LayoutNodeKind::Leaf { .. }=> unreachable!(),
        };

        let mut current = first_child;
        while let Some(child) = current {
            // Un-parent the child, save next sibling so that we can
            // continue the loop.
            let child_prev_sibling = {
                let mut child = &mut self[child];
                assert_eq!(child.parent, Some(from_node));
                child.parent = None;

                current = child.next_sibling.take();
                let prev_sibling = child.prev_sibling.take();
                prev_sibling
            };

            let ip = InsertionPoint {
                parent: to_node,
                prev_sibling: child_prev_sibling.or(prev_sibling),
            };
            self.insert_unchecked(child, ip);
        }
    }

    #[allow(unused)]
    fn create_ib_split_anonymous_block(&mut self) -> LayoutNodeId {
        unimplemented!()
    }

    /// Ensures that a valid ib-split block wrapper is created right after the
    /// previous sibling (along with corresponding inline next-siblings if
    /// needed, so that there's always a trailing inline).
    ///
    /// Returns the id of the new anonymous block parent.
    fn ensure_ib_sibling_is_created_for(&mut self, _ip: &InsertionPoint) -> LayoutNodeId {
        // Whatever happens, we're going to need to create an anonymous block
        // for the new block.
        // let block = self.create_ib_split_anonymous_block();
        // let inline = self.create_inline_continuation(ip.parent);
        // self.unchecked_move_all_children_to(ip.parent, inline, XXX need a 'move children range');
        // let ip = InsertionPoint {
        //     parent: self[ip.parent].parent.expect("There should be no unparented inlines"),
        //     prev_sibling: ip.parent,
        // };
        // self.insert_unchecked(block, ip);
        // block
        unimplemented!();
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
        if !self.creates_ib_split(&for_node.style, &ip) {
            return ip;
        }
        let parent_block = self.ensure_ib_sibling_is_created_for(&ip);
        InsertionPoint {
            parent: parent_block,
            prev_sibling: None,
        }
    }

    fn insert_unchecked(&mut self, node_id: LayoutNodeId, ip: InsertionPoint) {
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
        self[self.root].print(self, &mut printer);
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
