#[macro_use]
extern crate clap;
extern crate nglayoutng;

use nglayoutng::dom::print_dom;
use nglayoutng::layout_tree::builder::LayoutTreeBuilder;
use std::fs::File;

enum DumpKind {
    Layout,
    Dom,
}

fn main() {
    use clap::{AppSettings, SubCommand};

    let args = app_from_crate!()
        .subcommand(
            SubCommand::with_name("layout")
                .about("Dumps a layout tree from an HTML document")
                .arg_from_usage("<input>  'The document to build the tree for'"),
        )
        .subcommand(
            SubCommand::with_name("dom")
                .about("Dumps a DOM tree from an HTML document")
                .arg_from_usage("<input>  'The document to build the tree for'"),
        )
        .setting(AppSettings::ArgRequiredElseHelp)
        .get_matches();
    let (input, kind) = {
        if let Some(args) = args.subcommand_matches("layout") {
            let input = args.value_of("input").unwrap();
            (input, DumpKind::Layout)
        } else if let Some(args) = args.subcommand_matches("dom") {
            let input = args.value_of("input").unwrap();
            (input, DumpKind::Dom)
        } else {
            panic!("Unknown subcommand, {:?}", args);
        }
    };

    let mut file = File::open(input).expect("Couldn't open input file");

    let builder = LayoutTreeBuilder::new(&mut file).expect("Failed to parse input file?");

    let result = builder.build();
    match kind {
        DumpKind::Layout => result.layout_tree.print(),
        DumpKind::Dom => print_dom(&result.dom),
    }
}
