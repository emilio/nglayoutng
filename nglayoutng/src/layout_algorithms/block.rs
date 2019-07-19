use super::{ConstraintSpace, LayoutContext};
use crate::layout_tree::LayoutNode;
use crate::sizing;

pub enum BreakToken {}

pub type LayoutResult = super::LayoutResult<BreakToken>;

pub struct BlockFormattingContext<'a, 'b> {
    context: &'a LayoutContext<'b>,
    input_node: &'a LayoutNode,
}

impl<'a, 'b> BlockFormattingContext<'a, 'b> {
    pub fn new(context: &'a LayoutContext<'b>, input_node: &'a LayoutNode) -> Self {
        debug_assert!(input_node.is_block_container());
        Self { context, input_node }
    }

    fn compute_min_max_size(&self) -> sizing::MinMaxSizes {
        unimplemented!();
    }
}

impl<'a, 'b> super::LayoutAlgorithm for BlockFormattingContext<'a, 'b> {
    type BreakToken = BreakToken;

    fn layout(
        &self,
        _constraints: &ConstraintSpace,
        _break_token: Option<BreakToken>,
    ) -> LayoutResult {
        unimplemented!()
    }
}
