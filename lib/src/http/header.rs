use std::slice::Iter;

#[derive(Default, Debug)]
pub struct Headers {
    inner: Vec<Header>,
}

#[derive(Debug)]
pub struct Header {
    pub name: String,
    pub value: String,
}

impl Headers {
    pub fn insert(&mut self, name: String, value: String) {
        self.inner.push(Header { name, value });
    }

    pub fn replace(&mut self, name: String, value: String) {
        for header in self.inner.iter_mut() {
            if header.name == name {
                header.value = value.clone();
            }
        }
    }

    pub fn iter(&self) -> Iter<'_, Header> {
        self.inner.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}
