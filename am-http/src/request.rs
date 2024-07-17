// use crate::Body;
use crate::Error;
use crate::Header;
use crate::Headers;
use crate::Method;
use crate::Version;
use url::Url;

#[derive(Debug)]
pub struct Request {
    /// The request's method
    pub method: Method,
    /// The request's url
    pub url: Url,
    /// The request's version
    pub version: Version,
    /// The request's headers
    pub headers: Headers,
    // /// The request's body
    // pub body: dyn Body,
}

/// Converts a message to an http request.
impl TryFrom<&str> for Request {
    type Error = Error;
    fn try_from(message: &str) -> Result<Request, Error> {
        let mut lines = message.trim().lines();
        if let Some(first) = lines.next() {
            let mut splits = first.split_whitespace();
            let method = match splits.next() {
                Some(method) => Method::try_from(method)?,
                None => Method::default(),
            };
            let url = match splits.next() {
                Some(url) => match Url::parse(url) {
                    Ok(url) => Ok(url),
                    Err(error) => Err(Error::from(error)),
                },
                None => Err(Error::EmptyUrl),
            }?;
            let version = match splits.next() {
                Some(version) => Version::try_from(version)?,
                None => Version::default(),
            };
            let mut content_type = None;
            let mut headers = Headers::default();
            while let Some(line) = lines.next() {
                if line.trim().is_empty() {
                    break;
                } else {
                    if let Some((name, value)) = line.trim().split_once(':') {
                        headers.insert(Header {
                            name: name.trim().to_string(),
                            value: value.trim().to_string(),
                        });
                        if content_type.is_none() && name.trim().to_lowercase() == "content-type" {
                            content_type = Some(value.trim());
                        }
                    }
                }
            }
            Ok(Request {
                method,
                url,
                version,
                headers,
            })
        } else {
            Err(Error::EmptyMessage)
        }
    }
}

#[test]
fn test_from_message_get() {
    let message = r#"
    GET http://httpbin.org/get
    Host: httpbin.org"#;
    let request = Request::try_from(message);
    println!("request:{:?}", request)
}
