use super::Token;
use super::Value;
use crate::client::Clients;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

#[derive(Clone, PartialEq)]
pub enum Expr {
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(String),
    Array(Vec<Expr>),
    Map(Vec<(String, Expr)>),
    Index(Box<Expr>, Box<Expr>),
    // Field Access of a named field (left.field)
    Field(Box<Expr>, String),
    Ident(String),
    Let(String, Box<Expr>),
    Unary(Token, Box<Expr>),
    Binary(Token, Box<Expr>, Box<Expr>),
    Paren(Box<Expr>),
    If(Box<Expr>, Vec<Expr>, Vec<Expr>),
    Function(String, Vec<String>, Vec<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Break(Option<Box<Expr>>),
    Continue,
    Loop(Vec<Expr>),
    While(Box<Expr>, Vec<Expr>),
    For(Vec<String>, Box<Expr>, Vec<Expr>),
    Range(Option<Box<Expr>>, Option<Box<Expr>>, bool),
}

impl Expr {
    pub fn eval(&self) -> Value {
        match self {
            Expr::Integer(integer) => Value::Integer(*integer),
            Expr::Float(float) => Value::Float(*float),
            Expr::Boolean(boolean) => Value::Boolean(*boolean),
            Expr::String(string) => Value::String(string.to_owned()),
            Expr::Array(items) => {
                Value::Array(items.iter().map(|e| e.eval()).collect::<Vec<Value>>())
            }
            Expr::Map(pairs) => Value::Map(
                pairs
                    .iter()
                    .map(|(k, v)| (k.to_owned(), v.eval()))
                    .collect::<HashMap<String, Value>>(),
            ),
            _ => Value::Null,
        }
    }
}

macro_rules! join {
    ($ident: ident, $format: literal, $separator:literal) => {
        $ident
            .iter()
            .map(|e| format!($format, e))
            .collect::<Vec<String>>()
            .join($separator)
    };
    ($ident: ident, $format: literal, $middle:literal, $separator:literal) => {
        $ident
            .iter()
            .map(|(k, v)| format!(concat!($format, $middle, $format), k, v))
            .collect::<Vec<String>>()
            .join($separator)
    };
}

impl Display for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Expr::Integer(integer) => write!(f, "{integer}"),
            Expr::Float(float) => write!(f, "{float}"),
            Expr::Boolean(boolean) => write!(f, "{boolean}"),
            Expr::String(string) => write!(f, "\"{string}\""),
            Expr::Ident(ident) => write!(f, "{ident}"),
            Expr::Array(items) => write!(f, "[{}]", join!(items, "{}", ", ")),
            Expr::Map(pairs) => write!(f, "{{{}}}", join!(pairs, "{}", ": ", ", ")),
            Expr::Index(left, index) => write!(f, "{left}[{index}]"),
            Expr::Field(left, field) => write!(f, "{left}.{field}"),
            Expr::Let(name, value) => {
                write!(f, "let {name} = {value}")
            }
            Expr::Unary(token, right) => write!(f, "{token}{right}"),
            Expr::Binary(token, left, right) => {
                write!(f, "{left} {token} {right}")
            }
            Expr::Paren(value) => write!(f, "({value})"),
            Expr::If(condition, consequence, alternative) => {
                write!(f, "if ({}) {{ {}", condition, join!(consequence, "{}", ";"))?;
                if !alternative.is_empty() {
                    write!(f, "}} else {{ {}", join!(alternative, "{}", ";"))?
                }
                write!(f, " }}")
            }
            Expr::Function(name, params, body) => {
                write!(
                    f,
                    "fn {name}({}) {{ {} }}",
                    params.join(", "),
                    join!(body, "{}", ";")
                )
            }
            Expr::Call(function, arguments) => {
                write!(f, "{}({})", function, join!(arguments, "{}", ", "))
            }
            Expr::Break(value) => {
                write!(f, "break")?;
                if let Some(value) = value {
                    write!(f, " {value}")?;
                }
                Ok(())
            }
            Expr::Continue => write!(f, "continue"),
            Expr::Loop(body) => write!(f, "loop {{ {} }}", join!(body, "{}", ";")),
            Expr::While(condition, body) => {
                write!(f, "while ({condition}) {{ {} }}", join!(body, "{}", ";"))
            }
            Expr::For(bindings, iterator, body) => {
                write!(
                    f,
                    "for {} in {iterator} {{ {} }}",
                    bindings.join(", "),
                    join!(body, "{}", ";")
                )
            }
            Expr::Range(start, end, half) => {
                if let Some(start) = start {
                    write!(f, "{start}")?;
                }
                if *half {
                    write!(f, "..=")?;
                } else {
                    write!(f, "..")?;
                }
                if let Some(end) = end {
                    write!(f, "{end}")?;
                }
                Ok(())
            }
        }
    }
}

impl Debug for Expr {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Expr::Index(left, index) => write!(f, "({left:?}[{index:?}])"),
            Expr::Unary(token, right) => write!(f, "({token}{right:?})"),
            Expr::Binary(token, left, right) => {
                write!(f, "({left:?} {token} {right:?})")
            }
            Expr::Call(function, arguments) => {
                write!(f, "{}({})", function, join!(arguments, "{:?}", ", "))
            }
            _ => write!(f, "{self}"),
        }
    }
}

#[derive(Clone, Default)]
pub struct Source {
    pub exprs: Vec<Expr>,
    pub functions: HashMap<String, (Vec<String>, Vec<Expr>)>,
    pub clients: Clients,
    pub tests: HashMap<String, Vec<Expr>>,
}

impl Source {
    pub fn new() -> Self {
        Self {
            exprs: Vec::new(),
            functions: HashMap::new(),
            clients: Clients::default(),
            tests: HashMap::new(),
        }
    }

    pub fn extend(&mut self, source: Self) -> usize {
        let length = self.exprs.len();
        self.exprs.extend(source.exprs);
        self.functions.extend(source.functions);
        self.clients.extend(source.clients);
        self.tests.extend(source.tests);
        length
    }

    pub fn function(&self, name: &str) -> Option<&(Vec<String>, Vec<Expr>)> {
        self.functions.get(name)
    }

    pub fn test(&self, name: &str) -> Option<&Vec<Expr>> {
        self.tests.get(name)
    }
}
