use std::slice::Iter;
use std::vec::IntoIter;

#[derive(Default)]
pub struct Headers {
    inner: Vec<Header>,
}

pub struct Header {
    pub name: String,
    pub value: String,
}

impl Headers {
    pub fn insert(&mut self, header: Header) {
        self.inner.push(header)
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

impl IntoIterator for Headers {
    type Item = Header;
    type IntoIter = IntoIter<Header>;
    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}
