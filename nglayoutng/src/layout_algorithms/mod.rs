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

#[derive(Clone, Debug)]
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

    pub fn unconstrained_block(wm: WritingMode, inline: Au) -> Self {
        AvailableSize(LogicalSize::new(wm, Some(inline), None))
    }

    pub fn inline(&self) -> Au {
        self.inline.expect("Should never have unconstrained available inline size")
    }

    pub fn shrink_block_size(&mut self, by: Au) {
        if let Some(ref mut block) = self.0.block {
            *block -= by;
            if *block < Au(0) {
                *block = Au(0);
            }
        }
    }

    pub fn shrink_inline_size(&mut self, by: Au) {
        let inline = self.0.inline.as_mut().unwrap();
        *inline -= by;
        if *inline < Au(0) {
            *inline = Au(0);
        }
    }
}

/// The constraints we're using for a given layout.
pub struct ConstraintSpace {
    pub available_size: AvailableSize,
    pub percentage_resolution_size: AvailableSize,
    pub containing_block_writing_mode: WritingMode,
    // TODO(emilio): Sure we need to add more stuff here.
}

/// A layout result for a given layout algorithm.
pub struct LayoutResult {
    /// The main fragment this layout pass has generated.
    pub root_fragment: ChildFragment,
}

pub trait LayoutAlgorithm {
    fn layout(&mut self, constraints: &ConstraintSpace) -> LayoutResult;
}
