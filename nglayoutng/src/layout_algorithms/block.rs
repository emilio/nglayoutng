use sizing;
use super::{ConstraintSpace, LayoutContext};
use layout_tree::LayoutNode;

pub type BreakToken = (); // TODO: Need to figure out fragmentation.

pub type LayoutResult = super::LayoutResult<BreakToken>;

pub struct BlockLayoutAlgorithm<'a> {
    input_node: &'a LayoutNode,
}

impl<'a> BlockLayoutAlgorithm<'a> {
    fn compute_min_max_size(&self) -> sizing::MinMaxSizes {
        unimplemented!();
    }

    pub fn new(input_node: &'a LayoutNode) -> Self {
        debug_assert!(input_node.is_container());
        Self { input_node }
    }
}

impl<'a> super::LayoutAlgorithm for BlockLayoutAlgorithm<'a> {
    type BreakToken = BreakToken;

    fn layout(
        &self,
        _context: &LayoutContext,
        constraints: &ConstraintSpace,
        _break_token: Option<BreakToken>,
    ) -> LayoutResult {
        let min_max = self.compute_min_max_size();
        let _pref_size = sizing::pref_size(
            &self.input_node.style,
            constraints,
            &min_max,
        );

        unimplemented!()
    }
}
