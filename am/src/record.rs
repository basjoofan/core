use super::evaluator::eval;
use super::parser::Parser;
use super::syntax::Expr;
use super::token::Token;
use super::value::Context;
use super::value::Value;
use std::collections::HashMap;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;
use std::fs::File;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Clone)]
pub struct Record {
    pub group: Group,
    pub duration: Duration,
    pub request: Request,
    pub response: Response,
    pub asserts: Vec<Assert>,
}

#[derive(Clone, Default)]
pub struct Group {
    pub id: String,
    pub name: String,
}

#[derive(Clone)]
pub struct Request {
    pub id: String,
    pub name: String,
    pub method: String,
    pub url: String,
    pub version: String,
    pub headers: Vec<(String, String)>,
    pub fields: Vec<(String, String)>,
    pub content: String,
}

#[derive(Clone)]
pub struct Response {
    pub version: String,
    pub status: u16,
    pub reason: String,
    pub headers: Vec<(String, String)>,
    pub body: String,
}

#[derive(Clone)]
pub struct Assert {
    pub expression: Expr,
    pub left: Value,
    pub comparison: Token,
    pub right: Value,
    pub result: bool,
}

impl Record {
    pub fn to_value(&self) -> Value {
        let mut map = HashMap::new();
        map.insert(String::from("group"), self.group.to_value());
        map.insert(
            String::from("duration"),
            Value::Integer(self.duration.as_nanos() as i64),
        );
        map.insert(String::from("request"), self.request.to_value());
        map.insert(String::from("response"), self.response.to_value());
        map.insert(
            String::from("asserts"),
            Value::Array(self.asserts.iter().map(|a| a.to_value()).collect::<Vec<Value>>()),
        );
        Value::Map(map)
    }

    pub fn to_record<'a>(&'a self, schema: &'a apache_avro::Schema) -> apache_avro::types::Record {
        let mut record = apache_avro::types::Record::new(schema).unwrap();
        record.put("group_id", String::from("98765"));
        record.put("group_name", String::from("fn"));
        record.put("start_time", self.duration.as_nanos() as i64);
        record.put("end_time", self.duration.as_nanos() as i64);
        record.put("duration", self.duration.as_nanos() as i64);
        record.put("request_id", String::from("98765"));
        record.put("request_name", String::from("rq"));
        record.put("request_method", self.request.method.clone());
        record.put("request_url", self.request.url.clone());
        record.put("request_version", self.request.version.clone());
        record.put("request_headers", pairs_to_record(&self.request.headers));
        record.put("request_fields", pairs_to_record(&self.request.fields));
        record.put("request_content", self.request.content.clone());
        record.put("response_version", self.response.version.clone());
        record.put("response_status", self.response.status as i32);
        record.put("response_reason", self.response.reason.clone());
        record.put("response_headers", pairs_to_record(&self.response.headers));
        record.put("response_body", self.response.body.clone());
        record.put(
            "asserts",
            apache_avro::types::Value::Array(
                self.asserts
                    .iter()
                    .map(|a| {
                        apache_avro::types::Value::Record(vec![
                            (
                                String::from("expression"),
                                apache_avro::types::Value::String(a.expression.to_string()),
                            ),
                            (
                                String::from("left"),
                                apache_avro::types::Value::String(a.left.to_string()),
                            ),
                            (
                                String::from("comparison"),
                                apache_avro::types::Value::String(a.comparison.to_string()),
                            ),
                            (
                                String::from("right"),
                                apache_avro::types::Value::String(a.right.to_string()),
                            ),
                            (String::from("result"), apache_avro::types::Value::Boolean(a.result)),
                        ])
                    })
                    .collect::<Vec<apache_avro::types::Value>>(),
            ),
        );
        record.put("error", String::from("error"));
        record
    }
}

impl Group {
    pub fn to_value(&self) -> Value {
        let mut map = HashMap::new();
        map.insert(String::from("id"), Value::String(self.id.clone()));
        map.insert(String::from("name"), Value::String(self.name.clone()));
        Value::Map(map)
    }
}

