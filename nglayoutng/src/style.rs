use app_units::Au;
use euclid::{Size2D, SideOffsets2D};
use logical_geometry::{self, LogicalSize, LogicalMargin};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Display {
    None,
    Contents,
    Block,
    Inline,
    // ..
}

impl Display {
    fn blockify(self) -> Self {
        match self {
            Display::Block |
            Display::None |
            Display::Contents => self,
            Display::Inline => Display::Block,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Position {
    Static,
    Absolute,
    Fixed,
    Relative,
    Sticky,
}

impl Position {
    fn is_out_of_flow(&self) -> bool {
        match *self {
            Position::Absolute |
            Position::Fixed => true,
            Position::Static |
            Position::Relative |
            Position::Sticky => false,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Direction {
    Ltr,
    Rtl,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Float {
    Left,
    Right,
    None,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Clear {
    None,
    Left,
    Right,
    Both,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum WritingMode {
    HorizontalTb,
    VerticalRl,
    VerticalLr,
    SidewaysRl,
    SidewaysLr,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum TextOrientation {
    Mixed,
    Upright,
    Sideways,
}

/// A percentage in the range 0.0..1.0.
#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct Percentage(pub f32);

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Length(pub Au);

#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct LengthPercentage {
    pub fixed: Length,
    pub percentage: Option<Percentage>,
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum LengthPercentageOrAuto {
    LengthPercentage(LengthPercentage),
    Auto,
}

impl Default for LengthPercentageOrAuto {
    fn default() -> Self {
        LengthPercentageOrAuto::Auto
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PseudoElement {
    Before,
    After,
    Viewport,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MutableComputedStyle {
    pub pseudo: Option<PseudoElement>,
    pub writing_mode: logical_geometry::WritingMode,

    pub display: Display,
    /// The original display value of the item.
    ///
    /// Needed to compute hypothetical positions of abspos elements.
    pub original_display: Display,
    pub computed_writing_mode: WritingMode,
    pub position: Position,
    pub float: Float,
    pub clear: Clear,
    pub direction: Direction,
    pub text_orientation: TextOrientation,

    pub width: LengthPercentageOrAuto,
    pub height: LengthPercentageOrAuto,

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

impl MutableComputedStyle {
    /// Finish mutating this style.
    pub fn finish(mut self, is_root_element: bool) -> ComputedStyle {
        self.original_display = self.display;
        if self.is_floating() || self.is_out_of_flow() || is_root_element {
            self.display = self.display.blockify();
        }
        ComputedStyle(self)
    }
}

/// A version of `MutableComputedStyle` that can't be mutated. This is enforced
/// by the field being private and only `Deref` (but not `DerefMut`) being
/// implemented.
#[derive(Debug, Clone, PartialEq)]
pub struct ComputedStyle(MutableComputedStyle);
impl ::std::ops::Deref for ComputedStyle {
    type Target = MutableComputedStyle;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl MutableComputedStyle {
    pub fn is_floating(&self) -> bool {
        self.float != Float::None
    }

    pub fn is_out_of_flow(&self) -> bool {
        self.position.is_out_of_flow() || self.is_floating()
    }
}

impl ComputedStyle {
    pub fn initial() -> MutableComputedStyle {
        let direction = Direction::Ltr;
        let text_orientation = TextOrientation::Mixed;
        let writing_mode = WritingMode::HorizontalTb;

        MutableComputedStyle {
            pseudo: None,
            writing_mode: logical_geometry::WritingMode::new(
                direction,
                writing_mode,
                text_orientation,
            ),
            direction,
            computed_writing_mode: writing_mode,
            text_orientation,
            display: Display::Inline,
            original_display: Display::Inline,
            position: Position::Static,
            float: Float::None,
            clear: Clear::None,

            width: Default::default(),
            height: Default::default(),

            padding_top: Default::default(),
            padding_right: Default::default(),
            padding_bottom: Default::default(),
            padding_left: Default::default(),

            margin_top: Default::default(),
            margin_right: Default::default(),
            margin_bottom: Default::default(),
            margin_left: Default::default(),

            border_top_width: Default::default(),
            border_right_width: Default::default(),
            border_bottom_width: Default::default(),
            border_left_width: Default::default(),

            top: Default::default(),
            right: Default::default(),
            bottom: Default::default(),
            left: Default::default(),
        }
    }

    pub fn inherited(&self) -> MutableComputedStyle {
        MutableComputedStyle {
            direction: self.direction,
            writing_mode: self.writing_mode,
            text_orientation: self.text_orientation,
            computed_writing_mode: self.computed_writing_mode,
            .. Self::initial()
        }
    }

    pub fn for_viewport() -> ComputedStyle {
        MutableComputedStyle {
            pseudo: Some(PseudoElement::Viewport),
            display: Display::Block,
            original_display: Display::Block,
            .. Self::initial()
        }.finish(false)
    }


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

    fn physical_size(&self) -> Size2D<LengthPercentageOrAuto> {
        Size2D::new(
            self.width,
            self.height
        )
    }

    pub fn size(&self) -> LogicalSize<LengthPercentageOrAuto> {
        LogicalSize::from_physical(self.writing_mode, self.physical_size())
    }

    pub fn margin(&self) -> LogicalMargin<LengthPercentage> {
        LogicalMargin::from_physical(self.writing_mode, self.physical_margin())
    }

    pub fn padding(&self) -> LogicalMargin<LengthPercentage> {
        LogicalMargin::from_physical(self.writing_mode, self.physical_padding())
    }

    pub fn border_widths(&self) -> LogicalMargin<LengthPercentage> {
        LogicalMargin::from_physical(self.writing_mode, self.physical_border_widths())
    }
}
