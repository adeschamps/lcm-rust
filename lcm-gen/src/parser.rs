use ast;
use failure::Error;
use pest::Parser;
use pest::iterators::Pair;

/// A parser for the LCM language.
#[derive(Parser)]
#[grammar = "lcm.pest"]
pub struct LcmParser;

// The grammar is included like this so that changes in the .pest file
// trigger a recompilation.
#[cfg(debug_assertions)]
const _GRAMMAR: &str = include_str!("lcm.pest");

pub fn parse_file(input: &str) -> Result<ast::File, Error> {
    let mut pairs = LcmParser::parse(Rule::lcm_file, input)
        .map_err(|e| format_err!("Failed to parse file:\n{}", e))?
        .next()
        .expect("Exactly one file should have been parsed")
        .into_inner()
        .peekable();

    let namespaces = match pairs.peek() {
        Some(pair) if pair.as_rule() == Rule::lcm_package => pair.clone()
            .into_inner()
            .map(|p| parse_namespace(&p))
            .collect(),
        _ => vec![],
    };

    let mut structs = Vec::new();
    let mut last_comment = None;

    for pair in pairs {
        match pair.as_rule() {
            Rule::lcm_package => {}
            Rule::lcm_struct => {
                structs.push(parse_struct(last_comment.take(), pair));
            }
            Rule::comment => {
                last_comment = Some(parse_comment(pair));
            }
            _ => unreachable!(),
        }
    }

    Ok(ast::File {
        namespaces,
        structs,
    })
}

fn parse_namespace(pair: &Pair<Rule>) -> ast::Namespace {
    ast::Namespace(pair.as_str().into())
}

fn parse_struct(comment: Option<ast::Comment>, pair: Pair<Rule>) -> ast::Struct {
    let mut pairs = pair.into_inner();
    let name = match pairs.next() {
        Some(ref pair) if pair.as_rule() == Rule::struct_name => pair.as_str().into(),
        _ => unreachable!(),
    };

    let mut fields = Vec::new();
    let mut constants = Vec::new();
    let mut last_comment = None;

    for pair in pairs {
        match pair.as_rule() {
            Rule::member => {
                fields.push(parse_field(last_comment.take(), pair));
            }
            Rule::constant_group => {
                let mut pairs = pair.into_inner();
                let ty = parse_type(pairs.next().expect("Guaranteed by grammar"));
                for pair in pairs {
                    constants.push(parse_constant(last_comment.take(), ty.clone(), pair));
                }
            }
            Rule::comment => {
                last_comment = Some(parse_comment(pair));
            }
            _ => unreachable!(),
        }
    }

    ast::Struct {
        comment,
        name,
        fields,
        constants,
    }
}

fn parse_field(comment: Option<ast::Comment>, pair: Pair<Rule>) -> ast::Field {
    let mut pairs = pair.into_inner();
    let ty = parse_type(pairs.next().expect("Guaranteed by grammar"));
    let name = parse_name(&pairs.next().expect("Guaranteed by grammar"));
    let multiplicity = pairs.map(parse_multiplicity).collect();

    ast::Field {
        comment,
        name,
        ty,
        multiplicity,
    }
}

fn parse_constant(comment: Option<ast::Comment>, ty: ast::Type, pair: Pair<Rule>) -> ast::Constant {
    let mut pairs = pair.into_inner();
    let name = parse_name(&pairs.next().expect("Guaranteed by grammar"));
    let value = parse_value(&pairs.next().expect("Guaranteed by grammar"));
    ast::Constant {
        comment,
        name,
        ty,
        value,
    }
}

fn parse_type(pair: Pair<Rule>) -> ast::Type {
    let pair = pair.into_inner().next().expect("Guaranteed by grammar");
    match pair.as_rule() {
        Rule::int8_t => ast::Type::Int8,
        Rule::int16_t => ast::Type::Int16,
        Rule::int32_t => ast::Type::Int32,
        Rule::int64_t => ast::Type::Int64,
        Rule::float => ast::Type::Float,
        Rule::double => ast::Type::Double,
        Rule::string => ast::Type::String,
        Rule::boolean => ast::Type::Boolean,
        Rule::byte => ast::Type::Byte,
        Rule::message_t => parse_message_type(pair),
        rule => unreachable!(format!("Encountered {:?}", rule)),
    }
}

fn parse_message_type(pair: Pair<Rule>) -> ast::Type {
    let mut namespaces = Vec::new();

    pair.into_inner()
        .inspect(|pair| {
            if pair.as_rule() == Rule::package_name {
                namespaces.push(parse_namespace(pair));
            }
        })
        .last()
        .map(|pair| ast::Type::Struct(namespaces, pair.as_str().into()))
        .unwrap()
}

fn parse_name(pair: &Pair<Rule>) -> String {
    pair.as_str().into()
}

fn parse_multiplicity(pair: Pair<Rule>) -> ast::Multiplicity {
    let pair = pair.into_inner().next().unwrap();
    match pair.as_rule() {
        Rule::unsigned_int_literal => ast::Multiplicity::Constant(parse_integer(&pair)),
        Rule::member_name => ast::Multiplicity::Variable(pair.as_str().into()),
        _ => unreachable!(),
    }
}

fn parse_value(pair: &Pair<Rule>) -> String {
    pair.as_str().into()
}

fn parse_integer(pair: &Pair<Rule>) -> usize {
    pair.as_str()
        .parse()
        .expect("Should have parsed a valid integer")
}

fn parse_comment(pair: Pair<Rule>) -> ast::Comment {
    let mut comment = String::new();
    for pair in pair.into_inner() {
        match pair.as_rule() {
            Rule::line_comment => {
                if !comment.is_empty() {
                    comment.push('\n');
                }
                // Skip the leading two slashes.
                comment.push_str(&pair.as_str()[2..]);
            }
            Rule::block_comment => {
                let s = pair.as_str();
                comment.push_str(&s[2..s.len() - 2]);
            }
            _ => unreachable!(),
        }
    }
    ast::Comment(comment)
}
