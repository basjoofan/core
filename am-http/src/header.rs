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

    pub fn iter(&mut self) -> std::slice::Iter<'_, Header> {
        self.inner.iter()
    }

    pub fn len(&self) -> usize {
        self.inner.len()
    }
}
