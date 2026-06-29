mod client;
mod content;
mod error;
mod header;
mod mime;
mod request;
mod response;
mod stream;
mod time;
mod url;

use content::Content;
use content::Part;
use error::Error;
pub use header::Headers;
use stream::Stream;
pub use time::Time;
pub use url::Serializer;
pub use url::Url;

#[derive(Default)]
pub struct Client {
    connect_tiomeout: std::time::Duration,
    // TODO read_tiomeout: Option<std::time::Duration>,
}

impl Client {
    pub fn new() -> Self {
        Self {
            connect_tiomeout: std::time::Duration::from_secs(120),
        }
    }
}

#[derive(Default, Debug)]
pub struct Request {
    /// The request's method
    pub method: Method,
    /// The request's url
    pub url: Url,
    /// The request's version
    pub version: Version,
    /// The request's headers
    pub headers: Headers,
    /// The request's body
    pub body: String,
}

#[derive(Default, Debug)]
pub struct Response {
    /// The response's version
    pub version: String,
    /// The response's status
    pub status: u16,
    /// The response's reason
    pub reason: String,
    /// The response's headers
    pub headers: Headers,
    /// The response's body
    pub body: String,
}

pub async fn send(request: lib::HttpRequest) -> lib::HttpResult {
    let mut message = format!("{} {}\n", request.method, request.url);
    for header in &request.headers {
        message.push_str(&format!("{}: {}\n", header.name, header.value));
    }
    if let Some(body) = &request.body {
        message.push('\n');
        message.push_str(body);
    }
    let (sent, response, timing, error) = Client::new().send_message(&message).await;
    let _ = sent;
    lib::HttpResult {
        request,
        response: lib::HttpResponse {
            version: response.version,
            status: response.status,
            reason: response.reason,
            headers: response
                .headers
                .iter()
                .map(|header| lib::HttpHeader {
                    name: header.name.clone(),
                    value: header.value.clone(),
                })
                .collect(),
            body: response.body,
        },
        timing: lib::HttpTiming {
            start: timing.start,
            end: timing.end,
            total: timing.total,
            resolve: timing.resolve,
            connect: timing.connect,
            write: timing.write,
            delay: timing.delay,
            read: timing.read,
        },
        error,
    }
}

#[cfg(test)]
pub mod tests {
    use axum::Router;
    use axum::extract::{Form, Json, Multipart, Query};
    use axum::http::header::HeaderMap;
    use axum::routing::{get, post};
    use serde_json::{Value, json};
    use std::collections::HashMap;
    use std::net::ToSocketAddrs;
    use tokio::net::TcpListener;

    pub async fn start_server(port: u16) {
        let router = Router::new()
            .route("/get", get(handle_get))
            .route("/text", post(handle_text))
            .route("/json", post(handle_json))
            .route("/form", post(handle_form))
            .route("/multipart", post(handle_multipart));
        for addr in ("localhost", port).to_socket_addrs().unwrap() {
            let listener = TcpListener::bind(addr).await.unwrap();
            let router = router.clone();
            tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        }
    }

    fn headers(headers: HeaderMap) -> HashMap<String, String> {
        headers
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_str().unwrap().to_string()))
            .collect()
    }

    async fn handle_get(
        headers_: HeaderMap,
        Query(params): Query<HashMap<String, String>>,
    ) -> Json<Value> {
        Json(json!({ "headers": headers(headers_), "params": params }))
    }

    async fn handle_text(
        headers_: HeaderMap,
        Query(params): Query<HashMap<String, String>>,
        text: String,
    ) -> Json<Value> {
        Json(json!({ "headers": headers(headers_), "params": params, "text": text }))
    }

    async fn handle_json(
        headers_: HeaderMap,
        Query(params): Query<HashMap<String, String>>,
        Json(value): Json<Value>,
    ) -> Json<Value> {
        Json(json!({ "headers": headers(headers_), "params": params, "json": value }))
    }

    async fn handle_form(
        headers_: HeaderMap,
        Query(params): Query<HashMap<String, String>>,
        Form(form): Form<HashMap<String, String>>,
    ) -> Json<Value> {
        Json(json!({ "headers": headers(headers_), "params": params, "form": form }))
    }

    async fn handle_multipart(
        headers_: HeaderMap,
        Query(params): Query<HashMap<String, String>>,
        mut multipart: Multipart,
    ) -> Json<Value> {
        let mut form = HashMap::new();
        while let Some(field) = multipart.next_field().await.unwrap() {
            let name = field.name().unwrap().to_string();
            let mut data = field
                .file_name()
                .map(|file| format!("@{file}|"))
                .unwrap_or_default();
            data.push_str(&field.text().await.unwrap_or_default());
            form.insert(name, data);
        }
        Json(json!({ "headers": headers(headers_), "params": params, "form": form }))
    }
}

macro_rules! http_type {
    ($type: ident $(,$name: ident => $value: literal)+) => {
        #[derive(Default, Eq, PartialEq)]
        #[non_exhaustive]
        pub enum $type {
            #[default]
            $(
                $name,
            )+
        }

        impl std::convert::From<&str> for $type {
            fn from(str: &str) -> Self{
                match str {
                    $(
                        $value => $type::$name,
                    )+
                    _ => Self::default(),
                }
            }
        }

        impl std::convert::From<Option<&str>> for $type {
            fn from(str: Option<&str>) -> Self{
                match str {
                    Some(str) => Self::from(str),
                    None => Self::default(),
                }
            }
        }

        impl std::convert::AsRef<str> for $type {
            fn as_ref(&self) -> &str {
                match self {
                    $(
                        $type::$name => $value,
                    )+
                }
            }
        }

        impl std::fmt::Debug for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_ref())
            }
        }

        impl std::fmt::Display for $type {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_ref())
            }
        }
    }
}

http_type! {
    Version,
    Http11 => "HTTP/1.1",
    Http09 => "HTTP/0.9",
    Http10 => "HTTP/1.0",
    Http20 => "HTTP/2.0",
    Http30 => "HTTP/3.0"
}

http_type! {
    Method,
    Get => "GET",
    Post => "POST",
    Put => "PUT",
    Patch => "PATCH",
    Delete => "DELETE",
    Options => "OPTIONS",
    Head => "HEAD",
    Trace => "TRACE",
    Connect => "CONNECT"
}

http_type! {
    Scheme,
    Http => "http",
    Https => "https"
    // Ws => "ws",
    // Wss => "wss"
}

#[test]
fn test_method_from_str() {
    let method = Method::from("GET");
    println!("method:{method}");
}

#[test]
fn test_version_from_str() {
    let version = Version::from("HTTP/2.0");
    println!("version:{version}");
}
