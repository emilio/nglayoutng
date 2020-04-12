use app_units::Au;
use crate::fragment_tree::{ChildFragment, Fragment, FragmentKind, ContainerFragmentKind};
use super::{ConstraintSpace, LayoutContext, LayoutResult};
use crate::layout_tree::{LayoutNode, LayoutNodeId};

pub struct BlockFormattingContext<'a, 'b> {
    context: &'a LayoutContext<'b>,
    input_node: &'a LayoutNode,
}

impl<'a, 'b> BlockFormattingContext<'a, 'b> {
    pub fn new(context: &'a LayoutContext<'b>, input_node: &'a LayoutNode) -> Self {
        debug_assert!(input_node.is_block_container());
        Self {
            context,
            input_node,
        }
    }

    fn layout_block_children(&mut self, constraints: &ConstraintSpace) -> LayoutResult {
        let node = self.input_node;
        let style = &node.style;
        let wm = style.writing_mode;
        let border = style.border_widths();
        let padding = style.padding().map_all(|lp| {
            lp.resolve(constraints.percentage_resolution_size.inline())
        });

        let bp = border + padding;
        let _children_constraints = {
            let mut child_avail_size = constraints.available_size.clone();
            child_avail_size.shrink_block_size(bp.block_start_end());
            child_avail_size.shrink_inline_size(bp.inline_start_end());
            ConstraintSpace {
                available_size: child_avail_size.clone(),
                percentage_resolution_size: child_avail_size,
                containing_block_writing_mode: wm,
            }
        };

        for (id, child) in self.input_node.children_and_id(self.context.layout_tree) {
            if child.style.is_out_of_flow_positioned() {
                // FIXME: We need to do something with the static position.
                continue;
            }
            if child.style.is_floating() {

            }
        }

        LayoutResult {
            root_fragment: ChildFragment {
                offset: euclid::point2(Au(0), Au(0)),
                fragment: Box::new(Fragment {
                    size: euclid::size2(Au(0), Au(0)),
                    style: self.input_node.style.clone(),
                    kind: FragmentKind::Container {
                        kind: ContainerFragmentKind::Line {},
                        children: Box::new([]),
                    },
                }),
            }
        }
    }

    fn layout_inline_children(&mut self, _: &ConstraintSpace) -> LayoutResult {
        unimplemented!()
    }
}

impl<'a, 'b> super::LayoutAlgorithm for BlockFormattingContext<'a, 'b> {
    fn layout(&mut self, constraints: &ConstraintSpace) -> LayoutResult {
        if self.input_node.establishes_ifc(self.context.layout_tree) {
            self.layout_inline_children(constraints)
        } else {
            self.layout_block_children(constraints)
        }
    }
}
