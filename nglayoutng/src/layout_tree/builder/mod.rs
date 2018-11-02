
use app_units::Au;
use css;
use dom;
use euclid::Size2D;
use super::{LayoutTree, LayoutNodeId, LayoutNode, LayoutNodeKind, ContainerKind, LeafKind};
use style::{self, ComputedStyle};

use kuchiki::{self, NodeRef, NodeData};

use std::collections::HashMap;
use std::io::{self, Read};

trait NodeMapHelpers<V> {
    fn for_node(&self, node: &kuchiki::Node) -> Option<&V>;
}

impl<V> NodeMapHelpers<V> for HashMap<*const kuchiki::Node, V> {
    #[inline(always)]
    fn for_node(&self, node: &kuchiki::Node) -> Option<&V> {
        self.get(&(node as *const kuchiki::Node))
    }
}

/// The map from DOM node to its principal box.
///
/// This is needed to handle additions and removals to the DOM tree, and in
/// a real browser should be kept somewhere holding off the node.
///
/// Note that for now layout nodes don't have a back-reference to the DOM...
/// We may need to add one, eventually, maybe?
pub type PrincipalBoxes = HashMap<*const kuchiki::Node, LayoutNodeId>;

/// The LayoutTreeBuilder gets a DOM and style tree and outputs a LayoutTree.
///
/// For now it doesn't care at all about any dynamic change or anything, and
/// assumes the HTML doesn't contain stuff that we don't support, like IB splits
/// or what not.
pub struct LayoutTreeBuilder {
    dom: NodeRef,
    styles: css::StyleMap,
    layout_tree: LayoutTree,
    principal_boxes: PrincipalBoxes,
}

#[derive(Debug)]
pub struct LayoutTreeBuilderResult {
    pub principal_boxes: PrincipalBoxes,
    pub layout_tree: LayoutTree,
}

pub struct InsertionPoint {
    pub parent: LayoutNodeId,
    pub prev_sibling: Option<LayoutNodeId>,
}

impl LayoutTreeBuilder {
    pub fn new(input: &mut impl Read) -> io::Result<Self> {
        use std::fs;
        use std::path::Path;

        let dom = dom::build_dom(input)?;
        let css = dom::read_stylesheets(&dom);

        let ua_sheet = fs::read_to_string(
            Path::new(env!("CARGO_MANIFEST_DIR")).join("src").join("css").join("res").join("ua.css")
        )?;

        let mut style_rules = css::parse_css(&ua_sheet);
        style_rules.extend(css::parse_css(&css));
        let styles = css::compute_styles(&dom, &style_rules);
        Ok(Self {
            dom,
            styles,
            layout_tree: LayoutTree::new(),
            principal_boxes: Default::default(),
        })
    }

    /// Builds the whole layout tree.
    ///
    /// Obviously this will need to become incremental and such.
    pub fn build(mut self) -> LayoutTreeBuilderResult {
        assert!(self.dom.as_document().is_some());
        for child in self.dom.children() {
            self.insert_node(&child);
        }

        LayoutTreeBuilderResult {
            layout_tree: self.layout_tree,
            principal_boxes: self.principal_boxes,
        }
    }

    fn dom_insertion_parent(&self, node: &NodeRef) -> Option<LayoutNodeId> {
        // Early out for nodes that never generate boxes.
        match node.data() {
            NodeData::Doctype(..) |
            NodeData::Document(..) |
            NodeData::DocumentFragment |
            NodeData::Comment(..) |
            NodeData::ProcessingInstruction(..) => return None,
            NodeData::Text(..) |
            NodeData::Element(..) => {},
        }

        let mut dom_parent = node.parent()?;
        loop {
            if dom_parent.as_document().is_some() {
                return Some(self.layout_tree.root())
            }
            let layout_parent = match self.principal_boxes.for_node(&dom_parent) {
                Some(node) => *node,
                None => {
                    if self.styles.for_node(&dom_parent)?.display != style::Display::Contents {
                        return None;
                    }
                    dom_parent = dom_parent.parent()?;
                    continue;
                }
            };
            return match self.layout_tree[layout_parent].kind {
                LayoutNodeKind::Container { .. } => Some(layout_parent),
                LayoutNodeKind::Leaf { .. } => None,
            };
        }
    }

    /// Finds a valid insertion prev-sibling starting the search with `node`,
    /// recursing down into display contents nodes but not recursing up.
    fn find_insertion_prev_sibling(&self, mut node: NodeRef) -> Option<LayoutNodeId> {
        loop {
            if let Some(node) = self.principal_boxes.for_node(&node) {
                return Some(*node)
            }
            if let Some(style) = self.styles.for_node(&node) {
                if style.display == style::Display::Contents {
                    if let Some(last) = node.last_child() {
                        // Drill down if possible.
                        if let Some(kid) = self.find_insertion_prev_sibling(last) {
                            return Some(kid);
                        }
                    }
                }
            }
            node = node.previous_sibling()?;
        }
    }

