use std::collections::HashMap;
use std::fmt::Debug;
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

#[derive(Clone, PartialEq)]
pub enum Value {
    Null,
    Integer(i64),
    Float(f64),
    Boolean(bool),
    String(String),
    Array(Vec<Value>),
    Map(HashMap<String, Value>),
}

impl Value {
    pub fn json(&self) -> String {
        self.to_json()
    }

    pub fn to_json(&self) -> String {
        match self {
            Self::Null => "null".into(),
            Self::Integer(value) => value.to_string(),
            Self::Float(value) => value.to_string(),
            Self::Boolean(value) => value.to_string(),
            Self::String(value) => format!("\"{}\"", escape_json(value)),
            Self::Array(values) => format!(
                "[{}]",
                values
                    .iter()
                    .map(Self::to_json)
                    .collect::<Vec<_>>()
                    .join(",")
            ),
            Self::Map(values) => {
                let mut values = values.iter().collect::<Vec<_>>();
                values.sort_by_key(|(key, _)| *key);
                format!(
                    "{{{}}}",
                    values
                        .into_iter()
                        .map(|(key, value)| format!("\"{}\":{}", escape_json(key), value.to_json()))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            }
        }
    }

    pub fn from_json(text: &str) -> std::result::Result<Self, String> {
        JsonParser::new(text).parse()
    }
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
        }
    }
}

impl Debug for Value {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        match self {
            null @ Value::Null => write!(f, "{null}"),
            Value::Integer(integer) => write!(f, "{integer}"),
            Value::Float(float) => write!(f, "{float}"),
            Value::Boolean(boolean) => write!(f, "{boolean}"),
            Value::String(string) => write!(f, "\"{string}\""),
            Value::Array(items) => write!(f, "{items:?}"),
            Value::Map(pairs) => write!(f, "{pairs:?}"),
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

fn escape_json(value: &str) -> String {
    let mut output = String::new();
    for character in value.chars() {
        match character {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                output.push_str(&format!("\\u{:04X}", character as u32))
            }
            character => output.push(character),
        }
    }
    output
}

struct JsonParser<'a> {
    text: &'a str,
    index: usize,
}

impl<'a> JsonParser<'a> {
    fn new(text: &'a str) -> Self {
        Self { text, index: 0 }
    }

    fn parse(mut self) -> std::result::Result<Value, String> {
        let value = self.value()?;
        self.space();
        if self.index == self.text.len() {
            Ok(value)
        } else {
            Err(self.error("unexpected trailing JSON"))
        }
    }

    fn value(&mut self) -> std::result::Result<Value, String> {
        self.space();
        match self.peek() {
            Some('n') => {
                self.word("null")?;
                Ok(Value::Null)
            }
            Some('t') => {
                self.word("true")?;
                Ok(Value::Boolean(true))
            }
            Some('f') => {
                self.word("false")?;
                Ok(Value::Boolean(false))
            }
            Some('"') => self.string().map(Value::String),
            Some('[') => self.array(),
            Some('{') => self.object(),
            Some('-' | '0'..='9') => self.number(),
            _ => Err(self.error("expected JSON value")),
        }
    }

    fn array(&mut self) -> std::result::Result<Value, String> {
        self.take('[')?;
        let mut values = Vec::new();
        self.space();
        if self.consume(']') {
            return Ok(Value::Array(values));
        }
        loop {
            values.push(self.value()?);
            self.space();
            if self.consume(']') {
                break;
            }
            self.take(',')?;
        }
        Ok(Value::Array(values))
    }

    fn object(&mut self) -> std::result::Result<Value, String> {
        self.take('{')?;
        let mut values = HashMap::new();
        self.space();
        if self.consume('}') {
            return Ok(Value::Map(values));
        }
        loop {
            self.space();
            let key = self.string()?;
            self.space();
            self.take(':')?;
            values.insert(key, self.value()?);
            self.space();
            if self.consume('}') {
                break;
            }
            self.take(',')?;
        }
        Ok(Value::Map(values))
    }

    fn string(&mut self) -> std::result::Result<String, String> {
        self.take('"')?;
        let mut value = String::new();
        loop {
            match self.next() {
                Some('"') => return Ok(value),
                Some('\\') => match self.next() {
                    Some('"') => value.push('"'),
                    Some('\\') => value.push('\\'),
                    Some('/') => value.push('/'),
                    Some('b') => value.push('\u{8}'),
                    Some('f') => value.push('\u{c}'),
                    Some('n') => value.push('\n'),
                    Some('r') => value.push('\r'),
                    Some('t') => value.push('\t'),
                    Some('u') => value.push(self.unicode()?),
                    _ => return Err(self.error("invalid JSON escape")),
                },
                Some(character) if !character.is_control() => value.push(character),
                Some(_) => return Err(self.error("control character in JSON string")),
                None => return Err(self.error("unterminated JSON string")),
            }
        }
    }

