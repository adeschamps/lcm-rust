extern crate assert_cli;
#[macro_use]
extern crate pretty_assertions;
extern crate tempdir;

use std::fs::File;
use std::io::Read;
use tempdir::TempDir;

#[test]
fn creates_output() {
    let dir = TempDir::new("lcm-gen").unwrap();
    let output = dir.path().join("mod.rs");
    let output = output.to_str().unwrap();
    let input = "tests/data/temperature_t.lcm";

    assert_cli::Assert::command(&["../target/debug/lcm-gen-rust", "--out", output, input])
        .stdout()
        .is("")
        .unwrap();

    let mut output = File::open(output).unwrap();
    let mut generated = String::new();
    output.read_to_string(&mut generated).unwrap();
    assert!(generated.contains("struct Temperature"));
}

#[test]
fn package_prefix() {
    let dir = TempDir::new("lcm-gen").unwrap();
    let output = dir.path().join("mod.rs");
    let output = output.to_str().unwrap();
    let input = "tests/data/temperature_t.lcm";

    assert_cli::Assert::command(&[
        "../target/debug/lcm-gen-rust",
        "--out",
        output,
        "--package-prefix",
        "foo.bar",
        input,
    ]).stdout()
        .is("")
        .unwrap();

    let mut output = File::open(output).unwrap();
    let mut generated = String::new();
    output.read_to_string(&mut generated).unwrap();
    assert!(generated.contains("mod foo"));
    assert!(generated.contains("mod bar"));

    let mod_lines:Vec<_> = generated.lines().filter(|l| l.contains("mod")).collect();
    assert_eq!(mod_lines, ["pub mod foo {", "    pub mod bar {"]);
}

#[test]
fn custom_derives() {
    let dir = TempDir::new("lcm-gen").unwrap();
    let output = dir.path().join("mod.rs");
    let output = output.to_str().unwrap();
    let input = "tests/data/temperature_t.lcm";

    assert_cli::Assert::command(&[
        "../target/debug/lcm-gen-rust",
        "--out",
        output,
        "--derive",
        "Serialize",
        "--derive",
        "Deserialize",
        input,
    ]).stdout()
        .is("")
        .unwrap();

    let mut output = File::open(output).unwrap();
    let mut generated = String::new();
    output.read_to_string(&mut generated).unwrap();
    let derives = generated
        .lines()
        .find(|l| l.contains("derive"))
        .expect("no derives found");
    assert_eq!(
        derives,
        "#[derive(Clone, Debug, Deserialize, Message, Serialize)]"
    );
}
