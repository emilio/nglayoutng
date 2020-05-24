use app_units::Au;
use crate::fragment_tree::{ChildFragment, Fragment, FragmentKind, ContainerFragmentKind};
use super::{ConstraintSpace, LayoutContext, LayoutResult};
use crate::layout_tree::{LayoutNodeKind, LeafKind, ContainerKind, LayoutNode, LayoutNodeId};

pub struct InlineFormattingContext<'a, 'b> {
    context: &'a LayoutContext<'b>,
    input_node: &'a LayoutNode,
}

enum InlineItem {
    // The start of a nested inline box.
    TagStart(LayoutNodeId),
    // TODO(emilio): Probably want to reference-count this somehow, or something
    // something.
    Text(Box<str>),
    Replaced(LayoutNodeId),
    AtomicInline(LayoutNodeId),
    TagEnd(LayoutNodeId),
}

impl<'a, 'b> InlineFormattingContext<'a, 'b> {
    pub fn new(context: &'a LayoutContext<'b>, input_node: &'a LayoutNode) -> Self {
        debug_assert!(input_node.establishes_ifc(context.layout_tree));
        Self {
            context,
            input_node,
        }
    }

    fn collect_inline_items_in(&self, node: &LayoutNode) -> Vec<InlineItem> {
        let mut items = vec![];

        for (id, child) in node.children_and_id(self.context.layout_tree) {
            match child.kind {
                LayoutNodeKind::Leaf { ref kind } => {
                    match kind {
                        LeafKind::Replaced { .. } => items.push(InlineItem::Replaced(id)),
                        LeafKind::Text { ref text } => items.push(InlineItem::Text(text.clone())),
                    }
                }
                LayoutNodeKind::Container { ref kind, .. } => {
                    match kind {
                        ContainerKind::Inline { .. } => {
                            items.push(InlineItem::TagStart(id));
                            self.collect_inline_items_in(child);
                            items.push(InlineItem::TagEnd(id));
                        },
                        ContainerKind::Block { .. } => {
                            debug_assert!(
                                !child.style.display.is_block_outside(),
                                "Should've been split",
                            );
                            debug_assert!(
                                child.has_independent_layout(self.context),
                                "Should be atomic",
                            );
                            items.push(InlineItem::AtomicInline(id));
                        }
                    }
                }
            }
        }

        items
    }
}

// https://drafts.csswg.org/css-text-3/#white-space-phase-1
fn collapse_and_transform(items: &mut Vec<InlineItem>) {

}

// https://drafts.csswg.org/css-text-3/#white-space-rules
fn process_whitespace(items: &mut Vec<InlineItem>) {
    collapse_and_transform(items);
}

impl<'a, 'b> super::LayoutAlgorithm for InlineFormattingContext<'a, 'b> {
    fn layout(&mut self, constraints: &ConstraintSpace) -> LayoutResult {
        debug_assert!(!self.input_node.establishes_ifc(self.context.layout_tree));

        let mut items = self.collect_inline_items_in(self.input_node);
        process_whitespace(&mut items);

        unimplemented!();
    }
}
