//! This implements a very basic style engine without support for `!important`
//! other CSS rules that aren't style rules, or parsing specified values other
//! than the ones we need, which includes `calc(<length> + <percentage>)`.
//!
//! Also, it doesn't have any css-like error handling. Any syntax error reports
//! an error and stops parsing entirely.

use app_units::Au;
use cssparser::{self, CowRcStr, Parser, ParserInput, Token};
use smallvec::SmallVec;
use std::collections::HashMap;
use std::rc::Rc;
use style::{self, ComputedStyle, MutableComputedStyle};
use logical_geometry::WritingMode;

#[derive(PropertyDeclaration)]
pub enum PropertyDeclaration {
    #[declaration(early, field = "computed_writing_mode")]
    WritingMode(style::WritingMode),
    #[declaration(early)]
    Direction(style::Direction),
    #[declaration(early)]
    TextOrientation(style::TextOrientation),

    Width(style::LengthPercentageOrAuto),
    Height(style::LengthPercentageOrAuto),

    #[declaration(logical)]
    InlineSize(style::LengthPercentageOrAuto),
    #[declaration(logical)]
    BlockSize(style::LengthPercentageOrAuto),

    MarginTop(style::LengthPercentageOrAuto),
    MarginLeft(style::LengthPercentageOrAuto),
    MarginBottom(style::LengthPercentageOrAuto),
    MarginRight(style::LengthPercentageOrAuto),

    #[declaration(logical)]
    MarginBlockStart(style::LengthPercentageOrAuto),
    #[declaration(logical)]
    MarginBlockEnd(style::LengthPercentageOrAuto),
    #[declaration(logical)]
    MarginInlineStart(style::LengthPercentageOrAuto),
    #[declaration(logical)]
    MarginInlineEnd(style::LengthPercentageOrAuto),

    PaddingTop(style::LengthPercentage),
    PaddingLeft(style::LengthPercentage),
    PaddingBottom(style::LengthPercentage),
    PaddingRight(style::LengthPercentage),

    #[declaration(logical)]
    PaddingBlockStart(style::LengthPercentage),
    #[declaration(logical)]
    PaddingBlockEnd(style::LengthPercentage),
    #[declaration(logical)]
    PaddingInlineStart(style::LengthPercentage),
    #[declaration(logical)]
    PaddingInlineEnd(style::LengthPercentage),

    BorderTopWidth(style::LengthPercentage),
    BorderBottomWidth(style::LengthPercentage),
    BorderLeftWidth(style::LengthPercentage),
    BorderRightWidth(style::LengthPercentage),

    #[declaration(logical)]
    BorderBlockStartWidth(style::LengthPercentage),
    #[declaration(logical)]
    BorderBlockEndWidth(style::LengthPercentage),
    #[declaration(logical)]
    BorderInlineStartWidth(style::LengthPercentage),
    #[declaration(logical)]
    BorderInlineEndWidth(style::LengthPercentage),

    Display(style::Display),
    Position(style::Position),
    BoxSizing(style::BoxSizing),

    OverflowX(style::Overflow),
    OverflowY(style::Overflow),

    Float(style::Float),
    Clear(style::Clear),
}

pub struct CssStyleRule {
    selectors: kuchiki::Selectors,
    declarations: Vec<PropertyDeclaration>,
}

/// A rule with a single selector, used for sorting by specificity and source
/// order.
pub struct Rule {
    /// The index of the original selector in the rule.
    selector_index: usize,
    original_rule: Rc<CssStyleRule>,
    specificity: kuchiki::Specificity,
    source_order: usize,
}