    fn dom_insertion_prev_sibling(&self, node: &NodeRef) -> Option<LayoutNodeId> {
        if let Some(previous_sibling) = node.previous_sibling() {
            if let Some(layout_sibling) = self.find_insertion_prev_sibling(previous_sibling) {
                return Some(layout_sibling);
            }
        }

        // We've exhausted our siblings, look at potential display: contents
        // parents.
        let mut parent = node.parent()?;
        loop {
            if self.styles.for_node(&parent)?.display != style::Display::Contents {
                return None;
            }
            if let Some(prev_sibling) = parent.previous_sibling() {
                if let Some(layout_sibling) = self.find_insertion_prev_sibling(prev_sibling) {
                    return Some(layout_sibling);
                }
            }
            parent = parent.parent()?;
        }
    }


    /// Returns the layout node that should be the parent of `node` (without
    /// accounting for any kind of anonymous boxes or anything of the sort).
    fn dom_insertion_point(&self, node: &NodeRef) -> Option<InsertionPoint> {
        let parent = self.dom_insertion_parent(node)?;
        let prev_sibling = self.dom_insertion_prev_sibling(node);
        Some(InsertionPoint { parent, prev_sibling })
    }

    fn insert_node_children(&mut self, parent: &NodeRef) {
        // TODO(emilio): Pseudo-elements, though not a big deal for the
        // prototype at least, I guess.
        for node in parent.children() {
            self.insert_node(&node);
        }
    }

    /// Tries to insert a node in the layout tree.
    fn insert_node(&mut self, node: &NodeRef) {
        let insertion_point = match self.dom_insertion_point(node) {
            Some(ip) => ip,
            None => return,
        };

        let style = if node.as_text().is_some() {
            let parent = node.parent().unwrap();
            self.styles.for_node(&parent)
        } else {
            self.styles.for_node(&node)
        }.expect("Node should be styled if we found an insertion point for it");

        let new_box = match self.construct_box_for(node, style, &insertion_point) {
            Some(node) => node,
            None => {
                if style.display == style::Display::Contents {
                    self.insert_node_children(node);
                }
                return;
            }
        };

        let id = self.layout_tree.insert(new_box, insertion_point);
        self.principal_boxes.insert(&**node, id);

        self.insert_node_children(node);
    }

    /// This constructs the box for an object, but doesn't insert it yet.
    fn construct_box_for(
        &self,
        node: &NodeRef,
        style: &ComputedStyle,
        _insertion_point: &InsertionPoint,
    ) -> Option<LayoutNode> {
        if style.display == style::Display::None ||
           style.display == style::Display::Contents
        {
            return None;
        }

        if let Some(text) = node.as_text() {
            return Some(LayoutNode::new_leaf(
                style.clone(),
                LeafKind::Text { text: text.borrow().clone().into_boxed_str() },
            ));
        }

        // TODO(emilio): This needs to handle a lot more cases: Form controls,
        // <fieldset>, <br> & such, <svg>, etc...
        if let Some(intrinsic_size) = Self::replaced_size(node) {
            return Some(LayoutNode::new_leaf(
                style.clone(),
                LeafKind::Replaced { intrinsic_size },
            ));
        }

        let container_kind = match style.display {
            style::Display::None |
            style::Display::Contents => unreachable!(),
            style::Display::Block => ContainerKind::Block,
            style::Display::Inline => ContainerKind::Inline,
        };
        Some(LayoutNode::new_container(style.clone(), container_kind))
    }

    fn replaced_size(node: &NodeRef) -> Option<Size2D<Au>> {
        use html5ever::LocalName;
        let element = node.as_element()?;
        if element.name.local != LocalName::from("img") {
            return None;
        }

        // NOTE(emilio): Pretty much intentionally oversimplified.
        let attrs = element.attributes.borrow();
        let width = attrs
            .get("width")
            .and_then(|w| w.parse::<i32>().ok())
            .map(|w| Au::from_f32_px(w as f32))
            .filter(|w| *w >= Au(0))
            .unwrap_or(Au::from_f32_px(150.0));
        let height = attrs
            .get("height")
            .and_then(|h| h.parse::<i32>().ok())
            .map(|h| Au::from_f32_px(h as f32))
            .filter(|h| *h >= Au(0))
            .unwrap_or(Au::from_f32_px(150.0));
        Some(Size2D::new(width, height))
    }
}
