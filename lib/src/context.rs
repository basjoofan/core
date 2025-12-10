use super::Value;
use super::http::Request;
use super::http::Response;
use super::http::Time;
use std::collections::HashMap;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

#[derive(Default)]
pub struct Context {
    variables: HashMap<String, Value>,
    records: Vec<Record>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            records: Vec::new(),
        }
    }

    pub fn from(variables: HashMap<String, Value>) -> Self {
        Self {
            variables,
            records: Vec::new(),
        }
    }

    pub fn get(&self, key: &str) -> Option<&Value> {
        self.variables.get(key)
    }

    pub fn set(&mut self, key: String, value: Value) {
        self.variables.insert(key, value);
    }

    pub fn push(&mut self, record: Record) {
        self.records.push(record);
    }

    pub fn into_map(self) -> HashMap<String, Value> {
        self.variables
    }

    pub fn records(&mut self) -> Vec<Record> {
        std::mem::take(&mut self.records)
    }
}

impl Clone for Context {
    fn clone(&self) -> Self {
        Self {
            variables: self.variables.to_owned(),
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
        if !self.error.trim().is_empty() {
            writeln!(f, "ERROR: {}", self.error)?;
            return Ok(());
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
