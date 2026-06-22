use std::time::Duration;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HttpHeader {
    pub name: String,
    pub value: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HttpRequest {
    pub method: String,
    pub url: String,
    pub headers: Vec<HttpHeader>,
    pub body: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HttpResponse {
    pub version: String,
    pub status: u16,
    pub reason: String,
    pub headers: Vec<HttpHeader>,
    pub body: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct HttpTiming {
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
pub struct HttpResult {
    pub request: HttpRequest,
    pub response: HttpResponse,
    pub timing: HttpTiming,
    pub error: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PendingRequest {
    pub id: u64,
    pub request: HttpRequest,
}

pub(crate) fn form_urlencode(pairs: &[(String, String)]) -> String {
    pairs
        .iter()
        .map(|(name, value)| format!("{}={}", percent_encode(name), percent_encode(value)))
        .collect::<Vec<_>>()
        .join("&")
}

fn percent_encode(value: &str) -> String {
    let mut output = String::new();
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'*' | b'-' | b'.' | b'_' => {
                output.push(byte as char)
            }
            _ => output.push_str(&format!("%{byte:02X}")),
        }
    }
    output
}
