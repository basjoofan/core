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
    pub fn insert(&mut self, header: Header) {
        self.inner.push(header)
    }
}
