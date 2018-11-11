extern crate nglayoutng;
extern crate diff;

use std::io::{Cursor, Write};
use std::fs::{self, File};
use std::path::Path;
use nglayoutng::layout_tree::builder::{LayoutTreeBuilder, LayoutTreeBuilderResult};
use nglayoutng::dom;

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
    let layout_expectations = expectations.join(format!("{}.layout.txt", test_name));

    let expected_dom = fs::read_to_string(&dom_expectations).unwrap_or_default();
    let expected_layout = fs::read_to_string(&layout_expectations).unwrap_or_default();

    let dom = {
        let mut dom = Cursor::new(Vec::new());
        dom::print_dom_to(&result.dom, &mut dom);
        String::from_utf8(dom.into_inner()).unwrap()
    };
    let layout = {
        let mut layout = Cursor::new(Vec::new());
        result.layout_tree.print_to(&mut layout);
        String::from_utf8(layout.into_inner()).unwrap()
    };

    if dom == expected_dom && layout == expected_layout {
        return;
    }

    // Override the expectations.
    File::create(&dom_expectations).unwrap().write_all(dom.as_bytes()).unwrap();
    File::create(&layout_expectations).unwrap().write_all(layout.as_bytes()).unwrap();

    print_diff(&dom, &expected_dom, "DOM differed");
    print_diff(&layout, &expected_layout, "DOM differed");

    panic!("Expectation and test mismatch!");
}

macro_rules! test_doc {
    ($function:ident, $html_file:expr, $expectations_directory:expr) => (
        #[test]
        fn $function() {
            let mut header = File::open($html_file).unwrap();
            let builder = LayoutTreeBuilder::new(&mut header)
                .expect("Failed to parse input file?");

            compare_with_reference(
                $html_file,
                $expectations_directory,
                builder.build(),
            );
        }
    )
}

include!(concat!(env!("OUT_DIR"), "/tests.rs"));
