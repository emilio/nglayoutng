use std::env;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

fn generate_tests() {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let mut dst = File::create(Path::new(&out_dir).join("tests.rs")).unwrap();

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let html_dir = manifest_dir.join("tests").join("html");
    let html = fs::read_dir(html_dir).unwrap();

    let expectations_path = manifest_dir.join("tests").join("expectations");

    println!("cargo:rerun-if-changed=tests/html");

    for entry in html {
        let entry = entry.unwrap();
        assert_eq!(entry.path().extension().unwrap().to_str().unwrap(), "html");

        let func = entry
            .file_name()
            .to_str()
            .unwrap()
            .replace(|c: char| !c.is_alphanumeric(), "_")
            .to_lowercase();
        writeln!(
            dst,
            "test_doc!(header_{}, {:?}, {:?});",
            func,
            entry.path(),
            expectations_path,
        )
        .unwrap();
    }
}

fn main() {
    generate_tests();
}
