use std::collections::HashMap;

#[derive(Default)]
pub struct Module {
    pub submodules: HashMap<Namespace, Module>,
    pub structs: Vec<Struct>,
}

#[derive(Debug, PartialEq)]
pub struct File {
    pub namespaces: Vec<Namespace>,
    pub structs: Vec<Struct>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct Namespace(pub String);

#[derive(Debug, PartialEq)]
pub struct Struct {
    pub comment: Option<Comment>,
    pub name: String,
    pub fields: Vec<Field>,
    pub constants: Vec<Constant>,
}

#[derive(Debug, PartialEq)]
pub struct Field {
    pub comment: Option<Comment>,
    pub name: String,
    pub ty: Type,
    pub multiplicity: Vec<Multiplicity>,
}

#[derive(Debug, PartialEq)]
pub enum Multiplicity {
    Constant(usize),
    Variable(String),
}

#[derive(Debug, PartialEq)]
pub struct Constant {
    pub comment: Option<Comment>,
    pub name: String,
    pub ty: Type,
    pub value: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Type {
    Int8,
    Int16,
    Int32,
    Int64,
    Float,
    Double,
    String,
    Boolean,
    Byte,
    Struct(Vec<Namespace>, String),
}

#[derive(Debug, PartialEq)]
pub struct Comment(pub String);

impl Module {
    /// Insert a struct into either this module or the appropriate
    /// submodule.
    ///
    /// An effect of this method is that any struct created in a leaf
    /// module will cause all of its parent modules to be implicitly
    /// created.
    pub fn add_struct(&mut self, path: &[Namespace], s: Struct) {
        match path.first() {
            None => {
                self.structs.push(s);
            }
            Some(namespace) => {
                self.submodules
                    .entry(namespace.clone())
                    .or_insert_with(Default::default)
                    .add_struct(&path[1..], s);
            }
        }
    }
}

impl File {
    pub fn add_package_prefix(&mut self, prefix: &str) {
        self.namespaces
            .splice(0..0, prefix.split('.').map(|ns| Namespace(ns.into())));
    }
}

#[test]
fn add_package_prefix() {
    let mut file = File {
        namespaces: vec![Namespace("ns".into())],
        structs: vec![],
    };
    file.add_package_prefix("one.two");
    assert_eq!(
        file.namespaces,
        vec![
            Namespace("one".into()),
            Namespace("two".into()),
            Namespace("ns".into()),
        ]
    );
}
