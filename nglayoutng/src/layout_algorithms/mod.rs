pub mod block;
// pub mod inline;

use app_units::Au;
use euclid::Size2D;
use fragment_tree::ChildFragment;
use html5ever::tree_builder::QuirksMode;
use layout_tree::LayoutTree;
use logical_geometry::LogicalSize;

/// A struct that contains global information about this layout pass.
pub struct LayoutContext<'a> {
    /// The quirks mode of the document we're laying out.
    pub quirks_mode: QuirksMode,

    /// The size of the initial containing block.
    pub initial_containing_block_size: Size2D<Au>,

    /// The layout tree.
    pub layout_tree: &'a LayoutTree,
}

pub type AvailableSize = LogicalSize<Option<Au>>;

/// The constraints we're using for a given layout.
pub struct ConstraintSpace {
    pub available_size: AvailableSize,
    // TODO(emilio): Sure we need to add more stuff here.
}

#[derive(BreakToken)]
pub enum GenericBreakToken {
    Block(block::BreakToken),
}

/// A generic layout result from any layout algorithm.
pub struct GenericLayoutResult {
    pub root_fragment: ChildFragment,
    pub break_token: Option<GenericBreakToken>,
}

/// A layout result for a given layout algorithm.
pub struct LayoutResult<BreakToken> {
    /// The main fragment this layout pass has generated.
    pub root_fragment: ChildFragment,
    /// The break token allows to resume layout for the given layout algorithm
    /// and fragment.
    pub break_token: Option<BreakToken>,
}

impl<BreakToken> LayoutResult<BreakToken>
where
    BreakToken: Into<GenericBreakToken>,
{
    pub fn into_generic(self) -> GenericLayoutResult {
        GenericLayoutResult {
            root_fragment: self.root_fragment,
            break_token: self.break_token.map(Into::into),
        }
    }
}

pub trait LayoutAlgorithm {
    type BreakToken;

    fn layout(
        &self,
        context: &LayoutContext,
        constraints: &ConstraintSpace,
        break_token: Option<Self::BreakToken>,
    ) -> LayoutResult<Self::BreakToken>;
}
