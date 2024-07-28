use crate::Content;
use crate::Error;
use crate::Header;
use crate::Headers;
use crate::Method;
use crate::Version;
use std::io::BufWriter;
use std::io::Write;
use std::path::Path;
use url::Url;

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
    pub fn from(message: &str) -> Result<(Request, Content), Error> {
        let mut lines = message.trim().lines();
        if let Some(line) = lines.next() {
            let mut splits = line.split_whitespace();
            let method = Method::from(splits.next());
            let url = match splits.next() {
                Some(url) => match Url::parse(url) {
                    Ok(url) => Ok(url),
                    Err(error) => Err(Error::from(error)),
                },
                None => Err(Error::EmptyUrl),
            }?;
            let version = Version::from(splits.next());
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
            let mut body = String::default();
            let content: Content;
            match content_type {
                Some("application/x-www-form-urlencoded") => {
                    let mut serializer = form_urlencoded::Serializer::new(String::default());
                    while let Some(line) = lines.next() {
                        if let Some((name, value)) = line.trim().split_once(':') {
                            serializer.append_pair(name, value);
                            body.push_str(line);
                        }
                    }
                    content = Content::Byte(serializer.finish().into_bytes());
                }
                Some("multipart/form-data") => {
                    let mut parts = multipart::client::lazy::Multipart::new();
                    while let Some(line) = lines.next() {
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
                    content = Content::Multipart(parts.prepare().map_err(|_e| Error::MultipartPrepareFailed)?);
                }
                _ => {
                    body = String::from_iter(lines);
                    if body.trim().is_empty() {
                        content = Content::Empty;
                    } else {
                        content = Content::Byte(body.clone().into_bytes());
                    }
                }
            }
            Ok((
                Request {
                    method,
                    url,
                    version,
                    headers,
                    body,
                },
                content,
            ))
        } else {
            Err(Error::EmptyMessage)
        }
    }

    pub fn write<W: Write>(&mut self, writer: W, mut content: Content) -> Result<(), Error> {
        let mut writer = BufWriter::new(writer);
        write!(writer, "{} {}", self.method, self.url.path()).map_err(|e| Error::WriteFailed(e))?;
        if let Some(query) = self.url.query() {
            write!(writer, "?{}", query).map_err(|e| Error::WriteFailed(e))?;
        }
        write!(writer, " {}\r\n", self.version).map_err(|e| Error::WriteFailed(e))?;

        for header in self.headers.iter() {
            write!(writer, "{}: {}\r\n", header.name, header.value).map_err(|e| Error::WriteFailed(e))?;
        }
        write!(writer, "\r\n").map_err(|e| Error::WriteFailed(e))?;
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
    let (request, _content) = Request::from(message).unwrap();
    assert_eq!("GET", request.method.as_ref());
}
