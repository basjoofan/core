use crate::Token;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Ident(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(String),
    Let(String, Box<Expr>),
    Return(Box<Expr>),
    Unary(Token, Box<Expr>),
    Binary(Token, Box<Expr>, Box<Expr>),
    Paren(Box<Expr>),
    If(Box<Expr>, Vec<Expr>, Vec<Expr>),
    Function(Option<String>, Vec<String>, Vec<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Array(Vec<Expr>),
    Map(Vec<(Expr, Expr)>),
    Index(Box<Expr>, Box<Expr>),
    // Field Access of a named field (object.field)
    Field(Box<Expr>, String),
    Request(String, Vec<String>, String, Vec<Expr>),
    Test(String, Vec<Expr>),
    // TODO Assign An assignment expr: a = compute().
    // TODO Closure A closure expr: |a, b| a + b.
    // TODO Break A break, with an optional label to break and an optional expr.
    // TODO Continue A continue, with an optional label.
    // TODO Const A const expr: const a = 1.
    // TODO For A for loop: for pat in expr { ... }.
    // TODO Range A range expr: 1..2, 1.., ..2, 1..=2, ..=2.
    // TODO While A while loop: while expr { ... }.
}

impl Display for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Expr::Ident(ident) => write!(f, "{}", ident),
            Expr::Integer(integer) => write!(f, "{}", integer),
            Expr::Float(float) => write!(f, "{}", float),
            Expr::Boolean(boolean) => write!(f, "{}", boolean),
            Expr::String(string) => write!(f, "\"{}\"", string),
            Expr::Let(name, value) => {
                write!(f, "let {} = {}", name, value)
            }
            Expr::Return(value) => {
                write!(f, "return {}", value)
            }
            Expr::Unary(token, right) => write!(f, "({}{})", token, right),
            Expr::Binary(token, left, right) => {
                write!(f, "({} {} {})", left, token, right)
            }
            // TODO Paren A paren expr: (expr).
            Expr::Paren(value) => write!(f, "{}", value),
            Expr::If(condition, consequence, alternative) => {
                write!(f, "if ({}) {{ ", condition)?;
                write!(
                    f,
                    "{}",
                    consequence.iter().map(|e| e.to_string()).collect::<Vec<String>>().join(";")
                )?;
                if !alternative.is_empty() {
                    write!(
                        f,
                        "}} else {{ {}",
                        alternative.iter().map(|e| e.to_string()).collect::<Vec<String>>().join(";")
                    )?
                }
                write!(f, " }}")?;
                Ok(())
            }
            Expr::Function(name, parameters, body) => write!(
                f,
                "fn {:?} ({}) {{ {} }}",
                name,
                parameters.iter().map(|e| e.to_string()).collect::<Vec<String>>().join(", "),
                body.iter().map(|e| e.to_string()).collect::<Vec<String>>().join(";")
            ),
            Expr::Call(function, arguments) => write!(
                f,
                "{}({})",
                function,
                arguments.iter().map(|a| a.to_string()).collect::<Vec<String>>().join(", ")
            ),
            Expr::Array(elements) => write!(
                f,
                "[{}]",
                elements.iter().map(|e| e.to_string()).collect::<Vec<String>>().join(", ")
            ),
            Expr::Map(pairs) => write!(
                f,
                "{{{}}}",
                pairs
                    .iter()
                    .map(|(k, v)| format!("{}:{}", k, v))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Expr::Index(left, index) => write!(f, "({}[{}])", left, index),
            Expr::Field(object, field) => write!(f, "{}.{}", object, field),
            Expr::Request(name, parameters, message, asserts) => {
                write!(
                    f,
                    "rq {} ({})`{}`[{}]",
                    name,
                    parameters.iter().map(|e| e.to_string()).collect::<Vec<String>>().join(", "),
                    message,
                    asserts.iter().map(|e| e.to_string()).collect::<Vec<String>>().join(", ")
                )
            }
            Expr::Test(name, block) => write!(
                f,
                "test {} {{ {} }}",
                name,
                block.iter().map(|e| e.to_string()).collect::<Vec<String>>().join(";")
            ),
        }
    }
}
