extern crate lcm_gen;
#[macro_use]
extern crate pretty_assertions;

use lcm_gen::{ast, parser};

#[test]
fn parse_temperature() {
    let data = include_str!("data/temperature_t.lcm");
    let file = parser::parse_file(data).expect("Failed to parse file.");

    assert_eq!(
        file,
        ast::File {
            namespaces: vec![],
            structs: vec![
                ast::Struct {
                    comment: None,
                    name: "temperature_t".into(),
                    fields: vec![
                        ast::Field {
                            comment: None,
                            name: "utime".into(),
                            ty: ast::Type::Int64,
                            multiplicity: vec![],
                        },
                        ast::Field {
                            comment: Some(ast::Comment(
                                r#" Temperature in degrees Celsius. A "float" would probably
     * be good enough, unless we're measuring temperatures during
     * the big bang. Note that the asterisk on the beginning of this
     * line is not syntactically necessary, it's just pretty.
     "#.into(),
                            )),
                            name: "degCelsius".into(),
                            ty: ast::Type::Double,
                            multiplicity: vec![],
                        },
                    ],
                    constants: vec![],
                },
            ],
        }
    );
}

#[test]
fn parse_multiple_structs() {
    let data = include_str!("data/multiple_structs.lcm");
    let file = parser::parse_file(data).expect("Failed to parse file.");

    assert_eq!(
        file,
        ast::File {
            namespaces: vec![],
            structs: vec![
                ast::Struct {
                    comment: None,
                    name: "A".into(),
                    fields: vec![
                        ast::Field {
                            comment: None,
                            name: "b".into(),
                            ty: ast::Type::Struct(vec![], "B".into()),
                            multiplicity: vec![],
                        },
                        ast::Field {
                            comment: None,
                            name: "c".into(),
                            ty: ast::Type::Struct(vec![], "C".into()),
                            multiplicity: vec![],
                        },
                    ],
                    constants: vec![],
                },
                ast::Struct {
                    comment: None,
                    name: "B".into(),
                    fields: vec![
                        ast::Field {
                            comment: None,
                            name: "a".into(),
                            ty: ast::Type::Struct(vec![], "A".into()),
                            multiplicity: vec![],
                        },
                    ],
                    constants: vec![],
                },
                ast::Struct {
                    comment: None,
                    name: "C".into(),
                    fields: vec![
                        ast::Field {
                            comment: None,
                            name: "b".into(),
                            ty: ast::Type::Struct(vec![], "B".into()),
                            multiplicity: vec![],
                        },
                    ],
                    constants: vec![],
                },
            ],
        }
    );
}

#[test]
fn parse_point2d_list() {
    let data = include_str!("data/point2d_list_t.lcm");
    let file = parser::parse_file(data).expect("Failed to parse file.");

    assert_eq!(
        file,
        ast::File {
            namespaces: vec![],
            structs: vec![
                ast::Struct {
                    comment: None,
                    name: "point2d_list_t".into(),
                    fields: vec![
                        ast::Field {
                            comment: None,
                            name: "npoints".into(),
                            ty: ast::Type::Int32,
                            multiplicity: vec![],
                        },
                        ast::Field {
                            comment: None,
                            name: "points".into(),
                            ty: ast::Type::Double,
                            multiplicity: vec![
                                ast::Multiplicity::Variable("npoints".into()),
                                ast::Multiplicity::Constant(2),
                            ],
                        },
                    ],
                    constants: vec![],
                },
            ],
        }
    );
}

#[test]
fn parse_camera_image() {
    let data = include_str!("data/camera_image_t.lcm");
    let file = parser::parse_file(data).expect("Failed to parse file.");

    assert_eq!(
        file,
        ast::File {
            namespaces: vec![ast::Namespace("mycorp".into())],
            structs: vec![
                ast::Struct {
                    comment: None,
                    name: "camera_image_t".into(),
                    fields: vec![
                        ast::Field {
                            comment: None,
                            name: "utime".into(),
                            ty: ast::Type::Int64,
                            multiplicity: vec![],
                        },
                        ast::Field {
                            comment: None,
                            name: "camera_name".into(),
                            ty: ast::Type::String,
                            multiplicity: vec![],
                        },
                        ast::Field {
                            comment: None,
                            name: "jpeg_image".into(),
                            ty: ast::Type::Struct(
                                vec![ast::Namespace("jpeg".into())],
                                "image_t".into(),
                            ),
                            multiplicity: vec![],
                        },
                        ast::Field {
                            comment: None,
                            name: "pose".into(),
                            ty: ast::Type::Struct(
                                vec![ast::Namespace("mit".into())],
                                "pose_t".into(),
                            ),
                            multiplicity: vec![],
                        },
                    ],
                    constants: vec![],
                },
            ],
        }
    );
}

#[test]
fn parse_my_constants() {
    let data = include_str!("data/my_constants_t.lcm");
    let file = parser::parse_file(data).expect("Failed to parse file.");

    assert_eq!(
        file,
        ast::File {
            namespaces: vec![],
            structs: vec![
                ast::Struct {
                    comment: None,
                    name: "my_constants_t".into(),
                    fields: vec![],
                    constants: vec![
                        ast::Constant {
                            comment: None,
                            name: "YELLOW".into(),
                            ty: ast::Type::Int32,
                            value: "1".into(),
                        },
                        ast::Constant {
                            comment: None,
                            name: "GOLDENROD".into(),
                            ty: ast::Type::Int32,
                            value: "2".into(),
                        },
                        ast::Constant {
                            comment: None,
                            name: "CANARY".into(),
                            ty: ast::Type::Int32,
                            value: "3".into(),
                        },
                        ast::Constant {
                            comment: None,
                            name: "E".into(),
                            ty: ast::Type::Double,
                            value: "2.8718".into(),
                        },
                    ],
                },
            ],
        }
    );
}

#[test]
fn parse_struct_with_comments() {
    let data = include_str!("data/comments_t.lcm");
    let file = parser::parse_file(data).expect("Failed to parse file.");

    assert_eq!(
        file,
        ast::File {
            namespaces: vec![],
            structs: vec![
                ast::Struct {
                    comment: Some(ast::Comment(
                        r#" This is a comment
 that spans multiple lines"#.into(),
                    )),
                    name: "my_struct_t".into(),
                    fields: vec![
                        ast::Field {
                            comment: Some(ast::Comment(" Horizontal position in meters.".into())),
                            name: "x".into(),
                            ty: ast::Type::Int32,
                            multiplicity: vec![],
                        },
                        ast::Field {
                            comment: Some(ast::Comment(" Vertical position in meters.".into())),
                            name: "y".into(),
                            ty: ast::Type::Int32,
                            multiplicity: vec![],
                        },
                    ],
                    constants: vec![],
                },
            ],
        }
    );
}

#[test]
fn struct_with_namespace_creates_submodules() {
    let mut root_module = ast::Module::default();

    let path = vec![ast::Namespace("foo".into()), ast::Namespace("bar".into())];
    root_module.add_struct(
        &path,
        ast::Struct {
            comment: None,
            name: "S".into(),
            fields: vec![],
            constants: vec![],
        },
    );

    let foo_module = &root_module.submodules[&ast::Namespace("foo".into())];
    let bar_module = &foo_module.submodules[&ast::Namespace("bar".into())];
    assert_eq!(bar_module.structs.len(), 1);
}
