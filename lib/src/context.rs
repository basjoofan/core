use super::HttpRequest;
use super::HttpResponse;
use super::HttpResult;
use super::HttpTiming;
use super::Value;
use std::collections::HashMap;
use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

#[derive(Default)]
pub struct Context {
    variables: HashMap<String, Value>,
    records: Vec<Record>,
    responses: Vec<HttpResult>,
    request_cursor: usize,
    pending_request: Option<HttpRequest>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            variables: HashMap::new(),
            records: Vec::new(),
            responses: Vec::new(),
            request_cursor: 0,
            pending_request: None,
        }
    }

    pub fn from(variables: HashMap<String, Value>) -> Self {
        Self {
            variables,
            records: Vec::new(),
            responses: Vec::new(),
            request_cursor: 0,
            pending_request: None,
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

    pub(crate) fn prepare_replay(&mut self, responses: Vec<HttpResult>) {
        self.responses = responses;
        self.request_cursor = 0;
        self.pending_request = None;
    }

    pub(crate) fn clone_for_replay(&self) -> Self {
        Self {
            variables: self.variables.clone(),
            records: self.records.clone(),
            responses: Vec::new(),
            request_cursor: 0,
            pending_request: None,
        }
    }

    pub(crate) fn response_for(&mut self, request: HttpRequest) -> Option<HttpResult> {
        let response = self.responses.get(self.request_cursor).cloned();
        if response.is_some() {
            self.request_cursor += 1;
        } else {
            self.pending_request = Some(request);
        }
        response
    }

    pub(crate) fn take_pending_request(&mut self) -> Option<HttpRequest> {
        self.pending_request.take()
    }
}

impl Clone for Context {
    fn clone(&self) -> Self {
        Self {
            variables: self.variables.to_owned(),
            records: Vec::new(),
            responses: Vec::new(),
            request_cursor: 0,
            pending_request: None,
        }
    }
}

#[derive(Clone)]
pub struct Record {
    pub name: String,
    pub time: HttpTiming,
    pub request: HttpRequest,
    pub response: HttpResponse,
    pub asserts: Vec<Assert>,
    pub error: String,
}

#[derive(Clone)]
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
