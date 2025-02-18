use std::collections::HashMap;
use std::slice::Iter;

#[derive(Default, Debug)]
pub struct Headers {
    indices: HashMap<String, Vec<usize>>,
    entries: Vec<Header>,
}

#[derive(Debug)]
pub struct Header {
    pub name: String,
    pub value: String,
}

impl Headers {
    pub fn insert(&mut self, name: String, value: String) {
        self.indices.entry(name.to_lowercase()).or_default().push(self.entries.len());
        self.entries.push(Header { name, value });
    }

    pub fn replace(&mut self, name: &str, value: String) {
        if let Some(indices) = self.indices.get(name.to_lowercase().as_str()) {
            if let Some(index) = indices.first() {
                if let Some(header) = self.entries.get_mut(*index) {
                    header.value = value;
                }
            }
        }
    }

    pub fn iter(&self) -> Iter<'_, Header> {
        self.entries.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }
}
