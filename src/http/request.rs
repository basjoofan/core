use super::Content;
use super::Error;
use super::Header;
use super::Headers;
use super::Method;
use super::Url;
use super::Version;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;

#[derive(Default)]
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

impl Request {
    /// Converts a message to an http request.
    pub fn from(message: &str) -> (Request, Content) {
        let mut lines = message.trim().lines();
        if let Some(line) = lines.next() {
            let mut splits = line.split_whitespace();
            let method = Method::from(splits.next());
            let url = Url::from(splits.next());
            let version = Version::from(splits.next());
            let mut content_type = None;
            let mut headers = Headers::default();
            for line in lines.by_ref() {
                if line.trim().is_empty() {
                    break;
                } else if let Some((name, value)) = line.trim().split_once(':') {
                    headers.insert(Header {
                        name: name.trim().to_string(),
                        value: value.trim().to_string(),
                    });
                    if content_type.is_none() && name.trim().to_lowercase() == "content-type" {
                        content_type = Some(value.trim());
                    }
                }
            }
            let mut body = String::default();
            let content: Content;
            match content_type {
                Some("application/x-www-form-urlencoded") => {
                    let mut serializer = form_urlencoded::Serializer::new(String::default());
                    for line in lines.by_ref() {
                        if let Some((name, value)) = line.trim().split_once(':') {
                            serializer.append_pair(name, value);
                            body.push_str(line);
                        }
                    }
                    content = Content::Byte(serializer.finish().into_bytes());
                }
                Some("multipart/form-data") => {
                    let mut parts = multipart::client::lazy::Multipart::new();
                    for line in lines.by_ref() {
                        if let Some((name, value)) = line.trim().split_once(':') {
                            let (name, value) = (name.trim(), value.trim());
                            if value.starts_with('@') {
                                parts.add_file(name, Path::new(&value[1..value.len()]));
                            } else {
                                parts.add_text(name, value);
                            }
                            body.push_str(line);
                        }
                    }
                    match parts.prepare() {
                        Ok(parts) => content = Content::Multipart(parts),
                        Err(_) => content = Content::Empty,
                    }
                }
                _ => {
                    body = String::from_iter(lines);
                    if body.trim().is_empty() {
                        content = Content::Empty;
                    } else {
                        content = Content::Byte(body.as_bytes().to_owned());
                    }
                }
            }
            (
                Request {
                    method,
                    url,
                    version,
                    headers,
                    body,
                },
                content,
            )
        } else {
            (Request::default(), Content::Empty)
        }
    }

    pub fn write<W: Write>(&mut self, writer: W, mut content: Content) -> Result<(), Error> {
        let mut writer = BufWriter::new(writer);
        write!(writer, "{} {} {}\r\n", self.method, self.url.path, self.version).map_err(Error::WriteFailed)?;
        for header in self.headers.iter() {
            write!(writer, "{}: {}\r\n", header.name, header.value).map_err(Error::WriteFailed)?;
        }
        write!(writer, "\r\n").map_err(Error::WriteFailed)?;
        content.write(&mut writer)?;
        writer.flush().map_err(|_e| Error::WriteFlushFailed)?;
        Ok(())
    }
}

#[test]
fn test_from_message_get() {
    let message = r#"
    GET http://httpbin.org/get
    Host: httpbin.org"#;
    let (request, _content) = Request::from(message);
    assert_eq!("GET", request.method.as_ref());
}
