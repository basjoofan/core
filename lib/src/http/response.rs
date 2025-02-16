use super::Headers;
use super::Stream;
use crate::Parser;
use crate::Source;
use crate::Value;
use std::collections::HashMap;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncReadExt;
use tokio::io::BufReader;
use tokio::io::ReadHalf;

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
    /// Converts a stream to an http response.
    pub async fn from(reader: &mut ReadHalf<Stream>, f: Option<impl FnMut()>) -> Result<Response, std::io::Error> {
        let mut reader = BufReader::new(reader);
        let mut buf = Vec::with_capacity(1);
        reader.read_exact(&mut buf).await?;
        if let Some(mut f) = f {
            f()
        }
        let mut line = String::from_utf8(buf).unwrap_or_default();
        reader.read_line(&mut line).await?;
        let mut splits = line.split_whitespace();
        let version = parse::<String>(splits.next());
        let status = parse::<u16>(splits.next());
        let reason = parse::<String>(splits.next());
        // headers
        let mut headers = Headers::default();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).await?;
            if line.trim().is_empty() {
                break;
            } else if let Some((name, value)) = line.split_once(':') {
                headers.insert(name.trim().to_string(), value.trim().to_string());
            }
        }
        let mut body = String::default();
        reader.read_to_string(&mut body).await?;
        Ok(Response {
            version,
            status,
            reason,
            headers,
            body,
        })
    }

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

fn parse<T: std::str::FromStr + std::default::Default>(str: Option<&str>) -> T {
    match str {
        Some(str) => str.parse::<T>().unwrap_or_default(),
        None => T::default(),
    }
}

#[tokio::test]
async fn test_from_message_json() {
    let message = r#"HTTP/1.1 200 OK
    Date: Sun, 21 Jul 2025 14:32:11 GMT
    Content-Type: application/json
    Content-Length: 33
    Connection: close
    Server: gunicorn/19.9.0
    Access-Control-Allow-Origin: *
    Access-Control-Allow-Credentials: true

    {
    "origin": "104.28.152.141"
    }
    "#;
    let stream = Stream::Mock(std::io::Cursor::new(message.as_bytes().to_owned()));
    let (mut reader, _) = tokio::io::split(stream);
    let response = Response::from(&mut reader, None::<Box<dyn FnMut()>>).await.unwrap();
    assert_eq!(200, response.status);
    assert_eq!(7, response.headers.len());
    assert_eq!("    {\n    \"origin\": \"104.28.152.141\"\n    }\n    ", response.body);
    assert_eq!(
        Some(&Value::Map(HashMap::from_iter(vec![(
            String::from("origin"),
            Value::String(String::from("104.28.152.141"))
        )]))),
        response.to_map().get("json")
    )
}
