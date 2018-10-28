mod dom;
mod css;

use kuchiki;

use std::io::{self, Read};

/// The LayoutTreeBuilder gets a DOM and style tree and outputs a LayoutTree.
///
/// For now it doesn't care at all about any dynamic change or anything, and
/// assumes the HTML doesn't contain stuff that we don't support, like IB splits
/// or what not.
pub struct LayoutTreeBuilder {
    dom: kuchiki::NodeRef,
    styles: css::StyleMap,
}

impl LayoutTreeBuilder {
    pub fn new(input: &mut impl Read) -> io::Result<Self> {
        let dom = dom::build_dom(input)?;
        let css = dom::read_stylesheets(&dom);
        let style_rules = css::parse_css(&css).expect("Invalid CSS");
        let styles = css::compute_styles(&dom, &style_rules);
        Ok(Self {
            dom,
            styles,
        })
    }
}
