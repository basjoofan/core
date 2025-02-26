use super::Content;
use super::Headers;
use super::Method;
use super::Part;
use super::Url;
use super::Version;
use rand::Rng;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;

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

impl Request {
    /// Converts a message to an http request.
    pub async fn from(message: &str) -> Result<(Request, Content), std::io::Error> {
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
                    headers.insert(name.trim().to_string(), value.trim().to_string());
                    if content_type.is_none() && name.trim().to_lowercase() == "content-type" {
                        content_type = Some(value.trim());
                    }
                }
            }
            let mut body = String::default();
            let content: Content;
            let mut length = 0;
            match content_type {
                Some("application/x-www-form-urlencoded") => {
                    let mut serializer = form_urlencoded::Serializer::new(String::default());
                    for line in lines.by_ref() {
                        if let Some((name, value)) = line.trim().split_once(':') {
                            serializer.append_pair(name, value);
                            body.push_str(line);
                        }
                    }
                    let bytes = serializer.finish().into_bytes();
                    length = bytes.len();
                    content = Content::Bytes(bytes);
                }
                Some("multipart/form-data") => {
                    let mut boundary = String::from("FormDataBoundary");
                    let rng = rand::rng();
                    boundary.extend(rng.sample_iter(rand::distr::Alphanumeric).take(boundary.len()).map(char::from));
                    headers.replace("content-type", format!("multipart/form-data; boundary={}", boundary));
                    let mut parts = Vec::new();
                    for line in lines.by_ref() {
                        if let Some((name, value)) = line.trim().split_once(':') {
                            let (name, value) = (name.trim(), value.trim());
                            let mut bytes = Vec::new();
                            bytes.append(&mut format!("--{}\r\n", boundary).into_bytes());
                            if value.starts_with("@") {
                                let path = Path::new(&value[1..value.len()]);
                                let file = File::open(path).await?;
                                let metadata = file.metadata().await?;
                                bytes.append(
                                    &mut format!(
                                        "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\n",
                                        name,
                                        path.display()
                                    )
                                    .into_bytes(),
                                );
                                if let Some(mime) = mime_guess::from_path(path).first() {
                                    bytes.append(&mut format!("Content-Type: {}\r\n\r\n", mime).into_bytes());
                                };
                                length += bytes.len() + metadata.len() as usize + 2;
                                parts.push(Part::Bytes(bytes));
                                parts.push(Part::File(file));
                                parts.push(Part::Bytes(vec![b'\r', b'\n']));
                            } else {
                                bytes.append(
                                    &mut format!("Content-Disposition: form-data; name=\"{}\"\r\n\r\n{}\r\n", name, value).into_bytes(),
                                );
                                length += bytes.len();
                                parts.push(Part::Bytes(bytes));
                            }
                            body.push_str(line);
                        }
                    }
                    let bytes = format!("--{}--\r\n", boundary).into_bytes();
                    length += bytes.len();
                    parts.push(Part::Bytes(bytes));
                    content = Content::Parts(parts)
                }
                _ => {
                    body = String::from_iter(lines);
                    if body.trim().is_empty() {
                        content = Content::Empty;
                    } else {
                        let bytes = body.as_bytes().to_owned();
                        length = bytes.len();
                        content = Content::Bytes(bytes);
                    }
                }
            }
            headers.insert(String::from("Content-Length"), length.to_string());
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
            Ok((Request::default(), Content::Empty))
        }
    }

    pub async fn write<W: AsyncWrite + Unpin>(&mut self, mut writer: W, content: Content) -> Result<(), std::io::Error> {
        writer
            .write_all(format!("{} {} {}\r\n", self.method, self.url.path, self.version).as_bytes())
            .await?;
        for header in self.headers.iter() {
            writer
                .write_all(format!("{}: {}\r\n", header.name, header.value).as_bytes())
                .await?;
        }
        writer.write_all("\r\n".as_bytes()).await?;
        content.write(&mut writer).await?;
        writer.flush().await?;
        Ok(())
    }
}

#[tokio::test]
async fn test_from_message_get() {
    let message = r#"
    GET http://httpbin.org/get
    Host: httpbin.org"#;
    let (mut request, content) = Request::from(message).await.unwrap();
    let mut writer = tokio::io::stdout();
    request.write(&mut writer, content).await.unwrap();
    writer.write_all("\r\n".as_bytes()).await.unwrap();
    assert_eq!("GET", request.method.as_ref());
}

#[tokio::test]
async fn test_from_message_post_multipart() {
    let message = r#"
    POST https://httpbin.org/post
    Host: httpbin.org
    Content-Type: multipart/form-data

    a: b
    f: @src/lib.rs"#;
    let (mut request, content) = Request::from(message).await.unwrap();
    let mut writer = tokio::io::stdout();
    request.write(&mut writer, content).await.unwrap();
    writer.write_all("\r\n".as_bytes()).await.unwrap();
    assert_eq!("POST", request.method.as_ref());
}
