use std::time::Duration;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Header {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Request {
    pub method: String,
    pub url: String,
    pub headers: Vec<Header>,
    pub body: Option<Content>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Content {
    Text(String),
    File(String),
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Response {
    pub version: String,
    pub status: u16,
    pub reason: String,
    pub headers: Vec<Header>,
    pub body: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Timing {
    pub start: Duration,
    pub end: Duration,
    pub total: Duration,
    pub resolve: Duration,
    pub connect: Duration,
    pub write: Duration,
    pub delay: Duration,
    pub read: Duration,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Result {
    pub request: Request,
    pub response: Response,
    pub timing: Timing,
    pub error: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pending {
    pub id: u64,
    pub request: Request,
}
