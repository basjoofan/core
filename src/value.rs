use crate::join;
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
    Function(Vec<Opcode>, usize, usize),
    Closure(Vec<Opcode>, usize, usize, Vec<Value>),
}

impl Value {
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
            Value::Function(..) => "Function",
            Value::Closure(..) => "Closure",
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
            Value::Array(elements) => write!(f, "[{}]", join!(elements, "{}", ", ")),
            Value::Map(pairs) => write!(f, "{{{}}}", join!(pairs, "{}", ": ", ", ")),
            Value::Function(opcodes, length, number) => {
                write!(f, "({}:{}){:?}", length, number, opcodes)
            }
            Value::Closure(opcodes, length, number, frees) => {
                write!(f, "({}:{}:{}){:?}", length, number, frees.len(), opcodes)
            }
        }
    }
}
