mod client;
mod content;
mod error;
mod header;
mod request;
mod response;
mod stream;
mod time;
mod url;

use content::Content;
use content::Part;
use error::Error;
use header::Headers;
use stream::Stream;
use url::Url;

pub use client::Client;
pub use request::Request;
pub use response::Response;
pub use time::Time;

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
