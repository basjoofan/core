use crate::Token;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Ident(Token, String),
    Integer(Token, i64),
    Float(Token, f64),
    Boolean(Token, bool),
    String(Token, String),
    Let(Token, String, Box<Expr>),
    Return(Token, Box<Expr>),
    Unary(Token, Box<Expr>),
    Binary(Token, Box<Expr>, Box<Expr>),
    Paren(Token, Box<Expr>),
    If(Token, Box<Expr>, Vec<Expr>, Vec<Expr>),
    Function(Token, Vec<String>, Vec<Expr>),
    Call(Token, Box<Expr>, Vec<Expr>),
    Array(Token, Vec<Expr>),
    Map(Token, Vec<(Expr, Expr)>),
    Index(Token, Box<Expr>, Box<Expr>),
    // Field Access of a named field (object.field)
    Field(Token, Box<Expr>, String),
    Request(Token, String, Vec<Expr>, Vec<Expr>),
    Test(Token, String, Vec<Expr>),
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
            Expr::Ident(token, _) => write!(f, "{}", token),
            Expr::Integer(token, _) => write!(f, "{}", token),
            Expr::Float(token, _) => write!(f, "{}", token),
            Expr::Boolean(token, _) => write!(f, "{}", token),
            Expr::String(token, _) => write!(f, "\"{}\"", token),
            Expr::Let(token, name, value) => {
                write!(f, "{} {} = {}", token, name, value)
            }
            Expr::Return(token, value) => {
                write!(f, "{} {}", token, value)
            }
            Expr::Unary(token, right) => write!(f, "({}{})", token, right),
            Expr::Binary(token, left, right) => {
                write!(f, "({} {} {})", left, token, right)
            }
            Expr::Paren(_, value) => write!(f, "{}", value),
            Expr::If(token, condition, consequence, alternative) => {
                write!(f, "{} ({}) {{ ", token, condition)?;
                write!(
                    f,
                    "{}",
                    consequence
                        .iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<String>>()
                        .join(";")
                )?;
                if !alternative.is_empty() {
                    write!(
                        f,
                        "}} else {{ {}",
                        alternative
                            .iter()
                            .map(|e| e.to_string())
                            .collect::<Vec<String>>()
                            .join(";")
                    )?
                }
                write!(f, " }}")?;
                Ok(())
            }
            Expr::Function(token, parameters, body) => write!(
                f,
                "{} ({}) {{ {} }}",
                token,
                parameters
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<String>>()
                    .join(", "),
                body.iter().map(|e| e.to_string()).collect::<Vec<String>>().join(";")
            ),
            Expr::Call(_, function, arguments) => write!(
                f,
                "{}({})",
                function,
                arguments
                    .iter()
                    .map(|a| a.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Expr::Array(_, elements) => write!(
                f,
                "[{}]",
                elements
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Expr::Map(_, pairs) => write!(
                f,
                "{{{}}}",
                pairs
                    .iter()
                    .map(|(k, v)| format!("{}:{}", k, v))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Expr::Index(_, left, index) => write!(f, "({}[{}])", left, index),
            Expr::Field(token, object, field) => write!(f, "{}{}{}", object, token, field),
            Expr::Request(token, name, pieces, asserts) => {
                write!(
                    f,
                    "{} {} `{}`[{}]",
                    token,
                    name,
                    pieces
                        .iter()
                        .map(|e| {
                            if let Expr::String(token, _) = e {
                                token.to_string()
                            } else {
                                format!("${{{}}}", e)
                            }
                        })
                        .collect::<String>(),
                    asserts
                        .iter()
                        .map(|e| e.to_string())
                        .collect::<Vec<String>>()
                        .join(", ")
                )
            }
            Expr::Test(token, name, block) => write!(
                f,
                "{} {} {{ {} }}",
                token,
                name,
                block.iter().map(|e| e.to_string()).collect::<Vec<String>>().join(";")
            ),
        }
    }
}
