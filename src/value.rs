use std::collections::HashMap;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;
use std::ops::Add;
use std::ops::BitAnd;
use std::ops::BitOr;
use std::ops::BitXor;
use std::ops::Div;
use std::ops::Mul;
use std::ops::Rem;
use std::ops::Shl;
use std::ops::Shr;
use std::ops::Sub;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Null,
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(String),
    Array(Vec<Value>),
    Map(HashMap<String, Value>),
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Integer(integer) => write!(f, "{}", integer),
            Value::Float(float) => write!(f, "{}", float),
            Value::Boolean(boolean) => write!(f, "{}", boolean),
            Value::String(string) => write!(f, "{}", string),
            Value::Array(items) => write!(f, "{:?}", items),
            Value::Map(pairs) => write!(f, "{:?}", pairs),
        }
    }
}

impl Add for Value {
    type Output = std::result::Result<Self, String>;
    fn add(self, other: Self) -> Self::Output {
        match (self, other) {
            (Value::Integer(left), Value::Integer(right)) => Ok(Value::Integer(left + right)),
            (Value::Float(left), Value::Float(right)) => Ok(Value::Float(left + right)),
            (Value::Integer(left), Value::Float(right)) => Ok(Value::Float(left as f64 + right)),
            (Value::Float(left), Value::Integer(right)) => Ok(Value::Float(left + right as f64)),
            (Value::String(left), Value::String(right)) => Ok(Value::String(left + &right)),
            (left, right) => Err(format!("type mismatch: {} + {}", left, right)),
        }
    }
}

impl Sub for Value {
    type Output = std::result::Result<Self, String>;
    fn sub(self, other: Self) -> Self::Output {
        match (self, other) {
            (Value::Integer(left), Value::Integer(right)) => Ok(Value::Integer(left - right)),
            (Value::Float(left), Value::Float(right)) => Ok(Value::Float(left - right)),
            (Value::Integer(left), Value::Float(right)) => Ok(Value::Float(left as f64 - right)),
            (Value::Float(left), Value::Integer(right)) => Ok(Value::Float(left - right as f64)),
            (left, right) => Err(format!("type mismatch: {} - {}", left, right)),
        }
    }
}

impl Mul for Value {
    type Output = std::result::Result<Self, String>;
    fn mul(self, other: Self) -> Self::Output {
        match (self, other) {
            (Value::Integer(left), Value::Integer(right)) => Ok(Value::Integer(left * right)),
            (Value::Float(left), Value::Float(right)) => Ok(Value::Float(left * right)),
            (Value::Integer(left), Value::Float(right)) => Ok(Value::Float(left as f64 * right)),
            (Value::Float(left), Value::Integer(right)) => Ok(Value::Float(left * right as f64)),
            (left, right) => Err(format!("type mismatch: {} * {}", left, right)),
        }
    }
}

impl Div for Value {
    type Output = std::result::Result<Self, String>;
    fn div(self, other: Self) -> Self::Output {
        match (self, other) {
            (Value::Integer(left), Value::Integer(right)) => Ok(Value::Integer(left / right)),
            (Value::Float(left), Value::Float(right)) => Ok(Value::Float(left / right)),
            (Value::Integer(left), Value::Float(right)) => Ok(Value::Float(left as f64 / right)),
            (Value::Float(left), Value::Integer(right)) => Ok(Value::Float(left / right as f64)),
            (left, right) => Err(format!("type mismatch: {} / {}", left, right)),
        }
    }
}

impl Rem for Value {
    type Output = std::result::Result<Self, String>;
    fn rem(self, other: Self) -> Self::Output {
        match (self, other) {
            (Value::Integer(left), Value::Integer(right)) => Ok(Value::Integer(left % right)),
            (Value::Float(left), Value::Float(right)) => Ok(Value::Float(left % right)),
            (Value::Integer(left), Value::Float(right)) => Ok(Value::Float(left as f64 % right)),
            (Value::Float(left), Value::Integer(right)) => Ok(Value::Float(left % right as f64)),
            (left, right) => Err(format!("type mismatch: {} % {}", left, right)),
        }
    }
}

impl BitXor for Value {
    type Output = std::result::Result<Self, String>;
    fn bitxor(self, other: Self) -> Self::Output {
        match (self, other) {
            (Value::Integer(left), Value::Integer(right)) => Ok(Value::Integer(left ^ right)),
            (left, right) => Err(format!("type mismatch: {} ^ {}", left, right)),
        }
    }
}

impl BitOr for Value {
    type Output = std::result::Result<Self, String>;
    fn bitor(self, other: Self) -> Self::Output {
        match (self, other) {
            (Value::Integer(left), Value::Integer(right)) => Ok(Value::Integer(left | right)),
            (left, right) => Err(format!("type mismatch: {} | {}", left, right)),
        }
    }
}

impl BitAnd for Value {
    type Output = std::result::Result<Self, String>;
    fn bitand(self, other: Self) -> Self::Output {
        match (self, other) {
            (Value::Integer(left), Value::Integer(right)) => Ok(Value::Integer(left & right)),
            (left, right) => Err(format!("type mismatch: {} & {}", left, right)),
        }
    }
}

impl Shl for Value {
    type Output = std::result::Result<Self, String>;
    fn shl(self, other: Self) -> Self::Output {
        match (self, other) {
            (Value::Integer(left), Value::Integer(right)) => Ok(Value::Integer(left << right)),
            (left, right) => Err(format!("type mismatch: {} << {}", left, right)),
        }
    }
}

impl Shr for Value {
    type Output = std::result::Result<Self, String>;
    fn shr(self, other: Self) -> Self::Output {
        match (self, other) {
            (Value::Integer(left), Value::Integer(right)) => Ok(Value::Integer(left >> right)),
            (left, right) => Err(format!("type mismatch: {} >> {}", left, right)),
        }
    }
}

impl PartialOrd for Value {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        match (self, other) {
            (Value::Integer(left), Value::Integer(right)) => left.partial_cmp(right),
            (Value::Float(left), Value::Float(right)) => left.partial_cmp(right),
            (Value::Integer(left), Value::Float(right)) => (*left as f64).partial_cmp(right),
            (Value::Float(left), Value::Integer(right)) => left.partial_cmp(&(*right as f64)),
            (Value::String(left), Value::String(right)) => left.partial_cmp(right),
            (_, _) => None,
        }
    }
}
