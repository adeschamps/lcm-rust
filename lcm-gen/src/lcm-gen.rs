#[macro_use]
extern crate failure;
extern crate lcm_gen;
extern crate structopt;

use failure::Error;
use std::path::PathBuf;
use structopt::StructOpt;

/// LCM code generator for Rust.
///
/// Note that the lcm-gen crate can also be added as a build
/// dependency to your project, to generate code at build time.
#[derive(Debug, StructOpt)]
#[structopt(raw(setting = "structopt::clap::AppSettings::ColoredHelp"))]
struct Options {
    #[structopt(long = "package-prefix",
                help = "Add this package name as a prefix to the declared package.")]
    package_prefix: Option<String>,

    #[structopt(long = "out", short = "o", parse(from_os_str),
                help = "The file to write the generated code to.", default_value = "mod.rs")]
    output_file: PathBuf,

    #[structopt(long = "derive", short = "d", raw(number_of_values = "1"),
                raw(multiple = "true"), help = "Additional traits to derive.")]
    custom_derives: Vec<String>,

    #[structopt(parse(from_os_str), raw(required = "true"), help = "A list of .lcm files.")]
    input_files: Vec<PathBuf>,
}

fn main() {
    match run() {
        Ok(()) => {}
        Err(e) => {
            println!("Error: {}", e);
            for cause in e.iter_chain().skip(1) {
                println!("Caused by: {}", cause);
            }
            println!("Backtrace:\n{}", e.backtrace());
        }
    }
}

fn run() -> Result<(), Error> {
    let options = Options::from_args();

    ensure!(
        !options.input_files.is_empty(),
        "No input files were specified."
    );

    let mut config = lcm_gen::Config {
        package_prefix: options.package_prefix,
        output_file: Some(options.output_file),
        additional_traits: options.custom_derives,
    };
    config.generate(&options.input_files)
}
