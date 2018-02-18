extern crate lcm_gen;
#[macro_use]
extern crate pest;

use lcm_gen::parser::{LcmParser, Rule};

#[test]
fn package() {
    parses_to!{
        parser: LcmParser,
        input: "package exlcm ;",
        rule: Rule::lcm_package,
        tokens: [
            lcm_package(0, 15, [
                package_name(8, 13)
            ])
        ]
    }
}

#[test]
fn float_literal() {
    parses_to! {
        parser: LcmParser,
        input: "1.23",
        rule: Rule::float_literal,
        tokens: [
            float_literal(0, 4)
        ]
    }
}

#[test]
fn exponents() {
    parses_to! {
        parser: LcmParser,
        input: "1e6",
        rule: Rule::float_literal,
        tokens: [
            float_literal(0, 3)
        ]
    }
}

#[test]
fn member_type() {
    parses_to! {
        parser: LcmParser,
        input: "int32_t",
        rule: Rule::lcm_type,
        tokens: [
            lcm_type(0, 7, [
                int32_t(0, 7)
            ])
        ]
    }
}

#[test]
fn member() {
    parses_to!{
        parser: LcmParser,
        input: "int32_t foo ;",
        rule: Rule::member,
        tokens: [
            member(0, 13, [
                lcm_type(0, 7, [
                    int32_t(0, 7)
                ]),
                member_name(8, 11)
            ])
        ]
    }
}

#[test]
fn multiplicity_constant() {
    parses_to!{
        parser: LcmParser,
        input: "[16]",
        rule: Rule::multiplicity,
        tokens: [
            multiplicity(0, 4, [
                unsigned_int_literal(1, 3)
            ])
        ]
    }
}

#[test]
fn multiplicity_variable() {
    parses_to!{
        parser: LcmParser,
        input: "[num_elements]",
        rule: Rule::multiplicity,
        tokens: [
            multiplicity(0, 14, [
                member_name(1, 13)
            ])
        ]
    }
}

#[test]
fn member_2d_array() {
    parses_to!{
        parser: LcmParser,
        input: "int32_t foo[3][count];",
        rule: Rule::member,
        tokens: [
            member(0, 22, [
                lcm_type(0, 7, [
                    int32_t(0, 7)
                ]),
                member_name(8, 11),
                multiplicity(11, 14, [
                    unsigned_int_literal(12, 13)
                ]),
                multiplicity(14, 21, [
                    member_name(15, 20)
                ]),
            ])
        ]
    }
}

#[test]
fn constant() {
    parses_to!{
        parser: LcmParser,
        input: "E=2.8718",
        rule: Rule::constant,
        tokens: [
            constant(0, 8, [
                constant_name(0, 1),
                constant_value(2, 8, [
                    float_literal(2, 8)
                ])
            ])
        ]
    }
}

#[test]
fn simple_constant() {
    parses_to!{
        parser: LcmParser,
        input: "const double E=2.8718;",
        rule: Rule::constant_group,
        tokens: [
            constant_group(0, 22, [
                lcm_type(6, 12, [
                    double(6, 12)
                ]),
                constant(13, 21, [
                    constant_name(13, 14),
                    constant_value(15, 21, [
                        float_literal(15, 21)
                    ])
                ]),
            ])
        ]
    }
}

#[test]
fn multiple_constants() {
    parses_to!{
        parser: LcmParser,
        input: "const int32_t YELLOW=1, GOLDENROD=2, CANARY=3;",
        rule: Rule::constant_group,
        tokens: [
            constant_group(0, 46, [
                lcm_type(6, 13, [
                    int32_t(6, 13)
                ]),
                constant(14, 22, [
                    constant_name(14, 20),
                    constant_value(21, 22, [
                        int_literal(21, 22)
                    ]),
                ]),
                constant(24, 35, [
                    constant_name(24, 33),
                    constant_value(34, 35, [
                        int_literal(34, 35)
                    ]),
                ]),
                constant(37, 45, [
                    constant_name(37, 43),
                    constant_value(44, 45, [
                        int_literal(44, 45)
                    ]),
                ]),
            ])
        ]
    }
}

#[test]
fn lcm_struct() {
    parses_to!{
        parser: LcmParser,
        input: "struct foo_t {}",
        rule: Rule::lcm_struct,
        tokens: [
            lcm_struct(0, 15, [
                struct_name(7, 12),
            ])
        ]
    }
}

#[test]
fn struct_with_comments() {
    parses_to!{
        parser: LcmParser,
        input: include_str!("data/temperature_t.lcm"),
        rule: Rule::lcm_struct,
        tokens: [
            lcm_struct(0, 379, [
                struct_name(7, 20),
                member(27, 43, [
                    lcm_type(27, 34, [
                        int64_t(27, 34),
                    ]),
                    member_name(37, 42),
                ]),
                comment(52, 81, [
                    line_comment(52, 81),
                ]),
                comment(87, 351, [
                    block_comment(87, 351),
                ]),
                member(356, 377, [
                    lcm_type(356, 362, [
                        double(356, 362),
                    ]),
                    member_name(366, 376),
                ])
            ])
        ]
    }
}

#[test]
fn struct_with_array() {
    parses_to!{
        parser: LcmParser,
        input: include_str!("data/point2d_list_t.lcm"),
        rule: Rule::lcm_struct,
        tokens: [
            lcm_struct(0, 78, [
                struct_name(7, 21),
                member(28, 44, [
                    lcm_type(28, 35, [
                        int32_t(28, 35)
                    ]),
                    member_name(36, 43),
                ]),
                member(49, 76, [
                    lcm_type(49, 55, [
                        double(49, 55)
                    ]),
                    member_name(57, 63),
                    multiplicity(63, 72, [
                        member_name(64, 71)
                    ]),
                    multiplicity(72, 75, [
                        unsigned_int_literal(73, 74)
                    ]),
                ]),
            ])
        ]
    }
}

