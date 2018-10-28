#[macro_use]
extern crate clap;
extern crate nglayoutng;

use nglayoutng::layout_tree::builder::LayoutTreeBuilder;
use std::fs::File;

fn main() {
    use clap::{AppSettings, SubCommand};

    let args = app_from_crate!()
        .subcommand(
            SubCommand::with_name("dump")
                .about("Dumps a layout tree from an HTML document")
                .arg_from_usage("<input>  'The document to build the tree for'")
        )
        .setting(AppSettings::ArgRequiredElseHelp)
        .get_matches();

    if let Some(args) = args.subcommand_matches("dump") {
        let input = args.value_of("input").unwrap();
        let mut file = File::open(input).expect("Couldn't open input file");

        let builder =
            LayoutTreeBuilder::new(&mut file).expect("Failed to parse input file?");

        let result = builder.build();
        println!("{:#?}", result);
    }
}
