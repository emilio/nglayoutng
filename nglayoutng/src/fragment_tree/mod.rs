use crate::logical_geometry::*;
use crate::style::ComputedStyle;
use app_units::Au;
use euclid::default::{Point2D, Size2D};

/// A child fragment contains a given fragment and an offset relative to the
/// parent fragment.
pub struct ChildFragment {
    /// The offset relative to the parent fragment's origin.
    pub offset: LogicalPoint<Au>,
    /// The child fragment itself.
    ///
    /// TODO(emilio): We might want to refcount fragments or something.
    pub fragment: Box<Fragment>,
}

pub enum ContainerFragmentKind {
    Box {
        // TODO(emilio): Surely stuff will be needed here.
    },
    Line {
        // TODO(emilio): Surely stuff will be needed here.
    },
}

/// Fragments can be of multiple kinds, and are organized in a hierarchical way.
///
/// The fragment tree is a tree of physical boxes, and resembles the layout
/// tree.
///
/// A single fragment is immutable, and has no positioning information.
pub enum FragmentKind {
    Text {
        content: String,
    },
    Container {
        kind: ContainerFragmentKind,
        children: Box<[ChildFragment]>,
    },
}

/// A fragment is part of the result of layout, and it's immutable.
///
/// It contains only the sizing information, children are stored in .
pub struct Fragment {
    /// The physical size of this fragment.
    pub size: LogicalSize<Au>,
    /// The style of this fragment.
    pub style: ComputedStyle,
    /// Which kind of fragment this is.
    pub kind: FragmentKind,
}
