use crate::native;
use crate::value::Value;
use std::borrow::BorrowMut;
use std::collections::HashMap;

#[derive(Clone)]
pub struct Context<'a> {
    inner: HashMap<String, Value>,
    outer: Option<&'a Context<'a>>,
}

impl Default for Context<'_> {
    fn default() -> Self {
        let mut inner = HashMap::default();
        inner.insert(String::from("print"), Value::Native(native::print));
        inner.insert(String::from("println"), Value::Native(native::println));
        inner.insert(String::from("length"), Value::Native(native::length));
        Context {
            inner: inner,
            outer: None,
        }
    }
}

impl Context<'_> {
    pub fn new<'a>(parent: &'a Context<'a>) -> Context<'a> {
        Context {
            inner: HashMap::default(),
            outer: Some(parent),
        }
    }

    pub fn set(&mut self, name: String, value: Value) {
        self.inner.borrow_mut().insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.inner.get(name) {
            Some(value.clone())
        } else {
            if let Some(outer) = self.outer {
                outer.get(name)
            } else {
                None
            }
        }
    }
}
