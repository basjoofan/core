use crate::token::Token;
use std::collections::HashMap;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

pub struct Source {
    pub block: Vec<Expr>,
    pub calls: Vec<Expr>,
    pub tests: HashMap<String, Vec<Expr>>,
}

#[derive(Clone, PartialEq)]
pub enum Expr {
    Ident(Token, String),
    Integer(Token, Option<i64>),
    Float(Token, Option<f64>),
    Boolean(Token, Option<bool>),
    String(Token, String),
    Let(Token, Option<String>, Option<Box<Expr>>),
    Return(Token, Option<Box<Expr>>),
    Unary(Token, Option<Box<Expr>>),
    Binary(Token, Option<Box<Expr>>, Option<Box<Expr>>),
    Paren(Token, Option<Box<Expr>>),
    If(Token, Option<Box<Expr>>, Vec<Expr>, Vec<Expr>),
    Function(Token, Option<String>, Vec<String>, Vec<Expr>),
    Call(Token, Option<Box<Expr>>, Vec<Expr>),
    Array(Token, Vec<Expr>),
    Map(Token, Vec<(Expr, Expr)>),
    Index(Token, Option<Box<Expr>>, Option<Box<Expr>>),
    // Field Access of a named field (object.field)
    Field(Token, Option<Box<Expr>>, Option<String>),
    Request(Token, String, Vec<Expr>, Vec<Expr>),
    Test(Token, String, Vec<Expr>),
    // TODO Assign An assignment expression: a = compute().
    // TODO Closure A closure expression: |a, b| a + b.
    // TODO Break A break, with an optional label to break and an optional expression.
    // TODO Continue A continue, with an optional label.
    // TODO Const A const expression: const a = 1.
    // TODO For A for loop: for pat in expr { ... }.
    // TODO Range A range expression: 1..2, 1.., ..2, 1..=2, ..=2.
    // TODO While A while loop: while expr { ... }.
}

impl Display for Source {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        for expr in self.block.iter() {
            write!(f, "{}", expr)?
        }
        for call in self.calls.iter() {
            write!(f, "{}", call)?
        }
        for (name, block) in self.tests.iter() {
            write!(
                f,
                "test {} {{ {} }}",
                name,
                block.iter().map(|e| e.to_string()).collect::<Vec<String>>().join(";")
            )?
        }
        Ok(())
    }
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
                write!(f, "{} {} = {}", token, May(name), May(value))
            }
            Expr::Return(token, value) => {
                write!(f, "{} {}", token, May(value))
            }
            Expr::Unary(token, right) => write!(f, "({}{})", token, May(right)),
            Expr::Binary(token, left, right) => {
                write!(f, "({} {} {})", May(left), token, May(right))
            }
            Expr::Paren(_, value) => write!(f, "{}", May(value)),
            Expr::If(token, condition, consequence, alternative) => {
                write!(f, "{} ({}) {{ ", token, May(condition))?;
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
            Expr::Function(token, name, parameters, body) => write!(
                f,
                "{} {} ({}) {{ {} }}",
                token,
                May(name),
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
                May(function),
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
            Expr::Index(_, left, index) => write!(f, "({}[{}])", May(left), May(index)),
            Expr::Field(token, object, field) => write!(f, "{}{}{}", May(object), token, May(field)),
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

struct May<'a, T>(pub &'a Option<T>);

impl<'a, T: std::fmt::Display> std::fmt::Display for May<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.0 {
            Some(ref t) => write!(f, "{}", t),
            None => write!(f, "?"),
        }
    }
}

#[test]
fn test_source_display() {
    let source = Source {
        block: vec![Expr::Let(
            Token::new(crate::token::Kind::Let, String::from("let")),
            Some(String::from("myVar")),
            Some(Box::new(Expr::Ident(
                Token::new(crate::token::Kind::Ident, String::from("anotherVar")),
                String::from("anotherVar"),
            ))),
        )],
        calls: vec![],
        tests: HashMap::new(),
    };
    assert_eq!(source.to_string(), "let myVar = anotherVar");
}
