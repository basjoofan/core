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
    Range(Option<i64>, Option<i64>, bool),
}

impl Value {
    pub(crate) fn to_json(&self) -> std::result::Result<String, String> {
        let mut output = String::new();
        self.write_json(&mut output)?;
        Ok(output)
    }

    fn write_json(&self, output: &mut String) -> std::result::Result<(), String> {
        match self {
            Value::Null => output.push_str("null"),
            Value::Integer(value) => output.push_str(&value.to_string()),
            Value::Float(value) if value.is_finite() => output.push_str(&value.to_string()),
            Value::Float(value) => {
                return Err(format!("cannot encode non-finite float {value} as JSON"));
            }
            Value::Boolean(value) => output.push_str(if *value { "true" } else { "false" }),
            Value::String(value) => write_json_string(value, output),
            Value::Array(values) => {
                output.push('[');
                for (index, value) in values.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    value.write_json(output)?;
                }
                output.push(']');
            }
            Value::Map(values) => {
                output.push('{');
                for (index, (key, value)) in values.iter().enumerate() {
                    if index > 0 {
                        output.push(',');
                    }
                    write_json_string(key, output);
                    output.push(':');
                    value.write_json(output)?;
                }
                output.push('}');
            }
            Value::Range { .. } => {
                return Err("ranges cannot be encoded as JSON".to_string());
            }
        }
        Ok(())
    }
}

fn write_json_string(value: &str, output: &mut String) {
    output.push('"');
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\u{08}' => output.push_str("\\b"),
            '\u{0c}' => output.push_str("\\f"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character <= '\u{1f}' => {
                output.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => output.push(character),
        }
    }
    output.push('"');
}

impl Display for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Integer(integer) => write!(f, "{integer}"),
            Value::Float(float) => write!(f, "{float}"),
            Value::Boolean(boolean) => write!(f, "{boolean}"),
            Value::String(string) => write!(f, "{string}"),
            Value::Array(items) => write!(f, "{items:?}"),
            Value::Map(pairs) => write!(f, "{pairs:?}"),
            Value::Range(start, end, half) => {
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

impl Add for Value {
    type Output = std::result::Result<Self, String>;
    fn add(self, other: Self) -> Self::Output {
        match (self, other) {
            (Value::Integer(left), Value::Integer(right)) => Ok(Value::Integer(left + right)),
            (Value::Float(left), Value::Float(right)) => Ok(Value::Float(left + right)),
            (Value::Integer(left), Value::Float(right)) => Ok(Value::Float(left as f64 + right)),
            (Value::Float(left), Value::Integer(right)) => Ok(Value::Float(left + right as f64)),
            (Value::String(left), Value::String(right)) => Ok(Value::String(left + &right)),
            (left, right) => Err(format!("type mismatch: {left} + {right}")),
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
            (left, right) => Err(format!("type mismatch: {left} - {right}")),
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
            (left, right) => Err(format!("type mismatch: {left} * {right}")),
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
            (left, right) => Err(format!("type mismatch: {left} / {right}")),
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
            (left, right) => Err(format!("type mismatch: {left} % {right}")),
        }
    }
}

impl BitXor for Value {
    type Output = std::result::Result<Self, String>;
    fn bitxor(self, other: Self) -> Self::Output {
        match (self, other) {
            (Value::Integer(left), Value::Integer(right)) => Ok(Value::Integer(left ^ right)),
            (left, right) => Err(format!("type mismatch: {left} ^ {right}")),
        }
    }
}

impl BitOr for Value {
    type Output = std::result::Result<Self, String>;
    fn bitor(self, other: Self) -> Self::Output {
        match (self, other) {
            (Value::Integer(left), Value::Integer(right)) => Ok(Value::Integer(left | right)),
            (left, right) => Err(format!("type mismatch: {left} | {right}")),
        }
    }
}

impl BitAnd for Value {
    type Output = std::result::Result<Self, String>;
    fn bitand(self, other: Self) -> Self::Output {
        match (self, other) {
            (Value::Integer(left), Value::Integer(right)) => Ok(Value::Integer(left & right)),
            (left, right) => Err(format!("type mismatch: {left} & {right}")),
        }
    }
}

impl Shl for Value {
    type Output = std::result::Result<Self, String>;
    fn shl(self, other: Self) -> Self::Output {
        match (self, other) {
            (Value::Integer(left), Value::Integer(right)) => Ok(Value::Integer(left << right)),
            (left, right) => Err(format!("type mismatch: {left} << {right}")),
        }
    }
}

impl Shr for Value {
    type Output = std::result::Result<Self, String>;
    fn shr(self, other: Self) -> Self::Output {
        match (self, other) {
            (Value::Integer(left), Value::Integer(right)) => Ok(Value::Integer(left >> right)),
            (left, right) => Err(format!("type mismatch: {left} >> {right}")),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_encode_json() {
        let value = Value::Map(HashMap::from([(
            "data".to_string(),
            Value::Array(vec![
                Value::Null,
                Value::Integer(1),
                Value::Float(1.5),
                Value::Boolean(true),
                Value::String("say \"hi\"\\next\n".to_string()),
            ]),
        )]));
        assert_eq!(
            value.to_json().unwrap(),
            r#"{"data":[null,1,1.5,true,"say \"hi\"\\next\n"]}"#
        );
    }

    #[test]
    fn value_reject_that_json_cannot_represent() {
        assert!(Value::Float(f64::INFINITY).to_json().is_err());
        assert!(Value::Range(Some(1), Some(2), false,).to_json().is_err());
    }
}
