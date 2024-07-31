use crate::native;
use crate::record::Record;
use crate::value::Value;
use http::Client;
use std::borrow::BorrowMut;
use std::collections::HashMap;
use std::sync::mpsc::Sender;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct Context {
    // TODO inner use Arc
    inner: HashMap<String, Value>,
    client: Arc<Client>,
    sender: Option<Sender<(String, String, Record)>>,
    // TODO use Arc
    id: String,
    name: String,
}

impl Default for Context {
    fn default() -> Self {
        let mut inner = HashMap::default();
        inner.insert(String::from("print"), Value::Native(native::print));
        inner.insert(String::from("println"), Value::Native(native::println));
        inner.insert(String::from("length"), Value::Native(native::length));
        let client = Arc::new(Client::default());
        Context {
            inner,
            client,
            sender: None,
            id: String::default(),
            name: String::default(),
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
            id: clone.id,
            name: clone.name,
        }
    }

    pub fn set_sender(&mut self, sender: &Sender<(String, String, Record)>) {
        self.sender = Some(sender.clone());
    }

    pub fn send(&mut self, record: Record) {
        if let Some(sender) = &self.sender {
            let _ = sender.send((self.id.clone(), self.name.clone(), record));
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

    pub fn client(&self) -> Arc<Client> {
        self.client.clone()
    }

    pub fn set_name(&mut self, name: &str) {
        self.id = Uuid::now_v7().to_string();
        self.name = name.to_string();
    }
}
