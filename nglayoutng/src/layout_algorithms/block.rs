use app_units::Au;
use super::{ConstraintSpace, LayoutContext};
use layout_tree::LayoutNode;
use logical_geometry::LogicalSize;
use style::Size;

pub type BreakToken = (); // TODO: Need to figure out fragmentation.

pub type LayoutResult = super::LayoutResult<BreakToken>;

pub struct BlockLayoutAlgorithm<'a> {
    input_node: &'a LayoutNode,
}

impl<'a> BlockLayoutAlgorithm<'a> {
    pub fn new(input_node: &'a LayoutNode) -> Self {
        debug_assert!(input_node.is_container());
        Self { input_node }
    }

    pub fn pref_size(&self, constraints: &ConstraintSpace) -> LogicalSize<Au> {
        let style = &self.input_node.style;

        let cb_size = constraints
            .cb_size()
            .convert(constraints.containing_block_writing_mode, style.writing_mode);

        let pref_size = style.size();

        let inline_size = match pref_size.inline {
            Size::LengthPercentage(ref lop) => {
                lop.resolve(cb_size.inline)
            }
            Size::Keyword(_keyword) => unimplemented!(),
        };


        let block_size = match pref_size.block {
            Size::LengthPercentage(ref lop) => {
                lop.resolve(cb_size.inline)
            }
            Size::Keyword(_keyword) => unimplemented!(),
        };

        LogicalSize::new(style.writing_mode, inline_size, block_size)
    }
}

impl<'a> super::LayoutAlgorithm for BlockLayoutAlgorithm<'a> {
    type BreakToken = BreakToken;

    fn layout(
        &self,
        _context: &LayoutContext,
        _constraints: &ConstraintSpace,
        _break_token: Option<BreakToken>,
    ) -> LayoutResult {
        unimplemented!()
    }
}
