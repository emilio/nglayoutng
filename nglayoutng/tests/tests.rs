extern crate diff;
extern crate nglayoutng;

use app_units::Au;
use nglayoutng::dom;
use nglayoutng::layout_tree::builder::{LayoutTreeBuilder, LayoutTreeBuilderResult};
use nglayoutng::layout_tree::PrintId;
use std::fs::{self, File};
use std::io::{Cursor, Write};
use std::path::Path;

fn print_diff(actual: &str, expected: &str, label: &str) {
    if actual == expected {
        return;
    }

    println!("{}", label);
    println!("diff expected generated");
    for diff in diff::lines(&expected, &actual) {
        match diff {
            diff::Result::Left(l) => println!("-{}", l),
            diff::Result::Both(l, _) => println!(" {}", l),
            diff::Result::Right(r) => println!("+{}", r),
        }
    }
}

fn compare_with_reference(
    test_path: &str,
    expectations_directory: &str,
    result: LayoutTreeBuilderResult,
) {
    let test_name = Path::new(test_path).file_name().unwrap().to_str().unwrap();
    let expectations = Path::new(expectations_directory);
    let dom_expectations = expectations.join(format!("{}.dom.txt", test_name));
    let layout_tree_expectations = expectations.join(format!("{}.layout-tree.txt", test_name));
    let fragment_tree_expectations = expectations.join(format!("{}.fragment-tree.txt", test_name));

    let expected_dom = fs::read_to_string(&dom_expectations).unwrap_or_default();
    let expected_layout_tree = fs::read_to_string(&layout_tree_expectations).unwrap_or_default();
    let expected_fragment_tree = fs::read_to_string(&fragment_tree_expectations).unwrap_or_default();

    let dom = {
        let mut dom = Cursor::new(Vec::new());
        dom::print_dom_to(&result.dom, &mut dom);
        String::from_utf8(dom.into_inner()).unwrap()
    };
    let layout_tree = {
        let mut layout = Cursor::new(Vec::new());
        result.layout_tree.print_to(&mut layout, PrintId::No);
        String::from_utf8(layout.into_inner()).unwrap()
    };

    // Layout is expected to panic in a bunch of cases for now.
    //
    // TODO(emilio): Remove catch_unwind when stuff is more stable..
    let fragment_tree = {
        let quirks_mode = result.dom.as_document().unwrap().quirks_mode();
        let viewport = euclid::Size2D::new(Au::from_f32_px(800.0), Au::from_f32_px(600.0));
        let layout_tree = &result.layout_tree;
        std::panic::catch_unwind(|| {
            let tree = layout_tree.layout(quirks_mode, viewport);
            format!("{:#?}", tree)
        }).unwrap_or_default()
    };

    if dom == expected_dom && layout_tree == expected_layout_tree && fragment_tree == expected_fragment_tree {
        return;
    }

    // Override the expectations.
    File::create(&dom_expectations)
        .unwrap()
        .write_all(dom.as_bytes())
        .unwrap();
    File::create(&layout_tree_expectations)
        .unwrap()
        .write_all(layout_tree.as_bytes())
        .unwrap();
    File::create(&fragment_tree_expectations)
        .unwrap()
        .write_all(fragment_tree.as_bytes())
        .unwrap();

    print_diff(&dom, &expected_dom, "DOM differed");
    print_diff(&layout_tree, &expected_layout_tree, "Layout tree differed");
    print_diff(&fragment_tree, &expected_fragment_tree, "Fragment tree differed");

    panic!("Expectation and test mismatch!");
}

macro_rules! test_doc {
    ($function:ident, $html_file:expr, $expectations_directory:expr) => {
        #[test]
        fn $function() {
            let mut header = File::open($html_file).unwrap();
            let builder = LayoutTreeBuilder::new(&mut header).expect("Failed to parse input file?");

            compare_with_reference($html_file, $expectations_directory, builder.build());
        }
    };
}

include!(concat!(env!("OUT_DIR"), "/tests.rs"));
