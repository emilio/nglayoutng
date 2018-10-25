use allocator;
use style;
use logical_geometry;

#[derive(Debug, Copy, Clone)]
pub struct DisplayNodeId(usize);

pub enum LeafKind {
    Text { text: String, },
}

pub enum ContainerKind {
    /// The top-level viewport box.
    Viewport,
    Block,
    Inline,
}

pub enum DisplayNodeKind {
    Leaf { kind: LeafKind },
    Container {
        children: Vec<DisplayNodeId>,
        kind: ContainerKind,
    }
}

/// A display node is a node in the display tree, which contains the primary box
/// of each element.
///
/// A display node is the primary box of an element, but contains no layout
/// information, that's left to fragment.
pub struct DisplayNode {
    pub style: style::ComputedStyle,
    pub parent: Option<DisplayNodeId>,
    pub kind: DisplayNodeKind,
}

impl DisplayNode {
    pub fn display(&self) -> style::Display {
        self.style.display
    }

    pub fn position(&self) -> style::Position {
        self.style.position
    }

    pub fn writing_mode(&self) -> logical_geometry::WritingMode {
        self.style.writing_mode
    }

    pub fn children(&self) -> &[DisplayNodeId] {
        match self.kind {
            DisplayNodeKind::Container { ref children, .. } => &*children,
            DisplayNodeKind::Leaf { .. } => &[],
        }
    }

    pub fn in_flow_children<'a, 'dt: 'a>(
        &'a self,
        display_tree: &'dt DisplayTree,
    ) -> impl Iterator<Item = DisplayNodeId> + 'a {
        self.children()
            .iter()
            .cloned()
            .filter(move |el| display_tree[*el].style.is_out_of_flow())
    }
}

pub struct DisplayTree {
    nodes: allocator::Allocator<DisplayNode>,
    root: DisplayNodeId,
}

impl DisplayTree {
    pub fn new() -> Self {
        let root = DisplayNode {
            style: style::ComputedStyle::for_viewport(),
            parent: None,
            kind: DisplayNodeKind::Container {
                children: Vec::new(),
                kind: ContainerKind::Viewport,
            }
        };

        let mut nodes = allocator::Allocator::default();
        let root = DisplayNodeId(nodes.allocate(root));

        Self {
            nodes,
            root,
        }
    }

    pub fn root(&self) -> DisplayNodeId {
        self.root
    }

    pub fn insert(&mut self, node: DisplayNode) -> DisplayNodeId {
        assert!(node.parent.is_some());
        DisplayNodeId(self.nodes.allocate(node))
    }

    pub fn destroy(&mut self, node: DisplayNodeId) {
        // TODO(emilio): This would have to clean up fragments and such from
        // other places.
        let node = self.nodes.deallocate(node.0);
        for child in node.children() {
            self.destroy(*child);
        }
    }
}

impl ::std::ops::Index<DisplayNodeId> for DisplayTree {
    type Output = DisplayNode;

    fn index(&self, id: DisplayNodeId) -> &DisplayNode {
        &self.nodes[id.0]
    }
}

impl ::std::ops::IndexMut<DisplayNodeId> for DisplayTree {
    fn index_mut(&mut self, id: DisplayNodeId) -> &mut DisplayNode {
        &mut self.nodes[id.0]
    }
}
