pub mod block;
// pub mod inline;

use crate::fragment_tree::ChildFragment;
use crate::layout_tree::LayoutTree;
use crate::logical_geometry::{LogicalSize, WritingMode};
use app_units::Au;
use html5ever::tree_builder::QuirksMode;

/// A struct that contains global information about this layout pass.
pub struct LayoutContext<'a> {
    /// The quirks mode of the document we're laying out.
    pub quirks_mode: QuirksMode,

    /// The layout tree.
    pub layout_tree: &'a LayoutTree,
}

pub struct AvailableSize(LogicalSize<Option<Au>>);

impl std::ops::Deref for AvailableSize {
    type Target = LogicalSize<Option<Au>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl AvailableSize {
    pub fn definite(wm: WritingMode, size: LogicalSize<Au>) -> Self {
        AvailableSize(LogicalSize::new(wm, Some(size.inline), Some(size.block)))
    }

    pub fn inline(&self) -> Au {
        self.inline.expect("Should never have unconstrained available inline size")
    }

    pub fn unconstrained_block(wm: WritingMode, inline: Au) -> Self {
        AvailableSize(LogicalSize::new(wm, Some(inline), None))
    }
}

/// The constraints we're using for a given layout.
pub struct ConstraintSpace {
    pub available_size: AvailableSize,
    pub percentage_resolution_size: AvailableSize,
    pub containing_block_writing_mode: WritingMode,
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
        constraints: &ConstraintSpace,
        break_token: Option<Self::BreakToken>,
    ) -> LayoutResult<Self::BreakToken>;
}
