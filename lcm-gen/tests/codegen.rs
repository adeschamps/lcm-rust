extern crate lcm_gen;
#[macro_use]
extern crate pretty_assertions;

use lcm_gen::{ast, codegen, Config};
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

    let expected = r#"#[derive(Clone, Debug, Message)]
pub struct MyType {
    pub field: f64,
}
"#;

    assert_eq!(generated, expected);
}

macro_rules! check_generated {
    ( $lcm_type:ident, $expected:expr ) => {
        #[test]
        fn $lcm_type() {
            let generated = lcm_gen::Config::default()
                .generate_string(&[concat!("tests/data/", stringify!($lcm_type), ".lcm")])
                .unwrap();

            assert_eq!(generated, $expected);
        }
    }
}

check_generated!(
    camera_image_t,
    r#"pub mod mycorp {
    #[derive(Clone, Debug, Message)]
    pub struct CameraImage {
        pub utime: i64,
        pub camera_name: String,
        pub jpeg_image: jpeg::Image,
        pub pose: mit::Pose,
    }
}
"#
);

check_generated!(
    comments_t,
    r##"#[doc = r#" This is a comment
 that spans multiple lines"#]
#[derive(Clone, Debug, Message)]
pub struct MyStruct {
    #[doc = r#" Horizontal position in meters."#]
    pub x: i32,
    #[doc = r#" Vertical position in meters."#]
    pub y: i32,
}
"##
);

check_generated!(
    multiple_structs,
    r#"#[derive(Clone, Debug, Message)]
pub struct A {
    pub b: B,
    pub c: C,
}
#[derive(Clone, Debug, Message)]
pub struct B {
    pub a: A,
}
#[derive(Clone, Debug, Message)]
pub struct C {
    pub b: B,
}
"#
);

check_generated!(
    my_constants_t,
    r##"#[derive(Clone, Debug, Message)]
pub struct MyConstants {
}
impl MyConstants {
    pub const YELLOW: i32 = 1;
    pub const GOLDENROD: i32 = 2;
    pub const CANARY: i32 = 3;
    pub const E: f64 = 2.8718;
}
"##
);

check_generated!(
    point2d_list_t,
    r#"#[derive(Clone, Debug, Message)]
pub struct Point2dList {
    pub npoints: i32,
    #[lcm(length = "npoints")]
    pub points: Vec<[f64; 2]>,
}
"#
);

check_generated!(
    temperature_t,
    r##"#[derive(Clone, Debug, Message)]
pub struct Temperature {
    pub utime: i64,
    #[doc = r#" Temperature in degrees Celsius. A "float" would probably
     * be good enough, unless we're measuring temperatures during
     * the big bang. Note that the asterisk on the beginning of this
     * line is not syntactically necessary, it's just pretty.
     "#]
    pub degCelsius: f64,
}
"##
);

/// Tests the case where multiple members share the same type:
///
/// ```
/// double x, y, z;
/// ```
check_generated!(
    member_group,
    r##"#[derive(Clone, Debug, Message)]
pub struct MemberGroup {
    #[doc = r#" A vector."#]
    pub x: f64,
    #[doc = r#" A vector."#]
    pub y: f64,
    #[doc = r#" A vector."#]
    pub z: f64,
}
"##
);

#[test]
fn optional_traits() {
    let module = ast::Module {
        submodules: HashMap::new(),
        structs: vec![
            ast::Struct {
                comment: None,
                name: "MyType".into(),
                fields: vec![],
                constants: vec![],
            },
        ],
    };

    let config = Config {
        additional_traits: vec!["Serialize".into(), "Deserialize".into(), "PartialEq".into()],
        ..Config::default()
    };
    let generated = codegen::generate_with_config(&module, &config);

    let expected = r#"#[derive(Clone, Debug, Deserialize, Message, PartialEq, Serialize)]
pub struct MyType {
}
"#;

    assert_eq!(generated, expected);
}
