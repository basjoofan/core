use crate::http::Request;
use crate::http::Response;
use crate::http::Time;
use crate::Expr;
use crate::Value;
use std::collections::HashMap;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;
use std::sync::Arc;

#[derive(Default)]
pub struct Context {
    inner: HashMap<String, Value>,
    requests: Arc<HashMap<String, (String, Vec<Expr>)>>,
    records: Vec<Record>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
            requests: Arc::new(HashMap::new()),
            records: Vec::new(),
        }
    }

    pub fn from(inner: HashMap<String, Value>) -> Self {
        Self {
            inner,
            requests: Arc::new(HashMap::new()),
            records: Vec::new(),
        }
    }

    pub fn extend(&mut self, requests: HashMap<String, (String, Vec<Expr>)>) {
        if let Some(inner) = Arc::get_mut(&mut self.requests) {
            inner.extend(requests);
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.inner.get(key)
    }

    pub fn set(&mut self, key: String, value: Value) {
        self.inner.insert(key, value);
    }

    pub fn request(&self, name: &str) -> Option<&(String, Vec<Expr>)> {
        self.requests.get(name)
    }

    pub fn push(&mut self, record: Record) {
        self.records.push(record);
    }

    pub fn into_map(self) -> HashMap<String, Value> {
        self.inner
    }

    pub fn records(&mut self) -> Vec<Record> {
        std::mem::take(&mut self.records)
    }
}

impl Clone for Context {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            requests: self.requests.clone(),
            records: Vec::new(),
        }
    }
}

pub struct Record {
    pub name: String,
    pub time: Time,
    pub request: Request,
    pub response: Response,
    pub asserts: Vec<Assert>,
    pub error: String,
}

pub struct Assert {
    pub expr: String,
    pub left: String,
    pub compare: String,
    pub right: String,
    pub result: bool,
}

impl Display for Assert {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(
            f,
            "{} => ({} {} {}) => {}",
            self.expr, self.left, self.compare, self.right, self.result
        )
    }
}

impl Display for Record {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        writeln!(f, "=== TEST  {}", self.name)?;
        let mut flag = true;
        for assert in self.asserts.iter() {
            flag &= assert.result;
            writeln!(
                f,
                "{} => {} {} {} => {}",
                assert.expr, assert.left, assert.compare, assert.right, assert.result
            )?
        }
        write!(
            f,
            "--- {}  {} ({:?})",
            match flag {
                true => "PASS",
                false => "FAIL",
            },
            self.name,
            self.time.total
        )
    }
}
