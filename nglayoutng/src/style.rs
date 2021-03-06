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
pub enum WhiteSpace {
    Normal,
    Pre,
    Nowrap,
    PreWrap,
    BreakSpaces,
    PreLine,
}

impl WhiteSpace {
    // https://drafts.csswg.org/css-text-3/#white-space-phase-1
    pub fn collapses_spaces(self) -> bool {
        match self {
            Self::Normal | Self::Nowrap | Self::PreLine => true,
            _ => false,
        }
    }

    // https://drafts.csswg.org/css-text-3/#line-break-transform
    pub fn collapses_newlines(self) -> bool {
        match self {
            Self::Pre | Self::PreWrap | Self::PreLine | Self::BreakSpaces => false,
            _ => {
                debug_assert!(self.collapses_spaces());
                true
            }
        }
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

#[derive(Debug, Copy, Clone, PartialEq, Eq, Keyword)]
pub enum FontWeight {
    Normal,
    Bold,
    // TODO: Numbers
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Keyword)]
pub enum FontStyle {
    Normal,
    Italic,
    // TODO: Oblique <angle>
}

#[derive(Debug, Copy, Clone, PartialEq, Eq, Keyword)]
pub enum GenericFamily {
    Serif,
    SansSerif,
    Monospace,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum FontFamilyNameSyntax {
    Quoted,
    Identifiers,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedFamily {
    pub name: String,
    syntax: FontFamilyNameSyntax,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SingleFontFamily {
    Generic(GenericFamily),
    Named(NamedFamily),
}

impl SingleFontFamily {
    pub fn parse<'i, 't>(
        input: &mut cssparser::Parser<'i, 't>,
    ) -> Result<Self, crate::css::ParseError<'i>> {
        if let Ok(name) = input.try_parse(|input| input.expect_string_cloned()) {
            return Ok(SingleFontFamily::Named(NamedFamily {
                name: name.as_ref().to_owned(),
                syntax: FontFamilyNameSyntax::Quoted,
            }))
        }

        if let Ok(generic) = input.try_parse(GenericFamily::parse) {
            return Ok(SingleFontFamily::Generic(generic));
        }

        let first_ident = input.expect_ident()?;
        let reserved = match_ignore_ascii_case! { &first_ident,
            // https://drafts.csswg.org/css-fonts/#propdef-font-family
            // "Font family names that happen to be the same as a keyword value
            //  (`inherit`, `serif`, `sans-serif`, `monospace`, `fantasy`, and `cursive`)
            //  must be quoted to prevent confusion with the keywords with the same names.
            //  The keywords ‘initial’ and ‘default’ are reserved for future use
            //  and must also be quoted when used as font names.
            //  UAs must not consider these keywords as matching the <family-name> type."
            "inherit" | "initial" | "unset" | "revert" | "default" => true,
            _ => false,
        };

        let mut value = first_ident.as_ref().to_owned();
        let mut serialize_quoted = value.contains(' ');

        // These keywords are not allowed by themselves.
        // The only way this value can be valid with with another keyword.
        if reserved {
            let ident = input.expect_ident()?;
            serialize_quoted = serialize_quoted || ident.contains(' ');
            value.push(' ');
            value.push_str(&ident);
        }

        while let Ok(ident) = input.try_parse(|i| i.expect_ident_cloned()) {
            serialize_quoted = serialize_quoted || ident.contains(' ');
            value.push(' ');
            value.push_str(&ident);
        }
        let syntax = if serialize_quoted {
            // For font family names which contains special white spaces, e.g.
            // `font-family: \ a\ \ b\ \ c\ ;`, it is tricky to serialize them
            // as identifiers correctly. Just mark them quoted so we don't need
            // to worry about them in serialization code.
            FontFamilyNameSyntax::Quoted
        } else {
            FontFamilyNameSyntax::Identifiers
        };
        Ok(SingleFontFamily::Named(NamedFamily {
            name: value,
            syntax,
        }))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontFamilyList(Box<[SingleFontFamily]>);

impl std::ops::Deref for FontFamilyList {
    type Target = [SingleFontFamily];

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl FontFamilyList {
    pub fn parse<'i, 't>(
        input: &mut cssparser::Parser<'i, 't>,
    ) -> Result<Self, crate::css::ParseError<'i>> {
        let families = input.parse_comma_separated(SingleFontFamily::parse)?;
        Ok(FontFamilyList(families.into_boxed_slice()))
    }
}

/// A percentage in the range 0.0..1.0.
#[derive(Default, Debug, Copy, Clone, PartialEq)]
pub struct Percentage(pub f32);

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq)]
pub struct Length(pub Au);

impl Length {
    pub fn is_zero(&self) -> bool {
        (self.0).0 == 0
    }

    pub fn to_f32_px(self) -> f32 {
        self.0.to_f32_px()
    }
}

impl std::fmt::Display for Length {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        self.to_f32_px().fmt(f)?;
        "px".fmt(f)
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct LengthPercentage {
    pub fixed: Length,
    pub percentage: Option<Percentage>,
}

impl LengthPercentage {
    pub fn is_zero(&self) -> bool {
        self.fixed.is_zero() && self.percentage.is_none()
    }
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

impl LengthPercentageOrAuto {
    pub fn is_auto(&self) -> bool {
        matches!(*self, Self::Auto)
    }

    pub fn is_zero(&self) -> bool {
        match *self {
            LengthPercentageOrAuto::LengthPercentage(ref lp) => lp.is_zero(),
            LengthPercentageOrAuto::Auto => false,
        }
    }

    pub fn is_zero_or_auto(&self) -> bool {
        self.is_auto() || self.is_zero()
    }
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
pub enum LineHeight {
    Normal,
    Number(f32),
    Length(LengthPercentage),
}

impl LineHeight {
    pub fn parse<'i>(
        input: &mut cssparser::Parser<'i, '_>,
    ) -> Result<Self, crate::css::ParseError<'i>> {
        if input.try_parse(|i| i.expect_ident_matching("normal")).is_ok() {
            return Ok(LineHeight::Normal);
        }
        if let Ok(number) = input.try_parse(|i| i.expect_number()) {
            return Ok(LineHeight::Number(number));
        }
        crate::css::parse_length_or_percentage(input).map(LineHeight::Length)
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

    pub white_space: WhiteSpace,

    pub font_size: Length,
    pub font_family: FontFamilyList,
    pub font_style: FontStyle,
    pub font_weight: FontWeight,
    pub line_height: LineHeight,
}

impl MutableComputedStyle {
    pub fn set_named_font_family(&mut self, name: impl Into<String>) {
        self.font_family = FontFamilyList(Box::new([
            SingleFontFamily::Named(NamedFamily {
                name: name.into(),
                syntax: FontFamilyNameSyntax::Quoted,
            })
        ]))
    }

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

            white_space: WhiteSpace::Normal,

            font_size: Length(Au::from_px(16)),
            font_family: FontFamilyList(Box::new([SingleFontFamily::Generic(GenericFamily::Serif)])),
            font_style: FontStyle::Normal,
            font_weight: FontWeight::Normal,
            line_height: LineHeight::Normal,
        }
    }

    pub fn inherited(&self) -> MutableComputedStyle {
        MutableComputedStyle {
            direction: self.direction,
            writing_mode: self.writing_mode,
            text_orientation: self.text_orientation,
            computed_writing_mode: self.computed_writing_mode,
            color: self.color,
            white_space: self.white_space,
            font_size: self.font_size,
            font_family: self.font_family.clone(),
            font_style: self.font_style,
            font_weight: self.font_weight,
            line_height: self.line_height.clone(),
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

    pub fn first_available_font_metrics(&self) -> crate::fonts::metrics::FontMetrics {
        crate::fonts::metrics::FontMetrics::from_style(self)
    }
}
