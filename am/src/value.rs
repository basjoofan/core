use super::native;
use super::record::Group;
use super::record::Record;
use super::syntax::Expr;
use reqwest::Client;
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;
use std::sync::mpsc::Sender;
use uuid::Uuid;

#[derive(Clone, PartialEq)]
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
    Function(Vec<String>, Vec<Expr>),
    Native(fn(Vec<Value>) -> Value),
    Request(String, Vec<Expr>, Vec<Expr>),
}

impl Value {
    pub fn is_error(&self) -> bool {
        match self {
            Value::Error(_) => true,
            _ => false,
        }
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
            Value::Function(parameters, body) => {
                write!(
                    f,
                    "fn ({}) {{ {} }}",
                    parameters.join(", "),
                    body.iter().map(|e| e.to_string()).collect::<Vec<String>>().join(", ")
                )
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
                            format!("${{{}}}", e.to_string())
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

#[derive(Clone)]
pub struct Context {
    // TODO inner use Arc
    inner: HashMap<String, Value>,
    client: Client,
    sender: Option<Sender<Record>>,
    group: Group,
}

impl Default for Context {
    fn default() -> Self {
        let mut inner = HashMap::default();
        inner.insert(String::from("print"), Value::Native(native::print));
        inner.insert(String::from("println"), Value::Native(native::println));
        inner.insert(String::from("length"), Value::Native(native::length));
        let client = Client::new();
        let group = Group::default();
        Context {
            inner,
            client,
            sender: None,
            group,
        }
    }
}

impl Context {
    pub fn clone(parent: &Context) -> Context {
        let clone = parent.clone();
        Context {
            inner: clone.inner,
            client: clone.client,
            sender: clone.sender,
            group: clone.group,
        }
    }

    pub fn set_sender(&mut self, sender: &Sender<Record>) {
        self.sender = Some(sender.clone());
    }

    pub fn send(&mut self, record: &Record) {
        if let Some(sender) = &self.sender {
            let _ = sender.send(record.clone());
        }
    }

    pub fn set(&mut self, name: String, value: Value) {
        self.inner.borrow_mut().insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.inner.get(name) {
            return Some(value.clone());
        }
        None
    }

    pub fn client(&self) -> Client {
        self.client.clone()
    }

    pub fn set_group(&mut self, name: &String) {
        let id = Uuid::now_v7().to_string();
        let name = name.clone();
        self.group = Group { id, name };
    }

    pub fn group(&self) -> Group {
        self.group.clone()
    }
}
