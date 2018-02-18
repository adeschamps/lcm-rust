extern crate lcm_gen;
#[macro_use]
extern crate pretty_assertions;

use lcm_gen::{ast, codegen};
use std::collections::HashMap;

#[test]
fn simple_struct() {
    let module = ast::Module {
        submodules: HashMap::new(),
        structs: vec![
            ast::Struct {
                comment: None,
                name: "MyType".into(),
                fields: vec![
                    ast::Field {
                        comment: None,
                        name: "field".into(),
                        ty: ast::Type::Double,
                        multiplicity: vec![],
                    },
                ],
                constants: vec![],
            },
        ],
    };

    let generated = codegen::generate(&module);

    let expected = r#"#[derive(Debug, LcmMessage)]
pub struct MyType {
    pub field: f64,
}
"#;

    assert_eq!(generated, expected);
}

#[test]
fn temperature_t() {
    let generated = lcm_gen::Config::default()
        .generate_string(&["tests/data/temperature_t.lcm"])
        .unwrap();

    let expected = r#"#[derive(Debug, LcmMessage)]
pub struct temperature_t {
    pub utime: i64,
    #[doc = " Temperature in degrees Celsius. A "float" would probably
     * be good enough, unless we're measuring temperatures during
     * the big bang. Note that the asterisk on the beginning of this
     * line is not syntactically necessary, it's just pretty.
     "]
    pub degCelsius: f64,
}
"#;

    assert_eq!(generated, expected);
}

#[test]
fn point2d_list_t() {
    let generated = lcm_gen::Config::default()
        .generate_string(&["tests/data/point2d_list_t.lcm"])
        .unwrap();

    let expected = r#"#[derive(Debug, LcmMessage)]
pub struct point2d_list_t {
    pub npoints: i32,
    #[lcm(length = "npoints")]
    pub points: Vec<[f64; 2]>,
}
"#;

    assert_eq!(generated, expected);
}