#[derive(Debug)]
pub enum Error<'i> {
    InvalidSelector,
    UnknownPropertyName(CowRcStr<'i>),
    UnknownLengthUnit(CowRcStr<'i>),
}

pub type ParseError<'i> = cssparser::ParseError<'i, Error<'i>>;

struct CssParser;
impl<'i> cssparser::AtRuleParser<'i> for CssParser {
    type PreludeBlock = ();
    type PreludeNoBlock = ();
    type AtRule = CssStyleRule;
    type Error = Error<'i>;

    // Default methods reject everything.
}

impl<'i> cssparser::QualifiedRuleParser<'i> for CssParser {
    type Prelude = kuchiki::Selectors;
    type QualifiedRule = CssStyleRule;
    type Error = Error<'i>;

    #[inline]
    fn parse_prelude<'t>(
        &mut self,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::Prelude, ParseError<'i>> {
        let location = input.current_source_location();
        let position = input.position();
        while input.next().is_ok() {}
        kuchiki::Selectors::compile(input.slice_from(position))
            .map_err(|()| location.new_custom_error(Error::InvalidSelector))
    }

    #[inline]
    fn parse_block<'t>(
        &mut self,
        selectors: Self::Prelude,
        _location: cssparser::SourceLocation,
        input: &mut Parser<'i, 't>,
    ) -> Result<CssStyleRule, ParseError<'i>> {
        Ok(CssStyleRule {
            selectors,
            declarations: parse_declarations(input).map_err(|e| e.0)?,
        })
    }
}

fn length_from_dimension(
    unit: &str,
    value: f32,
) -> Result<style::Length, ()> {
    if !unit.eq_ignore_ascii_case("px") {
        return Err(());
    }
    Ok(style::Length(Au::from_f32_px(value)))
}

fn parse_length<'i>(input: &mut Parser<'i, '_>) -> Result<style::Length, ParseError<'i>> {
    let location = input.current_source_location();
    match *input.next()? {
        Token::Dimension { ref unit, value, .. } => {
            length_from_dimension(unit, value)
                .map_err(|()| location.new_custom_error(Error::UnknownLengthUnit(unit.clone())))
        }
        ref t => Err(location.new_unexpected_token_error(t.clone()))
    }
}

fn parse_length_or_percentage<'i>(input: &mut Parser<'i, '_>) -> Result<style::LengthPercentage, ParseError<'i>> {
    let location = input.current_source_location();
    match *input.next()? {
        Token::Dimension { ref unit, value, .. } => return Ok(style::LengthPercentage {
            fixed: length_from_dimension(unit, value)
                .map_err(|()| location.new_custom_error(Error::UnknownLengthUnit(unit.clone())))?,
            percentage: None,
        }),
        Token::Percentage { unit_value, .. } => return Ok(style::LengthPercentage {
            fixed: Default::default(),
            percentage: Some(style::Percentage(unit_value))
        }),
        Token::Function(ref name) if name.eq_ignore_ascii_case("calc") => {},
        ref t => return Err(location.new_unexpected_token_error(t.clone())),
    }
    input.parse_nested_block(|input| {
        let length = parse_length(input)?;
        let location = input.current_source_location();

        let sign = match *input.next()? {
            Token::Delim(c @ '-') |
            Token::Delim(c @ '+') => {
                if c == '+' { 1.0 } else { -1.0 }
            }
            ref t => return Err(location.new_unexpected_token_error(t.clone())),
        };

        let percentage = style::Percentage(sign * input.expect_percentage()?);

        Ok(style::LengthPercentage {
            fixed: length,
            percentage: Some(percentage),
        })
    })
}

fn parse_length_or_percentage_or_auto<'i>(input: &mut Parser<'i, '_>) -> Result<style::LengthPercentageOrAuto, ParseError<'i>> {
    if input.try(|i| i.expect_ident_matching("auto")).is_ok() {
        return Ok(style::LengthPercentageOrAuto::Auto);
    }
    Ok(style::LengthPercentageOrAuto::LengthPercentage(parse_length_or_percentage(input)?))
}

fn parse_overflow_shorthand<'i>(
    input: &mut Parser<'i, '_>,
) -> Result<SmallVec<[PropertyDeclaration; 1]>, ParseError<'i>> {
    let mut ret = SmallVec::new();
    let x = style::Overflow::parse(input)?;
    let y = input.try(|i| style::Overflow::parse(i)).unwrap_or(x);
    ret.push(PropertyDeclaration::OverflowX(x));
    ret.push(PropertyDeclaration::OverflowY(y));
    Ok(ret)
}

