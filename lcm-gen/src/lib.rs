#[macro_use]
extern crate failure;
extern crate heck;
extern crate itertools;
extern crate pest;
#[macro_use]
extern crate pest_derive;

use failure::{Error, ResultExt};
use std::fmt::Debug;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

pub mod ast;
pub mod codegen;
pub mod parser;

/// Generate Rust types from the given LCM schemas using the default
/// configuration.
///
/// By default, the output is written to `$OUT_DIR/mod.rs`. Include it
/// in your code like this:
///
/// ```ignore
/// include!(concat!(env!("OUT_DIR"), "/mod.rs"));
/// ```
///
/// If you need more control, use the [`Config`] struct to customize
/// code generation.
///
/// [`Config`]: struct.Config.html
pub fn generate<P: AsRef<Path> + Debug>(lcm_files: &[P]) -> Result<(), Error> {
    Config::default().generate(lcm_files)
}

/// Configuration for code generation.
pub struct Config {
    pub package_prefix: Option<String>,
    pub output_file: Option<PathBuf>,
    pub additional_traits: Vec<String>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            package_prefix: None,
            output_file: None,
            additional_traits: vec![],
        }
    }
}

impl Config {
    /// Generate Rust types from the given LCM schemas and write the
    /// results to a file.
    ///
    /// By default, the output is written to $OUT_DIR/mod.rs. Include
    /// it in your code like this:
    ///
    /// ```ignore
    /// include!(concat!(env!("OUT_DIR"), "/mod.rs"));
    /// ```
    pub fn generate<P: AsRef<Path> + Debug>(&mut self, lcm_files: &[P]) -> Result<(), Error> {
        let output = self.generate_string(lcm_files)?;

        let output_file = self.output_file
            .clone()
            .unwrap_or_else(|| PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("mod.rs"));
        let mut output_file =
            File::create(&output_file).context(format_err!("Opening {:?}", output_file))?;
        write!(output_file, "{}", output).context("Writing output")?;

        Ok(())
    }

    /// Generate Rust types from the given LCM schemas, and return the
    /// generated code a String.
    ///
    /// This is hidden from the docs because you should normally just
    /// call `generate`. This method exposes the intermediate result
    /// as a convenience for testing.
    #[doc(hidden)]
    pub fn generate_string<P: AsRef<Path> + Debug>(
        &mut self,
        lcm_files: &[P],
    ) -> Result<String, Error> {
        let mut root_module = ast::Module::default();

        for path in lcm_files {
            let mut file = File::open(&path).context(format_err!("Opening file {:?}", path))?;
            let mut buffer = String::new();
            file.read_to_string(&mut buffer)?;

            let mut lcm_file: ast::File =
                parser::parse_file(&buffer).context(format_err!("Parsing file {:?}", path))?;

            if let Some(ref prefix) = self.package_prefix {
                lcm_file.add_package_prefix(prefix);
            }

            for s in lcm_file.structs {
                root_module.add_struct(&lcm_file.namespaces, s);
            }
        }

        Ok(codegen::generate_with_config(&root_module, self))
    }
}
