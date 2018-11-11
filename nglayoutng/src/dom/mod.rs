//! These are little helpers that will help to get the layout tree builder up
//! and running.

use html5ever::LocalName;
use kuchiki::{self, NodeData, NodeRef};
use kuchiki::traits::*;
use misc::print_tree::PrintTree;
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

fn print_node(node: &NodeRef, print: &mut PrintTree) {
    print.new_level(match node.data() {
        NodeData::Document(..) => "#document".into(),
        NodeData::DocumentFragment => "#document-fragment".into(),
        NodeData::Comment(ref comment) => format!("<!-- {} -->", comment.borrow()),
        NodeData::ProcessingInstruction(ref content) => {
            let content = content.borrow();
            format!("<?{} {}?>", content.0, content.1)
        },
        NodeData::Doctype(ref doctype) => {
            format!("<!DOCTYPE {} {} {}>", doctype.name, doctype.public_id, doctype.system_id)
        },
        NodeData::Text(ref text) => {
            format!("#text {:?}", text.borrow())
        },
        NodeData::Element(ref element) => {
            format!("<{}>", element.name.local)
        }
    });

    for child in node.children() {
        print_node(&child, print);
    }

    print.end_level();
}

/// Prints the dom to stderr.
pub fn print_dom(root: &NodeRef) {
    print_dom_to(root, &mut std::io::stdout());
}

/// Prints the dom to a particular output.
pub fn print_dom_to(root: &NodeRef, dest: &mut ::std::io::Write) {
    let mut tree = PrintTree::new("DOM tree", dest);
    print_node(root, &mut tree);
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
