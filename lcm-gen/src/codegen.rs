use ast;
use itertools::Itertools;
use std::fmt::{self, Display, Formatter};

pub fn generate(module: &ast::Module) -> String {
    let mut buffer = String::new();
    {
        let mut generator = CodeGenerator::new(&mut buffer);
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
}

impl<'a> CodeGenerator<'a> {
    fn new(buffer: &mut String) -> CodeGenerator {
        CodeGenerator {
            buffer,
            indent: 0,
            start: true,
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
        if let Some(ref comment) = s.comment {
            self.generate_comment(comment);
        }
        self.push_line("#[derive(Debug, Message)]");
        self.push_line(&format!("pub struct {} {{", s.name));
        for field in &s.fields {
            self.indent().generate_field(field);
        }
        self.push_line("}");

        if !s.constants.is_empty() {
            self.push_line(&format!("impl {} {{", s.name));
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
                    ast::Multiplicity::Variable(ref len) => Some(format!("length = \"{}\"", len))
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
            "const {}: {} = {};",
            constant.name, constant.ty, constant.value
        ));
    }

    fn generate_comment(&mut self, comment: &ast::Comment) {
        self.push_line(&format!("#[doc = \"{}\"]", comment.0));
    }
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
                write!(f, "{}", struct_name)
            }
        }
    }
}
