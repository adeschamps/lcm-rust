use Config;
use ast;
use itertools::Itertools;
use std::fmt::{self, Display, Formatter};

pub fn generate(module: &ast::Module) -> String {
    generate_with_config(module, &Config::default())
}

pub fn generate_with_config(module: &ast::Module, config: &Config) -> String {
    let mut buffer = String::new();
    {
        let mut generator = CodeGenerator::new(&mut buffer, config);
        generator.generate_module(module);
    }
    buffer
}

/// A wrapper around a String that keeps track of indentation.
///
/// To increase indentation, create a new instance of this type using
/// the `indent` method. To decrease indentation, let that instance go
/// out of scope.
struct CodeGenerator<'a> {
    buffer: &'a mut String,
    indent: usize,
    start: bool,
    config: &'a Config,
}

impl<'a> CodeGenerator<'a> {
    fn new(buffer: &'a mut String, config: &'a Config) -> CodeGenerator<'a> {
        CodeGenerator {
            buffer,
            indent: 0,
            start: true,
            config,
        }
    }

    /// Increase the indentation level.
    ///
    /// This returns another instance of `Self`. Since the new
    /// instance mutably borrows the existing one, the existing
    /// instance cannot be used until the new one goes out of scope.
    fn indent(&mut self) -> CodeGenerator {
        CodeGenerator {
            buffer: self.buffer,
            indent: self.indent + 1,
            start: true,
            config: self.config,
        }
    }

    /// Add a string without adding a newline.
    fn push(&mut self, s: &str) {
        if self.start {
            for _ in 0..self.indent {
                self.buffer.push_str("    ");
            }
            self.start = false;
        }
        self.buffer.push_str(s);
    }

    /// Add a string including a newline.
    fn push_line(&mut self, s: &str) {
        self.push(s);
        self.buffer.push('\n');
        self.start = true;
    }
}

impl<'a> CodeGenerator<'a> {
    fn generate_module(&mut self, module: &ast::Module) {
        for s in &module.structs {
            self.generate_struct(s);
        }
        for (name, submodule) in &module.submodules {
            self.push_line(&format!("pub mod {} {{", name.0));
            self.indent().generate_module(submodule);
            self.push_line("}");
        }
    }

    fn generate_struct(&mut self, s: &ast::Struct) {
        let struct_name = make_struct_name(&s.name);

        if let Some(ref comment) = s.comment {
            self.generate_comment(comment);
        }
        let mut derives = vec!["Clone", "Debug", "Message"];
        derives.extend(self.config.additional_traits.iter().map(|s| s.as_str()));
        derives.sort();
        let derives = derives.into_iter().join(", ");
        self.push_line(&format!("#[derive({})]", derives));
        self.push_line(&format!("pub struct {} {{", struct_name));
        for field in &s.fields {
            self.indent().generate_field(field);
        }
        self.push_line("}");

        if !s.constants.is_empty() {
            self.push_line(&format!("impl {} {{", struct_name));
            for constant in &s.constants {
                self.indent().generate_constant(constant);
            }
            self.push_line("}");
        }
    }

    fn generate_field(&mut self, field: &ast::Field) {
        if let Some(ref comment) = field.comment {
            self.generate_comment(comment);
        }
        if !field.multiplicity.is_empty() {
            let lengths = field
                .multiplicity
                .iter()
                .filter_map(|mult| match *mult {
                    ast::Multiplicity::Constant(_) => None,
                    ast::Multiplicity::Variable(ref len) => Some(format!("length = \"{}\"", len)),
                })
                .join(", ");
            self.push_line(&format!("#[lcm({})]", lengths));
        }
        self.push(&format!("pub {}: ", field.name));
        for multiplicity in &field.multiplicity {
            match *multiplicity {
                ast::Multiplicity::Constant(_) => {
                    self.push("[");
                }
                ast::Multiplicity::Variable(_) => {
                    self.push("Vec<");
                }
            }
        }
        self.push(&format!("{}", field.ty));
        for multiplicity in field.multiplicity.iter().rev() {
            match *multiplicity {
                ast::Multiplicity::Constant(len) => {
                    self.push(&format!("; {}]", len));
                }
                ast::Multiplicity::Variable(_) => {
                    self.push(">");
                }
            }
        }
        self.push_line(",");
    }

    fn generate_constant(&mut self, constant: &ast::Constant) {
        if let Some(ref comment) = constant.comment {
            self.generate_comment(comment);
        }
        self.push_line(&format!(
            "pub const {}: {} = {};",
            constant.name, constant.ty, constant.value
        ));
    }

    fn generate_comment(&mut self, comment: &ast::Comment) {
        self.push_line(&format!("#[doc = r#\"{}\"#]", comment.0));
    }
}

/// Convert a struct name to Rust naming conventions.
///
/// This converts to `CamelCase`, and also removes the trailing "_t"
/// that is common in C and LCM type names.
fn make_struct_name(original: &str) -> String {
    use heck::CamelCase;

    let original = if original.ends_with("_t") {
        &original[0..original.len() - 2]
    } else {
        original
    };
    original.to_camel_case()
}

impl Display for ast::Type {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match *self {
            ast::Type::Int8 => write!(f, "i8"),
            ast::Type::Int16 => write!{f, "i16"},
            ast::Type::Int32 => write!{f, "i32"},
            ast::Type::Int64 => write!{f, "i64"},
            ast::Type::Float => write!{f, "f32"},
            ast::Type::Double => write!{f, "f64"},
            ast::Type::String => write!{f, "String"},
            ast::Type::Boolean => write!{f, "bool"},
            ast::Type::Byte => write!{f, "u8"},
            ast::Type::Struct(ref namespaces, ref struct_name) => {
                for ns in namespaces {
                    write!(f, "{}::", ns.0)?;
                }
                write!(f, "{}", make_struct_name(struct_name))
            }
        }
    }
}