fn parse_four_sides<'i, L>(
    input: &mut Parser<'i, '_>,
    get_top: fn(L) -> PropertyDeclaration,
    get_right: fn(L) -> PropertyDeclaration,
    get_bottom: fn(L) -> PropertyDeclaration,
    get_left: fn(L) -> PropertyDeclaration,
    parse_one: fn(&mut Parser<'i, '_>) -> Result<L, ParseError<'i>>,
) -> Result<SmallVec<[PropertyDeclaration; 1]>, ParseError<'i>>
where
    L: Clone,
{
    let mut ret = SmallVec::new();
    let top = parse_one(input)?;
    let right = input.try(parse_one).ok();
    let bottom = input.try(parse_one).ok();
    let left = input.try(parse_one).ok();

    match (right, bottom, left) {
        (Some(right), Some(bottom), Some(left)) => {
            ret.push(get_top(top));
            ret.push(get_right(right));
            ret.push(get_bottom(bottom));
            ret.push(get_left(left));
        }
        (None, None, None) => {
            ret.push(get_top(top.clone()));
            ret.push(get_right(top.clone()));
            ret.push(get_bottom(top.clone()));
            ret.push(get_left(top));
        }
        (Some(right), None, None) => {
            ret.push(get_top(top.clone()));
            ret.push(get_right(right.clone()));
            ret.push(get_bottom(top));
            ret.push(get_left(right));
        }
        (Some(right), Some(bottom), None) => {
            ret.push(get_top(top));
            ret.push(get_right(right.clone()));
            ret.push(get_bottom(bottom));
            ret.push(get_left(right));
        }
        _ => unreachable!(),
    }

    assert_eq!(ret.len(), 4);
    Ok(ret)
}



struct PropertyDeclarationParser;
impl<'i> cssparser::DeclarationParser<'i> for PropertyDeclarationParser {
    type Declaration = SmallVec<[PropertyDeclaration; 1]>;
    type Error = Error<'i>;

    fn parse_value<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::Declaration, ParseError<'i>> {
        if let Ok(longhand) = PropertyDeclaration::parse_longhand(&name, input) {
            let mut declarations = SmallVec::new();
            declarations.push(longhand);
            return Ok(declarations)
        }

        match_ignore_ascii_case! { &name,
            "margin" => parse_four_sides(
                input,
                PropertyDeclaration::MarginTop,
                PropertyDeclaration::MarginRight,
                PropertyDeclaration::MarginBottom,
                PropertyDeclaration::MarginLeft,
                parse_length_or_percentage_or_auto,
            ),
            "padding" => parse_four_sides(
                input,
                PropertyDeclaration::PaddingTop,
                PropertyDeclaration::PaddingRight,
                PropertyDeclaration::PaddingBottom,
                PropertyDeclaration::PaddingLeft,
                parse_length_or_percentage,
            ),
            "border-width" => parse_four_sides(
                input,
                PropertyDeclaration::BorderTopWidth,
                PropertyDeclaration::BorderRightWidth,
                PropertyDeclaration::BorderBottomWidth,
                PropertyDeclaration::BorderLeftWidth,
                parse_length_or_percentage,
            ),
            "overflow" => parse_overflow_shorthand(input),
            _ => Err(input.new_custom_error(Error::UnknownPropertyName(name.clone()))),
        }
    }
}

impl<'i> cssparser::AtRuleParser<'i> for PropertyDeclarationParser {
    type PreludeBlock = ();
    type PreludeNoBlock = ();
    type AtRule = SmallVec<[PropertyDeclaration; 1]>;
    type Error = Error<'i>;
}

