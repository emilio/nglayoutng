use crate::layout_algorithms::ConstraintSpace;
use crate::logical_geometry::LogicalSize;
use crate::style::ComputedStyle;
use app_units::Au;

/// A resolved size is either an automatic size, or an actual used value.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ResolvedSize {
    Auto,
    Resolved(Au),
}

/// The min and max content sizes. These are always inline sizes.
pub struct MinMaxSizes {
    pub min_content: Au,
    pub max_content: Au,
}

/// Computes the preferred size of a given box.
pub fn pref_size(
    _style: &ComputedStyle,
    _constraints: &ConstraintSpace,
    _min_max: &MinMaxSizes,
) -> LogicalSize<ResolvedSize> {
    /*
    use crate::style::{Size, SizeKeyword};

    let cb_size = constraints.cb_size().convert(
        constraints.containing_block_writing_mode,
        style.writing_mode,
    );

    let pref_size = style.size();
    let inline_size = match pref_size.inline {
        Size::LengthPercentage(ref lop) => ResolvedSize::Resolved(lop.resolve(cb_size.inline)),
        Size::Keyword(keyword) => match keyword {
            SizeKeyword::Auto => ResolvedSize::Auto,
            SizeKeyword::MinContent => ResolvedSize::Resolved(min_max.min_content),
            SizeKeyword::MaxContent => ResolvedSize::Resolved(min_max.max_content),
        },
    };

    let block_size = match pref_size.block {
        Size::LengthPercentage(ref lop) => ResolvedSize::Resolved(lop.resolve(cb_size.inline)),
        Size::Keyword(keyword) => match keyword {
            SizeKeyword::Auto | SizeKeyword::MaxContent | SizeKeyword::MinContent => {
                ResolvedSize::Auto
            },
        },
    };

    LogicalSize::new(style.writing_mode, inline_size, block_size)
     */
    unimplemented!()
}
