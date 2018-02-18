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
    #[derive(Debug, LcmMessage)]
    pub struct camera_image_t {
        pub utime: i64,
        pub camera_name: String,
        pub jpeg_image: jpeg::image_t,
        pub pose: mit::pose_t,
    }
}
"#
);

check_generated!(
    comments_t,
    r#"#[doc = " This is a comment
 that spans multiple lines"]
#[derive(Debug, LcmMessage)]
pub struct my_struct_t {
    #[doc = " Horizontal position in meters."]
    pub x: i32,
    #[doc = " Vertical position in meters."]
    pub y: i32,
}
"#
);

check_generated!(
    multiple_structs,
    r#"#[derive(Debug, LcmMessage)]
pub struct A {
    pub b: B,
    pub c: C,
}
#[derive(Debug, LcmMessage)]
pub struct B {
    pub a: A,
}
#[derive(Debug, LcmMessage)]
pub struct C {
    pub b: B,
}
"#
);

check_generated!(
    my_constants_t,
    r#"#[derive(Debug, LcmMessage)]
pub struct my_constants_t {
}
impl my_constants_t {
    const YELLOW: i32 = 1;
    const GOLDENROD: i32 = 2;
    const CANARY: i32 = 3;
    const E: f64 = 2.8718;
}
"#
);

check_generated!(
    point2d_list_t,
    r#"#[derive(Debug, LcmMessage)]
pub struct point2d_list_t {
    pub npoints: i32,
    #[lcm(length = "npoints; 2")]
    pub points: Vec<[f64; 2]>,
}
"#
);

check_generated!(
    temperature_t,
    r#"#[derive(Debug, LcmMessage)]
pub struct temperature_t {
    pub utime: i64,
    #[doc = " Temperature in degrees Celsius. A "float" would probably
     * be good enough, unless we're measuring temperatures during
     * the big bang. Note that the asterisk on the beginning of this
     * line is not syntactically necessary, it's just pretty.
     "]
    pub degCelsius: f64,
}
"#
);

/// Tests the case where multiple members share the same type:
///
/// ```
/// double x, y, z;
/// ```
check_generated!(
    member_group,
    r#"#[derive(Debug, LcmMessage)]
pub struct member_group {
    #[doc = " A vector."]
    pub x: f64,
    #[doc = " A vector."]
    pub y: f64,
    #[doc = " A vector."]
    pub z: f64,
}
"#
);
