pub mod builder;

use allocator;
use app_units::Au;
use euclid::Size2D;
use logical_geometry;
use self::builder::InsertionPoint;
use style::{self, ComputedStyle};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct LayoutNodeId(usize);

#[derive(Debug)]
pub enum LeafKind {
    Text { text: Box<str>, },
    Replaced { intrinsic_size: Size2D<Au>, }
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
    Leaf { kind: LeafKind },
    Container {
        first_child: Option<LayoutNodeId>,
        last_child: Option<LayoutNodeId>,
        // TODO(emilio): Put OOFs parented to me in here? Or collect them during
        // layout?
        kind: ContainerKind,
    }
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

    pub fn new_leaf(style: ComputedStyle, kind: LeafKind) -> Self {
        Self::new(style, LayoutNodeKind::Leaf { kind })
    }

    pub fn new_container(style: ComputedStyle, kind: ContainerKind) -> Self {
        Self::new(style, LayoutNodeKind::Container {
            first_child: None,
            last_child: None,
            kind,
        })
    }

    pub fn display(&self) -> style::Display {
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
}

#[derive(Debug)]
pub struct LayoutTree {
    nodes: allocator::Allocator<LayoutNode>,
    root: LayoutNodeId,
}

impl LayoutTree {
    pub fn new() -> Self {
        let root = LayoutNode::new_container(
            ComputedStyle::for_viewport(),
            ContainerKind::Viewport,
        );

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

    pub fn insert(
        &mut self,
        mut node: LayoutNode,
        ip: InsertionPoint,
    ) -> LayoutNodeId {
        assert!(node.parent.is_none());
        assert!(node.prev_sibling.is_none());
        assert!(node.next_sibling.is_none());
        match node.kind {
            LayoutNodeKind::Container { first_child, last_child, .. } => {
                assert!(first_child.is_none());
                assert!(last_child.is_none());
            }
            LayoutNodeKind::Leaf { .. } => {},
        }

        // TODO(emilio): Anon boxes will need to be handled here or somewhere
        // before here.
        if let Some(prev_sibling) = ip.prev_sibling {
            assert_eq!(self[prev_sibling].parent, Some(ip.parent));
        }

        node.parent = Some(ip.parent);
        node.prev_sibling = ip.prev_sibling;

        match ip.prev_sibling {
            Some(prev_sibling) => {
                node.next_sibling = self[prev_sibling].next_sibling;
            }
            None => {
                let parent = &mut self[ip.parent];
                node.next_sibling = parent.first_child();
            }
        }

        let new_next_sibling = node.next_sibling;

        let id = LayoutNodeId(self.nodes.allocate(node));

        if let Some(prev_sibling) = ip.prev_sibling {
            self[prev_sibling].next_sibling = Some(id);
        }

        if let Some(next_sibling) = new_next_sibling {
            self[next_sibling].prev_sibling = Some(id);
        }

        let parent = &mut self[ip.parent];
        match parent.kind {
            LayoutNodeKind::Container { ref mut first_child, ref mut last_child, .. } => {
                if ip.prev_sibling.is_none() {
                    *first_child = Some(id);
                }
                if *last_child == ip.prev_sibling {
                    *last_child = Some(id);
                }
            }
            LayoutNodeKind::Leaf { .. } => unreachable!(),
        }

        id
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
                LayoutNodeKind::Container { ref mut first_child, .. } => {
                    *first_child = removed_node.next_sibling;
                }
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
                LayoutNodeKind::Container { ref mut last_child, .. } => {
                    *last_child = removed_node.prev_sibling;
                }
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
