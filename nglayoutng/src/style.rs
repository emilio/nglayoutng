use crate::logical_geometry::{self, LogicalMargin, LogicalSize};
use app_units::Au;
use cssparser::{Color, RGBA};
use euclid::default::{SideOffsets2D, Size2D};

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DisplayInside {
    None,
    Flow,
    FlowRoot,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum DisplayOutside {
    None,
    Contents,
    Block,
    Inline,
    // ..
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct Display {
    outside: DisplayOutside,
    inside: DisplayInside,
    is_list_item: bool,
}

impl Display {
    fn new_list_item(outside: DisplayOutside, inside: DisplayInside, is_list_item: bool) -> Self {
        Self { outside, inside, is_list_item }
    }

    fn new(outside: DisplayOutside, inside: DisplayInside) -> Self {
        Self::new_list_item(outside, inside, false)
    }

    fn block() -> Self {
        Self::new(DisplayOutside::Block, DisplayInside::Flow)
    }

    fn inline() -> Self {
        Self::new(DisplayOutside::Inline, DisplayInside::Flow)
    }

    pub fn parse<'i, 't>(
        input: &mut cssparser::Parser<'i, 't>,
    ) -> Result<Self, crate::css::ParseError<'i>> {
        let location = input.current_source_location();
        let ident = input.expect_ident()?;
        // TODO(emilio): Multi-keyword syntax.
        Ok(match_ignore_ascii_case! { ident,
            "contents" => Self::new(DisplayOutside::Contents, DisplayInside::None),
            "none" => Self::new(DisplayOutside::None, DisplayInside::None),
            "block" => Self::block(),
            "inline" => Self::inline(),
            "flow-root" => Self::new(DisplayOutside::Block, DisplayInside::FlowRoot),
            "inline-block" => Self::new(DisplayOutside::Inline, DisplayInside::FlowRoot),
            "list-item" => Self::new_list_item(DisplayOutside::Block, DisplayInside::Flow, true),
            _ => return Err(location.new_unexpected_token_error(
                cssparser::Token::Ident(ident.clone())
            )),
        })
    }

    pub fn inside(&self) -> DisplayInside {
        self.inside
    }

    pub fn outside(&self) -> DisplayOutside {
        self.outside
    }

    pub fn is_list_item(&self) -> bool {
        self.is_list_item
    }

    pub fn is_none(&self) -> bool {
        self.outside() == DisplayOutside::None
    }

    pub fn is_contents(&self) -> bool {
        self.outside() == DisplayOutside::Contents
    }

    pub fn is_inline_inside(&self) -> bool {
        self.outside() == DisplayOutside::Inline &&
            self.inside() == DisplayInside::Flow
    }

    pub fn is_block_outside(self) -> bool {
        match self.outside() {
            DisplayOutside::Block => true,
            DisplayOutside::None | DisplayOutside::Contents | DisplayOutside::Inline => false,
        }
    }

    pub fn is_inline_outside(self) -> bool {
        match self.outside() {
            DisplayOutside::None | DisplayOutside::Contents | DisplayOutside::Block => false,
            DisplayOutside::Inline => true,
        }
    }

    fn blockify(self) -> Self {
        let outside = match self.outside() {
            DisplayOutside::Block | DisplayOutside::Contents | DisplayOutside::None => return self,
            DisplayOutside::Inline => DisplayOutside::Block,
        };

        let inside = match self.inside() {
            // inline-block blockifies to block, not to flow-root, for legacy
            // reasons.
            DisplayInside::FlowRoot => DisplayInside::Flow,
            inside => inside,
        };

        Self::new_list_item(outside, inside, self.is_list_item())
    }
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Keyword)]
pub enum BoxSizing {
    ContentBox,
    BorderBox,
}

impl BoxSizing {
    pub fn border_box(self) -> bool {
        self == BoxSizing::BorderBox
    }

    pub fn content_box(self) -> bool {
        self == BoxSizing::ContentBox
    }
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

#[derive(Default, Debug, Clone, PartialEq)]
pub struct LengthPercentage {
    pub fixed: Length,
    pub percentage: Option<Percentage>,
}

impl LengthPercentage {
    #[inline]
    pub fn resolve(&self, percentage_resolution_size: Au) -> Au {
        self.fixed.0 +
            self.percentage
                .map_or(Au(0), |p| percentage_resolution_size.scale_by(p.0))
    }

