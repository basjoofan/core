use crate::http::Request;
use crate::http::Response;
use crate::http::Time;
use crate::Expr;
use crate::Token;
use crate::Value;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;
use std::fs::File;
use std::path::PathBuf;

pub struct Record {
    pub name: String,
    pub time: Time,
    pub request: Request,
    pub response: Response,
    pub asserts: Vec<Assert>,
    pub error: String,
}

pub struct Assert {
    pub expression: Expr,
    pub left: Value,
    pub comparison: Token,
    pub right: Value,
    pub result: bool,
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

impl Record {
    pub fn to(self, id: String, name: String, schema: &avro::Schema) -> avro::types::Record<'_> {
        let mut record = avro::types::Record::new(schema).unwrap();
        record.put("context_id", id);
        record.put("context_name", name);
        record.put("time_start", self.time.start.as_nanos() as i64);
        record.put("time_end", self.time.end.as_nanos() as i64);
        record.put("time_total", self.time.total.as_nanos() as i64);
        record.put("time_resolve", self.time.resolve.as_nanos() as i64);
        record.put("time_connect", self.time.connect.as_nanos() as i64);
        record.put("time_write", self.time.write.as_nanos() as i64);
        record.put("time_delay", self.time.delay.as_nanos() as i64);
        record.put("time_read", self.time.read.as_nanos() as i64);
        record.put("request_name", self.name);
        record.put("request_method", self.request.method.to_string());
        record.put("request_url", self.request.url.to_string());
        record.put("request_version", self.request.version.to_string());
        record.put(
            "request_headers",
            avro::types::Value::Array(
                self.request
                    .headers
                    .into_iter()
                    .map(|header| {
                        avro::types::Value::Array(vec![
                            avro::types::Value::String(header.name),
                            avro::types::Value::String(header.value),
                        ])
                    })
                    .collect::<Vec<avro::types::Value>>(),
            ),
        );
        record.put("request_body", self.request.body);
        record.put("response_version", self.response.version);
        record.put("response_status", self.response.status as i32);
        record.put("response_reason", self.response.reason);
        record.put(
            "response_headers",
            avro::types::Value::Array(
                self.response
                    .headers
                    .into_iter()
                    .map(|header| {
                        avro::types::Value::Array(vec![
                            avro::types::Value::String(header.name),
                            avro::types::Value::String(header.value),
                        ])
                    })
                    .collect::<Vec<avro::types::Value>>(),
            ),
        );
        record.put("response_body", self.response.body);
        record.put(
            "asserts",
            avro::types::Value::Array(
                self.asserts
                    .into_iter()
                    .map(|a| {
                        avro::types::Value::Record(vec![
                            (
                                String::from("expression"),
                                avro::types::Value::String(a.expression.to_string()),
                            ),
                            (String::from("left"), avro::types::Value::String(a.left.to_string())),
                            (
                                String::from("comparison"),
                                avro::types::Value::String(a.comparison.to_string()),
                            ),
                            (String::from("right"), avro::types::Value::String(a.right.to_string())),
                            (String::from("result"), avro::types::Value::Boolean(a.result)),
                        ])
                    })
                    .collect::<Vec<avro::types::Value>>(),
            ),
        );
        record.put("error", self.error);
        record
    }
}

pub fn writer(schema: &avro::Schema, file: Option<PathBuf>) -> Option<avro::Writer<File>> {
    match file {
        Some(file) => {
            let display = file.display();
            let file = match File::create(&file) {
                Err(error) => panic!("couldn't create {}: {:?}", display, error),
                Ok(file) => file,
            };
            Some(avro::Writer::new(schema, file))
        }
        None => None,
    }
}

pub fn schema() -> avro::Schema {
    match avro::Schema::parse_str(RAW) {
        Err(error) => panic!("parse schema error: {:?}", error),
        Ok(schema) => schema,
    }
}

const RAW: &str = r#"
{
    "name": "record",
    "type": "record",
    "fields": [
        {"name": "context_id", "type": "string"},
        {"name": "context_name", "type": "string"},
        {"name": "time_start", "type": "long"},
        {"name": "time_end", "type": "long"},
        {"name": "time_total", "type": "long"},
        {"name": "time_resolve", "type": "long"},
        {"name": "time_connect", "type": "long"},
        {"name": "time_write", "type": "long"},
        {"name": "time_delay", "type": "long"},
        {"name": "time_read", "type": "long"},
        {"name": "request_name", "type": "string"},
        {"name": "request_method", "type": "string"},
        {"name": "request_url", "type": "string"},
        {"name": "request_version", "type": "string"},
        {"name": "request_headers", "type": {"type": "array", "items": {"type": "array", "items": "string"}}},
        {"name": "request_body", "type": "string"},
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

// #[test]
// fn test_record_to_record() {
//     let record = Record {
//         name: String::from("name"),
//         time: Time::default(),
//         request: Request::default(),
//         response: Response::default(),
//         asserts: vec![Assert {
//             expression: Expr::Binary(
//                 Token {
//                     kind: crate::token::Kind::Eq,
//                     literal: String::from("=="),
//                 },
//                 Some(Box::new(Expr::Ident(
//                     Token {
//                         kind: crate::token::Kind::Ident,
//                         literal: String::from("status"),
//                     },
//                     String::from("status"),
//                 ))),
//                 Some(Box::new(Expr::Integer(
//                     Token {
//                         kind: crate::token::Kind::Integer,
//                         literal: String::from("200"),
//                     },
//                     Some(200),
//                 ))),
//             ),
//             left: Value::Integer(200),
//             comparison: Token {
//                 kind: crate::token::Kind::Eq,
//                 literal: String::from("=="),
//             },
//             right: Value::Integer(200),
//             result: true,
//         }],
//         error: String::from("error"),
//     };
//     let schema = schema();
//     let avro = record.to("id".to_string(), "name".to_string(), &schema);
//     println!("avro:{:?}", avro);
//     let mut writer = avro::Writer::new(&schema, Vec::new());
//     println!("append: {:?}", writer.append(avro).unwrap());
//     println!("encoded: {:?}", writer.into_inner().unwrap());
// }
