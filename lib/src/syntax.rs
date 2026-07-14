use super::Token;
use super::Value;
use crate::api::Apis;
use std::collections::HashMap;
use std::fmt::{Debug, Display, Formatter, Result};

#[derive(Clone, PartialEq)]
pub enum Expr {
    Null,
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(String),
    Raw(String),
    Array(Vec<Expr>),
    Map(Vec<(String, Expr)>),
    Index(Box<Expr>, Box<Expr>),
    Field(Box<Expr>, String),
    Ident(String),
    Let(String, Box<Expr>),
    Expect(Box<Expr>),
    Unary(Token, Box<Expr>),
    Binary(Token, Box<Expr>, Box<Expr>),
    Paren(Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
}

impl Expr {
    pub fn eval(&self) -> Value {
        match self {
            Self::Integer(value) => Value::Integer(*value),
            Self::Null => Value::Null,
            Self::Float(value) => Value::Float(*value),
            Self::Boolean(value) => Value::Boolean(*value),
            Self::String(value) | Self::Raw(value) => Value::String(value.clone()),
            Self::Map(values) => Value::Map(
                values
                    .iter()
                    .map(|(key, value)| (key.clone(), value.eval()))
                    .collect(),
            ),
            Self::Array(values) => Value::Array(values.iter().map(Self::eval).collect()),
            _ => Value::Null,
        }
    }
}

impl Display for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Self::Integer(value) => write!(f, "{value}"),
            Self::Null => write!(f, "null"),
            Self::Float(value) => write!(f, "{value}"),
            Self::Boolean(value) => write!(f, "{value}"),
            Self::String(value) => write!(f, "\"{value}\""),
            Self::Raw(value) => write!(f, "`{value}`"),
            Self::Ident(value) => write!(f, "{value}"),
            Self::Map(values) => write!(
                f,
                "{{{}}}",
                values
                    .iter()
                    .map(|(key, value)| format!("{key}: {value}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Array(values) => write!(f, "[{}]", join(values, ", ")),
            Self::Index(left, index) => write!(f, "{left}[{index}]"),
            Self::Field(left, field) => write!(f, "{left}.{field}"),
            Self::Let(name, value) => write!(f, "let {name} = {value}"),
            Self::Expect(value) => write!(f, "expect {value}"),
            Self::Unary(token, value) => write!(f, "{token}{value}"),
            Self::Binary(token, left, right) => write!(f, "{left} {token} {right}"),
            Self::Paren(value) => write!(f, "({value})"),
            Self::Call(function, arguments) => write!(f, "{function}({})", join(arguments, ", ")),
        }
    }
}

impl Debug for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        Display::fmt(self, f)
    }
}

fn join(values: &[Expr], separator: &str) -> String {
    values
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(separator)
}

#[derive(Clone, Debug, PartialEq)]
pub struct Environment {
    pub name: String,
    pub fields: Vec<(String, Expr)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Test {
    pub name: String,
    pub tags: Vec<String>,
    pub body: Vec<Expr>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Declaration {
    Environment(Environment),
    Api(crate::api::Api),
    Test(Test),
}

#[derive(Clone, Debug, Default)]
pub struct Source {
    pub declarations: Vec<Declaration>,
    pub environments: HashMap<String, Environment>,
    pub apis: Apis,
    pub tests: HashMap<String, Test>,
    pub exprs: Vec<Expr>,
}

impl Source {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn extend(&mut self, source: Self) -> usize {
        let length = self.exprs.len();
        self.declarations.extend(source.declarations);
        self.environments.extend(source.environments);
        self.apis.extend(source.apis);
        self.tests.extend(source.tests);
        self.exprs.extend(source.exprs);
        length
    }

    pub fn test(&self, name: &str) -> Option<&Test> {
        self.tests.get(name)
    }
}
