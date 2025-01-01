use crate::Value;
use std::collections::HashMap;

#[derive(Default)]
pub struct Context {
    inner: HashMap<String, Value>,
}

impl Context {
    pub fn get(&self, name: &str) -> Option<Value> {
        match self.inner.get(name) {
            Some(value) => Some(value.to_owned()),
            None => todo!(),
        }
    }

    pub fn set(&mut self, name: &str, value: Value) -> Option<Value> {
        self.inner.insert(name.to_string(), value)
    }
}
