//! These are little helpers that will help to get the layout tree builder up
//! and running.

use html5ever::LocalName;
use kuchiki::{self, NodeData, NodeRef};
use kuchiki::traits::*;
use std::io::{self, Read};

/// Parses a DOM tree using html5ever and returns the root.
pub fn build_dom<R>(input: &mut R) -> io::Result<NodeRef>
where
    R: Read,
{
    kuchiki::parse_html()
        .from_utf8()
        .read_from(input)
}

/// Reads all the style sheets in the DOM and returns a CSS string with the
/// union of them in document order.
pub fn read_stylesheets(root: &NodeRef) -> String {
    let mut css = String::new();
    read_stylesheets_from(&root, &mut css, /* in_sheet = */ false);
    css
}

fn read_stylesheets_from(
    node: &NodeRef,
    css: &mut String,
    mut in_sheet: bool,
) {
    match node.data() {
        NodeData::Document(..) |
        NodeData::DocumentFragment |
        NodeData::Comment(..) |
        NodeData::ProcessingInstruction(..) |
        NodeData::Doctype(..) => {},
        NodeData::Text(ref text) => {
            if in_sheet {
                css.push_str(&text.borrow());
            }
        }
        NodeData::Element(ref element) => {
            in_sheet = element.name.local == LocalName::from("style");
        }
    }

    for child in node.children() {
        read_stylesheets_from(&child, css, in_sheet)
    }
}
