use crate::Expr;
use crate::Opcode;
use std::collections::HashMap;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    None,
    Error(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(String),
    Array(Vec<Value>),
    Map(HashMap<String, Value>),
    Return(Box<Value>),
    Function(Vec<Opcode>, usize, usize),
    Native(fn(Vec<Value>) -> Value),
    Request(String, Vec<Expr>, Vec<Expr>),
}

impl Value {
    pub fn is_error(&self) -> bool {
        matches!(self, Value::Error(_))
    }

    pub fn kind(&self) -> &str {
        match self {
            Value::None => "None",
            Value::Error(_) => "Error",
            Value::Integer(_) => "Integer",
            Value::Float(_) => "Float",
            Value::Boolean(_) => "Boolean",
            Value::String(_) => "String",
            Value::Array(_) => "Array",
            Value::Map(_) => "Map",
            Value::Return(_) => "Return",
            Value::Function(..) => "Function",
            Value::Native(_) => "Native",
            Value::Request(..) => "Request",
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Value::None => write!(f, "none"),
            Value::Error(message) => write!(f, "{}", message),
            Value::Integer(integer) => write!(f, "{}", integer),
            Value::Float(float) => write!(f, "{}", float),
            Value::Boolean(boolean) => write!(f, "{}", boolean),
            Value::String(string) => write!(f, "{}", string),
            Value::Array(elements) => write!(
                f,
                "[{}]",
                elements
                    .iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Value::Map(pairs) => write!(
                f,
                "{{{}}}",
                pairs
                    .iter()
                    .map(|(k, v)| format!("{}:{}", k, v))
                    .collect::<Vec<String>>()
                    .join(", ")
            ),
            Value::Return(value) => write!(f, "{}", value),
            Value::Function(opcodes, length, number) => {
                write!(f, "({}:{}){:?}", length, number, opcodes)
            }
            Value::Native(function) => write!(f, "{:?}", function),
            Value::Request(name, pieces, asserts) => write!(
                f,
                "{} {}[{}]",
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
            ),
        }
    }
}
