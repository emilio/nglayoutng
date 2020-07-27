use app_units::Au;
use crate::fragment_tree::{ChildFragment, Fragment, FragmentKind, ContainerFragmentKind};
use super::{ConstraintSpace, LayoutContext, LayoutResult};
use crate::layout_tree::LayoutNode;
use crate::logical_geometry::*;
use crate::style::*;

pub struct BlockFormattingContext<'a, 'b> {
    context: &'a LayoutContext<'b>,
    input_node: &'a LayoutNode,
}

#[derive(Default)]
struct BlockLayoutState {
    current_offset: Au,
}

impl<'a, 'b> BlockFormattingContext<'a, 'b> {
    pub fn new(context: &'a LayoutContext<'b>, input_node: &'a LayoutNode) -> Self {
        debug_assert!(input_node.is_block_container());
        Self {
            context,
            input_node,
        }
    }

    fn is_root(&self, node: &LayoutNode) -> bool {
        std::ptr::eq(node, self.input_node)
    }

    fn layout_block_children_of(
        &mut self,
        state: &mut BlockLayoutState,
        node: &LayoutNode,
        constraints: &ConstraintSpace,
    ) -> LayoutResult {
        let style = &node.style;
        let wm = style.writing_mode;
        let border = style.border_widths();
        let padding = style.padding().map_all(|lp| {
            lp.resolve(constraints.percentage_resolution_size.inline())
        });

        let start_block_offset = state.current_offset;
        if !self.is_root(node) {
            // TODO(margins / margin-collapsing)
        }

        let bp = border + padding;
        let my_inline_border_box_size = match style.size().inline {
            Size::Keyword(SizeKeyword::Auto) => constraints.available_size.inline(),
            Size::Keyword(SizeKeyword::MaxContent) |
            Size::Keyword(SizeKeyword::MinContent) => {
                // TODO(minmax)
                Au(0)
            },
            Size::LengthPercentage(lp) => {
                let mut size = lp.resolve(constraints.percentage_resolution_size.inline());
                if style.box_sizing.content_box() {
                    size += bp.inline_start_end();
                }
                size
            }
        };

        let children_constraints = {
            let mut child_avail_size = constraints.available_size.clone();
            child_avail_size.shrink_block_size(bp.block_start_end());
            child_avail_size.shrink_inline_size(bp.inline_start_end());
            ConstraintSpace {
                available_size: child_avail_size.clone(),
                percentage_resolution_size: child_avail_size,
                containing_block_writing_mode: wm,
            }
        };

        let mut children = vec![];
        for (_id, child) in node.children_and_id(self.context.layout_tree) {
            if child.style.is_out_of_flow_positioned() {
                let _static_pos = LogicalPoint::new(
                    wm,
                    bp.block_start + state.current_offset,
                    bp.inline_start,
                );
                // FIXME: We need to do something with the static position.
                continue;
            }
            if child.style.is_floating() {
                // TODO(floats)
                continue;
            }
            let mut child_result = if child.has_independent_layout(self.context) {
                child.layout(self.context, &children_constraints)
            } else {
                debug_assert!(child.is_block_container());
                self.layout_block_children_of(state, child, &children_constraints)
            };
            child_result.root_fragment.offset.b = state.current_offset;
            state.current_offset += child_result.root_fragment.fragment.size.block;
            children.push(child_result);
        }

        LayoutResult {
            root_fragment: ChildFragment {
                offset: LogicalPoint::zero(wm),
                fragment: Box::new(Fragment {
                    size: LogicalSize::new(wm, my_inline_border_box_size, state.current_offset - start_block_offset),
                    style: node.style.clone(),
                    kind: FragmentKind::Container {
                        kind: ContainerFragmentKind::Line {},
                        children: Box::new([]),
                    },
                }),
            }
        }
    }
}

impl<'a, 'b> super::LayoutAlgorithm for BlockFormattingContext<'a, 'b> {
    fn layout(&mut self, constraints: &ConstraintSpace) -> LayoutResult {
        debug_assert!(!self.input_node.establishes_ifc(self.context.layout_tree));
        let mut state = BlockLayoutState::default();
        self.layout_block_children_of(&mut state, self.input_node, constraints)
    }
}