impl Request {
    pub fn to_value(&self) -> Value {
        let mut map = HashMap::new();
        map.insert(String::from("id"), Value::String(self.id.clone()));
        map.insert(String::from("name"), Value::String(self.name.clone()));
        map.insert(String::from("method"), Value::String(self.method.clone()));
        map.insert(String::from("url"), Value::String(self.url.clone()));
        map.insert(String::from("version"), Value::String(self.version.clone()));
        let mut headers: HashMap<String, Value> = HashMap::new();
        for (key, value) in self.headers.clone() {
            match headers.get_mut(&key) {
                Some(Value::Array(array)) => array.push(Value::String(value)),
                _ => {
                    headers.insert(key, Value::Array(vec![Value::String(value)]));
                }
            }
        }
        map.insert(String::from("headers"), Value::Map(headers));
        let mut fields: HashMap<String, Value> = HashMap::new();
        for (key, value) in self.fields.clone() {
            match fields.get_mut(&key) {
                Some(Value::Array(array)) => array.push(Value::String(value)),
                _ => {
                    fields.insert(key, Value::Array(vec![Value::String(value)]));
                }
            }
        }
        map.insert(String::from("fields"), Value::Map(fields));
        map.insert(String::from("content"), Value::String(self.content.clone()));
        Value::Map(map)
    }
}

impl Assert {
    pub fn to_value(&self) -> Value {
        let mut map = HashMap::new();
        map.insert(String::from("expression"), Value::String(self.expression.to_string()));
        map.insert(String::from("left"), self.left.clone());
        map.insert(String::from("comparison"), Value::String(self.comparison.to_string()));
        map.insert(String::from("right"), self.right.clone());
        map.insert(String::from("result"), Value::Boolean(self.result));
        Value::Map(map)
    }
}

impl Response {
    pub fn to_value(&self) -> Value {
        let mut map = HashMap::new();
        map.insert(String::from("version"), Value::String(self.version.clone()));
        map.insert(String::from("status"), Value::Integer(self.status as i64));
        map.insert(String::from("reason"), Value::String(self.reason.clone()));
        let mut headers: HashMap<String, Value> = HashMap::new();
        for (key, value) in self.headers.clone() {
            match headers.get_mut(&key) {
                Some(Value::Array(array)) => array.push(Value::String(value)),
                _ => {
                    headers.insert(key, Value::Array(vec![Value::String(value)]));
                }
            }
        }
        map.insert(String::from("headers"), Value::Map(headers));
        map.insert(String::from("body"), Value::String(self.body.clone()));
        let json = eval(&Parser::new(&self.body).parse(), &mut Context::default());
        map.insert(String::from("json"), json);
        Value::Map(map)
    }
}

impl Display for Record {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "{}", self.group)?;
        writeln!(f, "duration:{:?}", self.duration)?;
        write!(f, "request:\n{}", self.request)?;
        write!(f, "response:\n{}", self.response)?;
        writeln!(f, "asserts:")?;
        for assert in &self.asserts {
            writeln!(f, "{}", assert)?;
        }
        Ok(())
    }
}

impl Display for Group {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "group({}, {})", self.name, self.id)
    }
}

impl Display for Request {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "{} {} {}", self.method, self.url, self.version)?;
        for (key, value) in &self.headers {
            writeln!(f, "{}: {}", key, value)?;
        }
        writeln!(f, "")?;
        for (key, value) in &self.fields {
            writeln!(f, "{}: {}", key, value)?;
        }
        if !self.content.is_empty() {
            write!(f, "{}", &self.content)?;
        }
        Ok(())
    }
}

impl Display for Response {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "{} {} {}", self.version, self.status, self.reason)?;
        for (key, value) in &self.headers {
            writeln!(f, "{}: {}", key, value)?;
        }
        write!(f, "{}", self.body)
    }
}

impl Display for Assert {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{} => ({} {} {}) => {}",
            self.expression, self.left, self.comparison, self.right, self.result
        )
    }
}

fn pairs_to_record(pairs: &Vec<(String, String)>) -> apache_avro::types::Value {
    apache_avro::types::Value::Array(
        pairs
            .iter()
            .map(|(k, v)| {
                apache_avro::types::Value::Array(vec![
                    apache_avro::types::Value::String(k.clone()),
                    apache_avro::types::Value::String(v.clone()),
                ])
            })
            .collect::<Vec<apache_avro::types::Value>>(),
    )
}

