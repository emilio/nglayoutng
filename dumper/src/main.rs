#[macro_use]
extern crate clap;

use app_units::Au;
use nglayoutng::dom::print_dom;
use nglayoutng::layout_tree::builder::LayoutTreeBuilder;
use std::fs::File;

enum DumpKind {
    Layout,
    LayoutTree,
    Dom,
}

fn main() {
    use clap::{AppSettings, SubCommand};

    env_logger::init();

    let args = app_from_crate!()
        .subcommand(
            SubCommand::with_name("layout")
                .about("Dumps a fragment tree from an HTML document")
                .arg_from_usage("<input>  'The document to build the tree for'"),
        )
        .subcommand(
            SubCommand::with_name("layout-tree")
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
        } else if let Some(args) = args.subcommand_matches("layout-tree") {
            let input = args.value_of("input").unwrap();
            (input, DumpKind::LayoutTree)
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
    result.layout_tree.assert_consistent();
    match kind {
        DumpKind::Layout => {
            let result = result.layout_tree.layout(result.dom.as_document().unwrap().quirks_mode(), euclid::Size2D::new(Au::from_f32_px(800.0), Au::from_f32_px(600.0)));
            println!("{:?}", result.fragment);
        },
        DumpKind::LayoutTree => result.layout_tree.print(),
        DumpKind::Dom => print_dom(&result.dom),
    }
}
