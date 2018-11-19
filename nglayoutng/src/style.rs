use app_units::Au;
use cssparser::{Color, RGBA};
use euclid::{SideOffsets2D, Size2D};
use logical_geometry::{self, LogicalMargin, LogicalSize};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Keyword)]
pub enum Display {
    None,
    Contents,
    Block,
    FlowRoot,
    Inline,
    // ..
}

impl Display {
    fn blockify(self) -> Self {
        match self {
            Display::Block | Display::FlowRoot | Display::None | Display::Contents => self,
            Display::Inline => Display::Block,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Keyword)]
pub enum BoxSizing {
    ContentBox,
    BorderBox,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Keyword)]
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
            Position::Absolute | Position::Fixed => true,
            Position::Static | Position::Relative | Position::Sticky => false,
        }
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Keyword)]
pub enum Direction {
    Ltr,
    Rtl,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Keyword)]
pub enum Float {
    Left,
    Right,
    None,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Keyword)]
pub enum Overflow {
    Visible,
    Hidden,
    Scroll,
    Auto,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Keyword)]
pub enum Clear {
    None,
    Left,
    Right,
    Both,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Keyword)]
pub enum BorderStyle {
    None,
    Solid,
    Double,
    Dotted,
    Dashed,
    Hidden,
    Groove,
    Ridge,
    Inset,
    Outset,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Keyword)]
pub enum WritingMode {
    HorizontalTb,
    VerticalRl,
    VerticalLr,
    SidewaysRl,
    SidewaysLr,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Keyword)]
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
    pub box_sizing: BoxSizing,
    pub float: Float,
    pub clear: Clear,
    pub overflow_x: Overflow,
    pub overflow_y: Overflow,
    pub direction: Direction,
    pub text_orientation: TextOrientation,

    pub color: RGBA,
    pub background_color: Color,

    pub width: LengthPercentageOrAuto,
    pub height: LengthPercentageOrAuto,

    pub padding_top: LengthPercentage,
    pub padding_right: LengthPercentage,
    pub padding_bottom: LengthPercentage,
    pub padding_left: LengthPercentage,

    pub margin_top: LengthPercentageOrAuto,
    pub margin_right: LengthPercentageOrAuto,
    pub margin_bottom: LengthPercentageOrAuto,
    pub margin_left: LengthPercentageOrAuto,

    pub border_top_width: Length,
    pub border_right_width: Length,
    pub border_bottom_width: Length,
    pub border_left_width: Length,

    pub border_top_style: BorderStyle,
    pub border_right_style: BorderStyle,
    pub border_bottom_style: BorderStyle,
    pub border_left_style: BorderStyle,

    pub border_top_color: Color,
    pub border_right_color: Color,
    pub border_bottom_color: Color,
    pub border_left_color: Color,

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
            color: RGBA::new(0, 0, 0, 255),
            background_color: Color::RGBA(RGBA::transparent()),
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
            box_sizing: BoxSizing::ContentBox,
            float: Float::None,
            clear: Clear::None,
            overflow_x: Overflow::Visible,
            overflow_y: Overflow::Visible,

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

            border_top_style: BorderStyle::None,
            border_right_style: BorderStyle::None,
            border_bottom_style: BorderStyle::None,
            border_left_style: BorderStyle::None,

            border_top_color: Color::CurrentColor,
            border_right_color: Color::CurrentColor,
            border_bottom_color: Color::CurrentColor,
            border_left_color: Color::CurrentColor,

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
            color: self.color,
            ..Self::initial()
        }
    }

    pub fn for_viewport() -> ComputedStyle {
        MutableComputedStyle {
            pseudo: Some(PseudoElement::Viewport),
            display: Display::Block,
            original_display: Display::Block,
            ..Self::initial()
        }
        .finish(false)
    }

    fn physical_padding(&self) -> SideOffsets2D<LengthPercentage> {
        SideOffsets2D::new(
            self.padding_top,
            self.padding_right,
            self.padding_bottom,
            self.padding_left,
        )
    }

    fn physical_margin(&self) -> SideOffsets2D<LengthPercentageOrAuto> {
        SideOffsets2D::new(
            self.margin_top,
            self.margin_right,
            self.margin_bottom,
            self.margin_left,
        )
    }

    fn physical_border_widths(&self) -> SideOffsets2D<Length> {
        SideOffsets2D::new(
            self.border_top_width,
            self.border_right_width,
            self.border_bottom_width,
            self.border_left_width,
        )
    }

    fn physical_size(&self) -> Size2D<LengthPercentageOrAuto> {
        Size2D::new(self.width, self.height)
    }

    pub fn size(&self) -> LogicalSize<LengthPercentageOrAuto> {
        LogicalSize::from_physical(self.writing_mode, self.physical_size())
    }

    pub fn margin(&self) -> LogicalMargin<LengthPercentageOrAuto> {
        LogicalMargin::from_physical(self.writing_mode, self.physical_margin())
    }

    pub fn padding(&self) -> LogicalMargin<LengthPercentage> {
        LogicalMargin::from_physical(self.writing_mode, self.physical_padding())
    }

    pub fn border_widths(&self) -> LogicalMargin<Length> {
        LogicalMargin::from_physical(self.writing_mode, self.physical_border_widths())
    }
}
