//! This implements a very basic style engine without support for `!important`
//! other CSS rules that aren't style rules, or parsing specified values other
//! than the ones we need, which includes `calc(<length> + <percentage>)`.
//!
//! Also, it doesn't have any css-like error handling. Any syntax error reports
//! an error and stops parsing entirely.

use app_units::Au;
use cssparser::{self, CowRcStr, Parser, ParserInput, Token};
use std::collections::HashMap;
use std::rc::Rc;
use style::{self, ComputedStyle, MutableComputedStyle};

pub enum PropertyDeclaration {
    Width(style::LengthPercentageOrAuto),
    Height(style::LengthPercentageOrAuto),
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
type ParseError<'i> = cssparser::ParseError<'i, Error<'i>>;

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

struct PropertyDeclarationParser;
impl<'i> cssparser::DeclarationParser<'i> for PropertyDeclarationParser {
    type Declaration = PropertyDeclaration;
    type Error = Error<'i>;

    fn parse_value<'t>(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i, 't>,
    ) -> Result<Self::Declaration, ParseError<'i>> {
        Ok(match_ignore_ascii_case! { &name,
            "height" => PropertyDeclaration::Height(parse_length_or_percentage_or_auto(input)?),
            "width" => PropertyDeclaration::Width(parse_length_or_percentage_or_auto(input)?),
            _ => return Err(input.new_custom_error(Error::UnknownPropertyName(name.clone()))),
        })
    }
}

impl<'i> cssparser::AtRuleParser<'i> for PropertyDeclarationParser {
    type PreludeBlock = ();
    type PreludeNoBlock = ();
    type AtRule = PropertyDeclaration;
    type Error = Error<'i>;
}

pub fn parse_declarations<'i>(input: &mut Parser<'i, '_>) -> Result<Vec<PropertyDeclaration>, (ParseError<'i>, &'i str)> {
    let mut declarations = Vec::new();
    let iter = cssparser::DeclarationListParser::new(input, PropertyDeclarationParser);
    for declaration in iter {
        declarations.push(declaration?);
    }
    Ok(declarations)
}

pub fn parse_css<'i>(css: &'i str) -> Result<Vec<Rule>, (ParseError<'i>, &'i str)> {
    let mut input = ParserInput::new(css);
    let mut input = Parser::new(&mut input);

    let iter = cssparser::RuleListParser::new_for_stylesheet(&mut input, CssParser);
    let mut css_rules = Vec::new();

    for result in iter {
        css_rules.push(Rc::new(result?));
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

    Ok(rules)
}

/// A map with styles from each element to its style.
pub type StyleMap = HashMap<*const kuchiki::Node, ComputedStyle>;

pub fn compute_styles(root: &kuchiki::NodeRef, rules: &[Rule]) -> StyleMap {
    let mut map = Default::default();
    compute_styles_for_tree(root, rules, None, &mut map);
    map
}

fn apply_declaration(style: &mut MutableComputedStyle, declaration: &PropertyDeclaration) {
    match *declaration {
        PropertyDeclaration::Height(val) => style.height = val,
        PropertyDeclaration::Width(val) => style.width = val,
    }
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

    let mut style = match inherited_style {
        Some(s) => s.inherited(),
        None => ComputedStyle::initial(),
    };

    for rule in rules {
        if rule.original_rule.selectors.0[rule.selector_index].matches(&element) {
            for declaration in &rule.original_rule.declarations {
                apply_declaration(&mut style, declaration);
            }
        }
    }

    if let Some(style_attr) = element.attributes.borrow().get("style") {
        let mut input = ParserInput::new(style_attr);
        let mut input = Parser::new(&mut input);
        if let Ok(declarations) = parse_declarations(&mut input) {
            for ref declaration in declarations {
                apply_declaration(&mut style, declaration);
            }
        }
    }

    let style = style.finish(inherited_style.is_none());
    for child in node.children() {
        compute_styles_for_tree(&child, rules, Some(&style), map);
    }

    map.insert(&*node.0, style);
}
