mod client;
mod content;
mod error;
mod header;
mod request;
mod response;
mod stream;

pub use crate::client::Client;
pub use crate::client::Time;
pub use crate::content::Content;
pub use crate::error::Error;
pub use crate::header::Header;
pub use crate::header::Headers;
pub use crate::request::Request;
pub use crate::response::Response;
pub use crate::stream::Stream;

macro_rules! http_type {
    ($type: ident $(,$name: ident => $value: literal)+) => {
        #[derive(Default)]
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

#[test]
fn test_method_from_str() {
    let method = Method::from("GET");
    println!("method:{}", method);
}

#[test]
fn test_version_from_str() {
    let version = Version::from("HTTP/2.0");
    println!("version:{}", version)
}