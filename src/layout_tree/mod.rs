use allocator;
use style;
use logical_geometry;

#[derive(Debug, Copy, Clone)]
pub struct LayoutNodeId(usize);

pub enum LeafKind {
    Text { text: Box<str>, },
}

pub enum ContainerKind {
    /// The top-level viewport box.
    Viewport,
    Block,
    Inline,
}

pub enum LayoutNodeKind {
    Leaf { kind: LeafKind },
    Container {
        children: Vec<LayoutNodeId>,
        kind: ContainerKind,
    }
}

/// A display node is a node in the display tree, which contains the primary box
/// of each element.
///
/// A display node is the primary box of an element, but contains no layout
/// information, that's left to fragment.
pub struct LayoutNode {
    pub style: style::ComputedStyle,
    pub parent: Option<LayoutNodeId>,
    pub kind: LayoutNodeKind,
}

impl LayoutNode {
    pub fn display(&self) -> style::Display {
        self.style.display
    }

    pub fn position(&self) -> style::Position {
        self.style.position
    }

    pub fn writing_mode(&self) -> logical_geometry::WritingMode {
        self.style.writing_mode
    }

    pub fn children(&self) -> &[LayoutNodeId] {
        match self.kind {
            LayoutNodeKind::Container { ref children, .. } => &*children,
            LayoutNodeKind::Leaf { .. } => &[],
        }
    }

    pub fn in_flow_children<'a, 'dt: 'a>(
        &'a self,
        display_tree: &'dt DisplayTree,
    ) -> impl Iterator<Item = LayoutNodeId> + 'a {
        self.children()
            .iter()
            .cloned()
            .filter(move |el| !display_tree[*el].style.is_out_of_flow())
    }
}

pub struct DisplayTree {
    nodes: allocator::Allocator<LayoutNode>,
    root: LayoutNodeId,
}

impl DisplayTree {
    pub fn new() -> Self {
        let root = LayoutNode {
            style: style::ComputedStyle::for_viewport(),
            parent: None,
            kind: LayoutNodeKind::Container {
                children: Vec::new(),
                kind: ContainerKind::Viewport,
            }
        };

        let mut nodes = allocator::Allocator::default();
        let root = LayoutNodeId(nodes.allocate(root));

        Self {
            nodes,
            root,
        }
    }

    pub fn root(&self) -> LayoutNodeId {
        self.root
    }

    pub fn insert(&mut self, node: LayoutNode) -> LayoutNodeId {
        assert!(node.parent.is_some());
        LayoutNodeId(self.nodes.allocate(node))
    }

    pub fn destroy(&mut self, node: LayoutNodeId) {
        // TODO(emilio): This would have to clean up fragments and such from
        // other places.
        let node = self.nodes.deallocate(node.0);
        for child in node.children() {
            self.destroy(*child);
        }
    }
}

impl ::std::ops::Index<LayoutNodeId> for DisplayTree {
    type Output = LayoutNode;

    fn index(&self, id: LayoutNodeId) -> &LayoutNode {
        &self.nodes[id.0]
    }
}

impl ::std::ops::IndexMut<LayoutNodeId> for DisplayTree {
    fn index_mut(&mut self, id: LayoutNodeId) -> &mut LayoutNode {
        &mut self.nodes[id.0]
    }
}