pub fn parse_declarations<'i>(input: &mut Parser<'i, '_>) -> Result<Vec<PropertyDeclaration>, (ParseError<'i>, &'i str)> {
    let mut declarations = Vec::new();
    let iter = cssparser::DeclarationListParser::new(input, PropertyDeclarationParser);
    for declaration_list in iter {
        let declaration_list = match declaration_list {
            Ok(l) => l,
            Err(e) => {
                eprintln!("CSS declaration dropped: {:?}", e);
                continue;
            }
        };
        for declaration in declaration_list {
            declarations.push(declaration);
        }
    }
    Ok(declarations)
}

pub fn parse_css<'i>(css: &'i str) -> Vec<Rule> {
    let mut input = ParserInput::new(css);
    let mut input = Parser::new(&mut input);

    let iter = cssparser::RuleListParser::new_for_stylesheet(&mut input, CssParser);
    let mut css_rules = Vec::new();

    for result in iter {
        let rule = match result {
            Ok(r) => r,
            Err((error, string)) => {
                eprintln!("Rule dropped: {:?}, {:?}", error, string);
                continue;
            }
        };
        css_rules.push(Rc::new(rule));
    }

    // Now sort each selector by (specificity, source_order).
    let mut rules = Vec::new();

    for (source_order, rule) in css_rules.into_iter().enumerate() {
        for (selector_index, selector) in rule.selectors.0.iter().enumerate() {
            rules.push(Rule {
                selector_index,
                original_rule: rule.clone(),
                specificity: selector.specificity(),
                source_order,
            });
        }
    }

    rules.sort_by_key(|rule| (rule.specificity, rule.source_order));

    rules
}

/// A map with styles from each element to its style.
pub type StyleMap = HashMap<*const kuchiki::Node, ComputedStyle>;

pub fn compute_styles(root: &kuchiki::NodeRef, rules: &[Rule]) -> StyleMap {
    let mut map = Default::default();
    compute_styles_for_tree(root, rules, None, &mut map);
    map
}

fn apply_declaration(style: &mut MutableComputedStyle, declaration: &PropertyDeclaration) {
    declaration.compute(style);
}

fn compute_element_style(
    matching_declaration_blocks: &[&Vec<PropertyDeclaration>],
    inherited_style: Option<&ComputedStyle>,
) -> ComputedStyle {
    let mut style = match inherited_style {
        Some(s) => s.inherited(),
        None => ComputedStyle::initial(),
    };

    // Apply early properties first.
    for block in matching_declaration_blocks {
        for declaration in &**block {
            if declaration.is_early() {
                apply_declaration(&mut style, declaration);
            }
        }
    }

    // Now compute the writing mode, on which late properties may depend on.
    style.writing_mode = WritingMode::new(
        style.direction,
        style.computed_writing_mode,
        style.text_orientation,
    );

    // Now apply the late properties.
    for block in matching_declaration_blocks {
        for declaration in &**block {
            if !declaration.is_early() {
                apply_declaration(&mut style, declaration);
            }
        }
    }

    // Done!
    style.finish(inherited_style.is_none())
}

fn compute_styles_for_tree(
    node: &kuchiki::NodeRef,
    rules: &[Rule],
    inherited_style: Option<&ComputedStyle>,
    map: &mut StyleMap,
) {
    let element = match node.clone().into_element_ref() {
        Some(e) => e,
        None => {
            for child in node.children() {
                compute_styles_for_tree(&child, rules, inherited_style, map);
            }
            return;
        }
    };

    let mut matching_declaration_blocks = Vec::new();

    for rule in rules {
        if rule.original_rule.selectors.0[rule.selector_index].matches(&element) {
            matching_declaration_blocks.push(&rule.original_rule.declarations);
        }
    }

    let style_attr = element.attributes.borrow().get("style").and_then(|style_attr| {
        let mut input = ParserInput::new(style_attr);
        let mut input = Parser::new(&mut input);
        parse_declarations(&mut input).ok()
    });

    if let Some(ref s) = style_attr {
        matching_declaration_blocks.push(s);
    }

    let style = compute_element_style(&matching_declaration_blocks, inherited_style);

    for child in node.children() {
        compute_styles_for_tree(&child, rules, Some(&style), map);
    }

    map.insert(&*node.0, style);
}