#[test]
fn struct_with_constants() {
    parses_to!{
        parser: LcmParser,
        input: include_str!("data/my_constants_t.lcm"),
        rule: Rule::lcm_struct,
        tokens: [
            lcm_struct(0, 103, [
                struct_name(7, 21),
                constant_group(28, 74, [
                    lcm_type(34, 41, [
                        int32_t(34, 41)
                    ]),
                    constant(42, 50, [
                        constant_name(42, 48),
                        constant_value(49, 50, [
                            int_literal(49, 50)
                        ]),
                    ]),
                    constant(52, 63, [
                        constant_name(52, 61),
                        constant_value(62, 63, [
                            int_literal(62, 63)
                        ]),
                    ]),
                    constant(65, 73, [
                        constant_name(65, 71),
                        constant_value(72, 73, [
                            int_literal(72, 73)
                        ]),
                    ]),
                ]),
                constant_group(79, 101, [
                    lcm_type(85, 91, [
                        double(85, 91)
                    ]),
                    constant(92, 100, [
                        constant_name(92, 93),
                        constant_value(94, 100, [
                            float_literal(94, 100)
                        ]),
                    ])
                ]),
            ])
        ]
    }
}

#[test]
fn struct_with_namespace() {
    parses_to!{
        parser: LcmParser,
        input: include_str!("data/camera_image_t.lcm"),
        rule: Rule::lcm_file,
        tokens: [
            lcm_file(0, 149, [
                lcm_package(0, 15, [
                    package_name(8, 14),
                ]),
                lcm_struct(17, 148, [
                    struct_name(24, 38),
                    member(45, 64, [
                        lcm_type(45, 52, [
                            int64_t(45, 52),
                        ]),
                        member_name(58, 63),
                    ]),
                    member(69, 94, [
                        lcm_type(69, 75, [
                            string(69, 75),
                        ]),
                        member_name(82, 93),
                    ]),
                    member(99, 123, [
                        lcm_type(99, 111, [
                            message_t(99, 111, [
                                package_name(99, 103),
                                struct_name(104, 111),
                            ]),
                        ]),
                        member_name(112, 122),
                    ]),
                    member(128, 146, [
                        lcm_type(128, 138, [
                            message_t(128, 138, [
                                package_name(128, 131),
                                struct_name(132, 138),
                            ]),
                        ]),
                        member_name(141, 145),
                    ]),
                ]),
            ])
        ]
    }
}

#[test]
fn multiple_structs() {
    parses_to!{
        parser: LcmParser,
        input: include_str!("data/multiple_structs.lcm"),
        rule: Rule::lcm_file,
        tokens: [
            lcm_file(0, 93, [
                lcm_struct(0, 38, [
                    struct_name(7, 8),
                    member(19, 23, [
                        lcm_type(19, 20, [
                            message_t(19, 20, [
                                struct_name(19, 20),
                            ]),
                        ]),
                        member_name(21, 22),
                    ]),
                    member(32, 36, [
                        lcm_type(32, 33, [
                            message_t(32, 33, [
                                struct_name(32, 33),
                            ]),
                        ]),
                        member_name(34, 35),
                    ]),
                ]),
                lcm_struct(40, 65, [
                    struct_name(47, 48),
                    member(59, 63, [
                        lcm_type(59, 60, [
                            message_t(59, 60, [
                                struct_name(59, 60),
                            ]),
                        ]),
                        member_name(61, 62),
                    ]),
                ]),
                lcm_struct(67, 92, [
                    struct_name(74, 75),
                    member(86, 90, [
                        lcm_type(86, 87, [
                            message_t(86, 87, [
                                struct_name(86, 87),
                            ]),
                        ]),
                        member_name(88, 89),
                    ]),
                ]),
            ])
        ]
    }
}

#[test]
fn line_comment() {
    parses_to!{
        parser: LcmParser,
        input: "// comment",
        rule: Rule::comment,
        tokens: [
            comment(0, 10, [
                line_comment(0, 10)
            ])
        ]
    }
}

#[test]
fn multiline_comment() {
    parses_to!{
            parser: LcmParser,
            input: r#"// line 1
    // line 2"#,
            rule: Rule::comment,
            tokens: [
                comment(0, 23, [
                    line_comment(0, 9),
                    line_comment(14, 23),
                ])
            ]
        }
}

#[test]
fn block_comment() {
    parses_to!{
        parser: LcmParser,
        input: r#"/* A comment
on multiple
lines */"#,
        rule: Rule::comment,
        tokens: [
            comment(0, 33, [
                block_comment(0, 33)
            ])
        ]
    }
}

#[test]
fn comments() {
    parses_to!{
        parser: LcmParser,
        input: include_str!("data/comments_t.lcm"),
        rule: Rule::lcm_file,
        tokens: [
            lcm_file(0, 231, [
                comment(0, 52, [
                    line_comment(0, 52),
                ]),
                comment(54, 103, [
                    line_comment(54, 74),
                    line_comment(75, 103),
                ]),
                lcm_struct(104, 230, [
                    struct_name(111, 122),
                    comment(129, 162, [
                        line_comment(129, 162),
                    ]),
                    member(167, 177, [
                        lcm_type(167, 174, [
                            int32_t(167, 174),
                        ]),
                        member_name(175, 176)
                    ]),
                    comment(182, 213, [
                        line_comment(182, 213),
                    ]),
                    member(218, 228, [
                        lcm_type(218, 225, [
                            int32_t(218, 225),
                        ]),
                        member_name(226, 227),
                    ]),
                ])
            ])
        ]
    }
}