    /// Resolve a `LengthPercentage` value against a resolution size, if
    /// present.
    #[inline]
    pub fn maybe_resolve(&self, percentage_resolution_size: Option<Au>) -> Option<Au> {
        let mut result = self.fixed.0;
        if let Some(percentage) = self.percentage {
            result += percentage_resolution_size?.scale_by(percentage.0);
        }
        Some(result)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum LengthPercentageOrAuto {
    LengthPercentage(LengthPercentage),
    Auto,
}

impl Default for LengthPercentageOrAuto {
    fn default() -> Self {
        LengthPercentageOrAuto::Auto
    }
}

/// https://drafts.csswg.org/css-sizing/#sizing-properties
#[derive(Debug, Copy, Clone, PartialEq, Eq, Keyword)]
pub enum SizeKeyword {
    Auto,
    MinContent,
    MaxContent,
}

impl Default for SizeKeyword {
    fn default() -> Self {
        SizeKeyword::Auto
    }
}

/// https://drafts.csswg.org/css-sizing/#sizing-properties
#[derive(Debug, Clone, PartialEq)]
pub enum Size {
    LengthPercentage(LengthPercentage),
    Keyword(SizeKeyword),
    // TODO(emilio): fit-content?
}

impl Default for Size {
    fn default() -> Self {
        Size::Keyword(SizeKeyword::Auto)
    }
}

#[derive(Debug, Copy, Clone, PartialEq)]
pub enum PseudoElement {
    Before,
    After,
    Viewport,
    /// An anonymous block wrapping inline contents directly inside another
    /// block.
    InlineInsideBlockWrapper,
    /// An anonymous block wrapping a block inserted inside an inline.
    BlockInsideInlineWrapper,
    /// An anonymous inline box for the continuation of an inline.
    InlineContinuation,
}

impl PseudoElement {
    /// Returns whether this pseudo-style is for an anonymous box.
    #[inline]
    pub fn is_anonymous(self) -> bool {
        match self {
            PseudoElement::Before | PseudoElement::After => false,
            PseudoElement::Viewport |
            PseudoElement::InlineInsideBlockWrapper |
            PseudoElement::InlineContinuation |
            PseudoElement::BlockInsideInlineWrapper => true,
        }
    }
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

    pub width: Size,
    pub height: Size,

    pub min_width: Size,
    pub min_height: Size,

    pub max_width: Size,
    pub max_height: Size,

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

        if self.overflow_x != self.overflow_y {
            if self.overflow_x == Overflow::Visible {
                self.overflow_y = Overflow::Auto;
            }
            if self.overflow_y == Overflow::Visible {
                self.overflow_x = Overflow::Auto;
            }
        }

        // FIXME(emilio): Blockify flex items and such.
        if self.is_out_of_flow() || is_root_element {
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

    pub fn is_out_of_flow_positioned(&self) -> bool {
        self.position.is_out_of_flow()
    }

    pub fn is_out_of_flow(&self) -> bool {
        self.is_out_of_flow_positioned() || self.is_floating()
    }

    pub fn is_ib_split_wrapper(&self) -> bool {
        self.pseudo
            .map_or(false, |p| p == PseudoElement::BlockInsideInlineWrapper)
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
            display: Display::inline(),
            original_display: Display::inline(),
            position: Position::Static,
            box_sizing: BoxSizing::ContentBox,
            float: Float::None,
            clear: Clear::None,
            overflow_x: Overflow::Visible,
            overflow_y: Overflow::Visible,

            width: Default::default(),
            height: Default::default(),

            min_width: Default::default(),
            min_height: Default::default(),

            max_width: Default::default(),
            max_height: Default::default(),

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

    pub fn for_viewport() -> Self {
        Self::new_anonymous(PseudoElement::Viewport, Display::block())
    }

    pub fn for_ib_split_block_wrapper() -> Self {
        Self::new_anonymous(PseudoElement::BlockInsideInlineWrapper, Display::block())
    }

    pub fn for_inline_inside_block_wrapper() -> Self {
        Self::new_anonymous(PseudoElement::InlineInsideBlockWrapper, Display::block())
    }

    pub fn new_anonymous(pseudo: PseudoElement, display: Display) -> Self {
        debug_assert!(pseudo.is_anonymous());
        MutableComputedStyle {
            pseudo: Some(pseudo),
            display,
            original_display: display,
            ..Self::initial()
        }
        .finish(false)
    }

    fn physical_padding(&self) -> SideOffsets2D<&LengthPercentage> {
        SideOffsets2D::new(
            &self.padding_top,
            &self.padding_right,
            &self.padding_bottom,
            &self.padding_left,
        )
    }

    fn physical_margin(&self) -> SideOffsets2D<&LengthPercentageOrAuto> {
        SideOffsets2D::new(
            &self.margin_top,
            &self.margin_right,
            &self.margin_bottom,
            &self.margin_left,
        )
    }

    fn physical_border_widths(&self) -> SideOffsets2D<Au> {
        SideOffsets2D::new(
            self.border_top_width.0,
            self.border_right_width.0,
            self.border_bottom_width.0,
            self.border_left_width.0,
        )
    }

    fn physical_size(&self) -> Size2D<&Size> {
        Size2D::new(&self.width, &self.height)
    }

    pub fn size(&self) -> LogicalSize<&Size> {
        LogicalSize::from_physical(self.writing_mode, self.physical_size())
    }

    fn physical_max_size(&self) -> Size2D<&Size> {
        Size2D::new(&self.max_width, &self.max_height)
    }

    pub fn max_size(&self) -> LogicalSize<&Size> {
        LogicalSize::from_physical(self.writing_mode, self.physical_max_size())
    }

    fn physical_min_size(&self) -> Size2D<&Size> {
        Size2D::new(&self.min_width, &self.min_height)
    }

    pub fn min_size(&self) -> LogicalSize<&Size> {
        LogicalSize::from_physical(self.writing_mode, self.physical_min_size())
    }

    pub fn margin(&self) -> LogicalMargin<&LengthPercentageOrAuto> {
        LogicalMargin::from_physical(self.writing_mode, self.physical_margin())
    }

    pub fn padding(&self) -> LogicalMargin<&LengthPercentage> {
        LogicalMargin::from_physical(self.writing_mode, self.physical_padding())
    }

    pub fn border_widths(&self) -> LogicalMargin<Au> {
        LogicalMargin::from_physical(self.writing_mode, self.physical_border_widths())
    }
}