    fn unicode(&mut self) -> std::result::Result<char, String> {
        let high = self.unicode_unit()?;
        let scalar = match high {
            0xD800..=0xDBFF => {
                if !self.text[self.index..].starts_with("\\u") {
                    return Err(self.error("high surrogate must be followed by a low surrogate"));
                }
                self.index += 2;
                let low = self.unicode_unit()?;
                if !(0xDC00..=0xDFFF).contains(&low) {
                    return Err(self.error("invalid low surrogate"));
                }
                0x10000 + (((high as u32 - 0xD800) << 10) | (low as u32 - 0xDC00))
            }
            0xDC00..=0xDFFF => return Err(self.error("unexpected low surrogate")),
            value => value as u32,
        };
        char::from_u32(scalar).ok_or_else(|| self.error("invalid unicode scalar"))
    }

    fn unicode_unit(&mut self) -> std::result::Result<u16, String> {
        let start = self.index;
        let end = start + 4;
        let digits = self
            .text
            .get(start..end)
            .ok_or_else(|| self.error("invalid unicode escape"))?;
        let value =
            u16::from_str_radix(digits, 16).map_err(|_| self.error("invalid unicode escape"))?;
        self.index = end;
        Ok(value)
    }

    fn number(&mut self) -> std::result::Result<Value, String> {
        let start = self.index;
        self.consume('-');
        while self.peek().is_some_and(|c| c.is_ascii_digit()) {
            self.next();
        }
        let float = if self.consume('.') {
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.next();
            }
            true
        } else {
            false
        };
        let exponent = if matches!(self.peek(), Some('e' | 'E')) {
            self.next();
            if matches!(self.peek(), Some('+' | '-')) {
                self.next();
            }
            while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                self.next();
            }
            true
        } else {
            false
        };
        let number = &self.text[start..self.index];
        if float || exponent {
            number
                .parse()
                .map(Value::Float)
                .map_err(|_| self.error("invalid JSON number"))
        } else {
            number
                .parse()
                .map(Value::Integer)
                .map_err(|_| self.error("invalid JSON integer"))
        }
    }

    fn word(&mut self, expected: &str) -> std::result::Result<(), String> {
        if self.text[self.index..].starts_with(expected) {
            self.index += expected.len();
            Ok(())
        } else {
            Err(self.error("invalid JSON literal"))
        }
    }
    fn space(&mut self) {
        while self.peek().is_some_and(char::is_whitespace) {
            self.next();
        }
    }
    fn take(&mut self, expected: char) -> std::result::Result<(), String> {
        if self.consume(expected) {
            Ok(())
        } else {
            Err(self.error(&format!("expected '{expected}'")))
        }
    }
    fn consume(&mut self, expected: char) -> bool {
        if self.peek() == Some(expected) {
            self.next();
            true
        } else {
            false
        }
    }
    fn peek(&self) -> Option<char> {
        self.text[self.index..].chars().next()
    }
    fn next(&mut self) -> Option<char> {
        let value = self.peek()?;
        self.index += value.len_utf8();
        Some(value)
    }
    fn error(&self, message: &str) -> String {
        format!("JSON byte {}: {message}", self.index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn value_display_string() {
        let tests = vec![
            (Value::Null, "null"),
            (Value::Integer(1), "1"),
            (Value::Float(1.5), "1.5"),
            (Value::Boolean(true), "true"),
            (
                Value::String(String::from("say \"hi\"\\next\n")),
                "say \"hi\"\\next\n",
            ),
            (
                Value::Array(vec![
                    Value::String(String::from("a")),
                    Value::String(String::from("b")),
                    Value::String(String::from("c")),
                ]),
                "[\"a\", \"b\", \"c\"]",
            ),
            (
                Value::Map(HashMap::from([(
                    String::from("key"),
                    Value::String(String::from("value")),
                )])),
                "{\"key\": \"value\"}",
            ),
        ];
        for (value, string) in tests {
            assert_eq!(value.to_string(), string);
        }
    }

    #[test]
    fn value_display_json() {
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
            value.json(),
            r#"{"data":[null,1,1.5,true,"say \"hi\"\\next\n"]}"#
        );
    }

    #[test]
    fn value_parses_json_for_response_assertions() {
        let value = Value::from_json(
            r#"{"data":{"id":7},"tags":["rust","http"],"ok":true,"emoji":"\uD83D\uDE00"}"#,
        )
        .unwrap();
        let Value::Map(root) = value else {
            panic!("expected object")
        };
        let Value::Map(data) = &root["data"] else {
            panic!("expected data object")
        };
        assert_eq!(data["id"], Value::Integer(7));
        assert_eq!(root["ok"], Value::Boolean(true));
        assert_eq!(root["emoji"], Value::String("😀".into()));
    }

    #[test]
    fn value_rejects_invalid_json() {
        assert!(Value::from_json(r#"{"id":}"#).is_err());
        assert!(Value::from_json("[1] trailing").is_err());
        assert!(Value::from_json(r#""\uD83D""#).is_err());
        assert!(Value::from_json(r#""\uDE00""#).is_err());
    }
}
