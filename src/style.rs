use app_units::Au;
use euclid::{Size2D, SideOffsets2D};
use logical_geometry::{self, LogicalSize, LogicalMargin};

#[derive(Debug, Copy, Clone)]
pub enum Display {
    None,
    Block,
    Inline,
    // ..
}

#[derive(Debug, Copy, Clone)]
pub enum Position {
    Static,
    Absolute,
    Fixed,
    Relative,
    Sticky,
}

#[derive(Debug, Copy, Clone)]
pub enum Direction {
    Ltr,
    Rtl,
}

#[derive(Debug, Copy, Clone)]
pub enum WritingMode {
    HorizontalTb,
    VerticalRl,
    VerticalLr,
    SidewaysRl,
    SidewaysLr,
}

#[derive(Debug, Copy, Clone)]
pub enum TextOrientation {
    Mixed,
    Upright,
    Sideways,
}

/// A percentage in the range 0.0..1.0.
#[derive(Default, Debug, Copy, Clone)]
pub struct Percentage(pub f32);

#[derive(Default, Debug, Copy, Clone)]
pub struct Length(pub Au);

#[derive(Default, Debug, Copy, Clone)]
pub struct LengthPercentage {
    pub fixed: Au,
    pub percentage: Option<Percentage>,
}

pub struct ComputedStyle {
    used_writing_mode: logical_geometry::WritingMode,

    pub display: Display,
    pub direction: Direction,
    pub writing_mode: WritingMode,
    pub text_orientation: TextOrientation,

    pub width: LengthPercentage,
    pub height: LengthPercentage,

    pub padding_top: LengthPercentage,
    pub padding_right: LengthPercentage,
    pub padding_bottom: LengthPercentage,
    pub padding_left: LengthPercentage,

    pub margin_top: LengthPercentage,
    pub margin_right: LengthPercentage,
    pub margin_bottom: LengthPercentage,
    pub margin_left: LengthPercentage,

    pub border_top_width: LengthPercentage,
    pub border_right_width: LengthPercentage,
    pub border_bottom_width: LengthPercentage,
    pub border_left_width: LengthPercentage,

    pub top: LengthPercentage,
    pub right: LengthPercentage,
    pub bottom: LengthPercentage,
    pub left: LengthPercentage,
}

impl ComputedStyle {
    fn physical_padding(&self) -> SideOffsets2D<LengthPercentage> {
        SideOffsets2D::new(
            self.padding_top,
            self.padding_right,
            self.padding_bottom,
            self.padding_left,
        )
    }

    fn physical_margin(&self) -> SideOffsets2D<LengthPercentage> {
        SideOffsets2D::new(
            self.margin_top,
            self.margin_right,
            self.margin_bottom,
            self.margin_left,
        )
    }

    fn physical_border_widths(&self) -> SideOffsets2D<LengthPercentage> {
        SideOffsets2D::new(
            self.border_top_width,
            self.border_right_width,
            self.border_bottom_width,
            self.border_left_width,
        )
    }

    fn physical_size(&self) -> Size2D<LengthPercentage> {
        Size2D::new(
            self.width,
            self.height
        )
    }

    pub fn size(&self) -> LogicalSize<LengthPercentage> {
        LogicalSize::from_physical(self.used_writing_mode, self.physical_size())
    }

    pub fn margin(&self) -> LogicalMargin<LengthPercentage> {
        LogicalMargin::from_physical(self.used_writing_mode, self.physical_margin())
    }

    pub fn padding(&self) -> LogicalMargin<LengthPercentage> {
        LogicalMargin::from_physical(self.used_writing_mode, self.physical_padding())
    }

    pub fn border_widths(&self) -> LogicalMargin<LengthPercentage> {
        LogicalMargin::from_physical(self.used_writing_mode, self.physical_border_widths())
    }
}
