use crate::Token;
use crate::Value;
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
    Map(Vec<(Expr, Expr)>),
    Index(Box<Expr>, Box<Expr>),
    // Field Access of a named field (left.field)
    Field(Box<Expr>, String),
    Ident(String),
    Let(String, Box<Expr>),
    Unary(Token, Box<Expr>),
    Binary(Token, Box<Expr>, Box<Expr>),
    Paren(Box<Expr>),
    If(Box<Expr>, Vec<Expr>, Vec<Expr>),
    Call(String, Vec<Expr>),
    Send(String),
    // TODO Break A break, with an optional label to break and an optional expr.
    // TODO For A for loop: for pat in expr { ... }.
    // TODO Range A range expr: 1..2, 1.., ..2, 1..=2, ..=2.
    // TODO While A while loop: while expr { ... }.
}

impl Expr {
    pub fn eval(&self) -> Value {
        match self {
            Expr::Integer(integer) => Value::Integer(*integer),
            Expr::Float(float) => Value::Float(*float),
            Expr::Boolean(boolean) => Value::Boolean(*boolean),
            Expr::String(string) => Value::String(string.clone()),
            Expr::Array(items) => Value::Array(items.iter().map(|e| e.eval()).collect::<Vec<Value>>()),
            Expr::Map(pairs) => Value::Map(
                pairs
                    .iter()
                    .map(|(k, v)| (k.eval().to_string(), v.eval()))
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
            Expr::Call(function, arguments) => write!(f, "{}({})", function, join!(arguments, "{}", ", ")),
            Expr::Send(request) => write!(f, "{request}->"),
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
            Expr::Call(function, arguments) => write!(f, "{}({})", function, join!(arguments, "{:?}", ", ")),
            _ => write!(f, "{self}"),
        }
    }
}

#[derive(Default)]
pub struct Source {
    pub exprs: Vec<Expr>,
    pub functions: HashMap<String, (Vec<String>, Vec<Expr>)>,
    pub requests: HashMap<String, (String, Vec<Expr>)>,
    pub tests: HashMap<String, Vec<Expr>>,
}

impl Source {
    pub fn new() -> Self {
        Self {
            exprs: Vec::new(),
            functions: HashMap::new(),
            requests: HashMap::new(),
            tests: HashMap::new(),
        }
    }

    pub fn extend(&mut self, source: Self) -> usize {
        let length = self.exprs.len();
        self.exprs.extend(source.exprs);
        self.functions.extend(source.functions);
        self.requests.extend(source.requests);
        self.tests.extend(source.tests);
        length
    }

    pub fn function(&self, name: &str) -> Option<&(Vec<String>, Vec<Expr>)> {
        self.functions.get(name)
    }

    pub fn request(&self, name: &str) -> Option<&(String, Vec<Expr>)> {
        self.requests.get(name)
    }

    pub fn test(&self, name: &str) -> Option<&Vec<Expr>> {
        self.tests.get(name)
    }
}
