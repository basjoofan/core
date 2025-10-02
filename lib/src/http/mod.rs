#[cfg(feature = "univ")]
mod univ;
#[cfg(feature = "wasm")]
mod wasm;

#[cfg(feature = "univ")]
mod error;
mod header;
mod mime;
mod time;
mod url;

#[cfg(feature = "univ")]
use error::Error;
pub use header::Headers;
pub use time::Time;
pub use url::Url;

use crate::Parser;
use crate::Source;
use crate::Value;
use std::collections::HashMap;

pub struct Client {
    #[cfg(feature = "univ")]
    connect_tiomeout: std::time::Duration,
    // TODO read_tiomeout: Option<std::time::Duration>,
    #[cfg(feature = "wasm")]
    fetch_timeout: u32,
    #[cfg(feature = "wasm")]
    base: String,
}

impl Client {
    #[cfg(feature = "univ")]
    pub fn new() -> Self {
        Self {
            connect_tiomeout: std::time::Duration::from_secs(120),
        }
    }

    #[cfg(feature = "wasm")]
    pub fn new(base: &str) -> Self {
        Self {
            fetch_timeout: 300_000,
            base: base.to_owned(),
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

impl Response {
    pub fn to_map(&self) -> HashMap<String, Value> {
        let mut map = HashMap::new();
        map.insert(String::from("version"), Value::String(self.version.to_string()));
        map.insert(String::from("status"), Value::Integer(self.status as i64));
        map.insert(String::from("reason"), Value::String(self.reason.to_string()));
        let mut headers: HashMap<String, Value> = HashMap::new();
        for header in self.headers.iter() {
            match headers.get_mut(&header.name) {
                Some(Value::Array(array)) => array.push(Value::String(header.value.to_string())),
                _ => {
                    headers.insert(header.name.to_string(), Value::Array(vec![Value::String(header.value.to_string())]));
                }
            }
        }
        map.insert(String::from("headers"), Value::Map(headers));
        map.insert(String::from("body"), Value::String(self.body.to_string()));
        if let Ok(Source { exprs, .. }) = Parser::new(&self.body).parse() {
            if let Some(expr) = exprs.first() {
                map.insert(String::from("json"), expr.eval());
            }
        }
        map
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
