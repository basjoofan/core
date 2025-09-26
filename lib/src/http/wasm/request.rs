use super::super::Headers;
use super::super::Method;
use super::super::Request;
use super::super::Url;
use super::super::Version;
use wasm_bindgen::JsValue;

impl Request {
    /// Converts a message to an http request.
    pub fn from(message: &str) -> Result<(Request, Option<JsValue>), std::io::Error> {
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
            Ok((
                Request {
                    method,
                    url,
                    version,
                    headers,
                    body: String::new(),
                },
                None,
            ))
        } else {
            Ok((Request::default(), None))
        }
    }
}