pub fn writer(schema: &apache_avro::Schema, file: Option<PathBuf>) -> Option<apache_avro::Writer<File>> {
    match file {
        Some(file) => {
            let display = file.display();
            let file = match File::create(&file) {
                Err(error) => panic!("couldn't create {}: {:?}", display, error),
                Ok(file) => file,
            };
            Some(apache_avro::Writer::with_codec(
                schema,
                file,
                apache_avro::Codec::Zstandard,
            ))
        }
        None => None,
    }
}

pub fn schema() -> apache_avro::Schema {
    match apache_avro::Schema::parse_str(RECORD_SCHEMA) {
        Err(error) => panic!("parse schema error: {:?}", error),
        Ok(schema) => schema,
    }
}

const RECORD_SCHEMA: &str = r#"
{
    "name": "record",
    "type": "record",
    "fields": [
        {"name": "group_id", "type": "string", "logicalType": "uuid"},
        {"name": "group_name", "type": "string"},
        {"name": "start_time", "type": "long", "logicalType": "timestamp-micros"},
        {"name": "end_time", "type": "long", "logicalType": "timestamp-micros"},
        {"name": "duration", "type": "long"},
        {"name": "request_id", "type": "string", "logicalType": "uuid"},
        {"name": "request_name", "type": "string"},
        {"name": "request_method", "type": "string"},
        {"name": "request_url", "type": "string"},
        {"name": "request_version", "type": "string"},
        {"name": "request_headers", "type": {"type": "array", "items": {"type": "array", "items": "string"}}},
        {"name": "request_fields", "type": {"type": "array", "items": {"type": "array", "items": "string"}}},
        {"name": "request_content", "type": "string"},
        {"name": "response_version", "type": "string"},
        {"name": "response_status", "type": "int"},
        {"name": "response_reason", "type": "string"},
        {"name": "response_headers", "type": {"type": "array", "items": {"type": "array", "items": "string"}}},
        {"name": "response_body", "type": "string"},
        {"name": "asserts", "type":
            {
                "type": "array",
                "items": {
                    "name": "assert",
                    "type": "record",
                    "fields": [
                        {"name": "expression", "type": "string"},
                        {"name": "left", "type": "string"},
                        {"name": "comparison", "type": "string"},
                        {"name": "right", "type": "string"},
                        {"name": "result", "type": "boolean"}
                    ]
                }
            }
        },
        {"name": "error", "type": "string"}
    ]
}
"#;

#[test]
fn test_record_to_record() {
    let record = Record {
        group: Group {
            id: String::from("id"),
            name: String::from("name"),
        },
        duration: Duration::from_nanos(1_000_000_123),
        request: Request {
            id: String::from("id"),
            name: String::from("name"),
            method: String::from("GET"),
            url: String::from("http://httpbin.org/get"),
            version: String::from("HTTP/1.1"),
            headers: vec![(String::from("host"), String::from("httpbin.org"))],
            fields: vec![],
            content: String::from(""),
        },
        response: Response {
            version: String::from("HTTP/1.1"),
            status: 200,
            reason: String::from("OK"),
            headers: vec![
                (String::from("content-type"), String::from("application/json")),
                (String::from("access-control-allow-origin"), String::from("*")),
            ],
            body: String::from("This is body"),
        },
        asserts: vec![Assert {
            expression: Expr::Binary(
                Token {
                    kind: super::token::Kind::Eq,
                    literal: String::from("=="),
                },
                Some(Box::new(Expr::Ident(
                    Token {
                        kind: super::token::Kind::Ident,
                        literal: String::from("status"),
                    },
                    String::from("status"),
                ))),
                Some(Box::new(Expr::Integer(
                    Token {
                        kind: super::token::Kind::Integer,
                        literal: String::from("200"),
                    },
                    Some(200),
                ))),
            ),
            left: Value::Integer(200),
            comparison: Token {
                kind: super::token::Kind::Eq,
                literal: String::from("=="),
            },
            right: Value::Integer(200),
            result: true,
        }],
    };
    let schema = apache_avro::Schema::parse_str(RECORD_SCHEMA).unwrap();
    println!("{:?}", record.to_record(&schema));
    let mut writer = apache_avro::Writer::new(&schema, Vec::new());
    println!("append: {:?}", writer.append(record.to_record(&schema)).unwrap());
    println!("encoded: {:?}", writer.into_inner().unwrap());
}
