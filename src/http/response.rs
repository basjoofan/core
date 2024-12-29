use super::Error;
use super::Header;
use super::Headers;
use super::Stream;
use crate::Value;
use std::collections::HashMap;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;

#[derive(Default)]
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
    pub fn from(mut reader: BufReader<Stream>, f: Option<impl FnMut()>) -> Result<Response, Error> {
        let mut buf = Vec::with_capacity(1);
        reader.read(&mut buf).map_err(Error::ReadFailed)?;
        if let Some(mut f) = f {
            f()
        }
        let mut line = String::from_utf8(buf).unwrap_or_default();
        reader.read_line(&mut line).map_err(Error::ReadFailed)?;
        let mut splits = line.split_whitespace();
        let version = parse::<String>(splits.next());
        let status = parse::<u16>(splits.next());
        let reason = parse::<String>(splits.next());
        // headers
        let mut headers = Headers::default();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).map_err(Error::ReadFailed)?;
            if line.trim().is_empty() {
                break;
            } else if let Some((name, value)) = line.split_once(':') {
                headers.insert(Header {
                    name: name.trim().to_string(),
                    value: value.trim().to_string(),
                });
            }
        }
        let mut body = String::default();
        reader.read_to_string(&mut body).map_err(Error::ReadFailed)?;
        Ok(Response {
            version,
            status,
            reason,
            headers,
            body,
        })
    }

    pub fn into_value(self) -> Value {
        let mut map = HashMap::new();
        map.insert(String::from("version"), Value::String(self.version));
        map.insert(String::from("status"), Value::Integer(self.status as i64));
        map.insert(String::from("reason"), Value::String(self.reason));
        let mut headers: HashMap<String, Value> = HashMap::new();
        for header in self.headers {
            match headers.get_mut(&header.name) {
                Some(Value::Array(array)) => array.push(Value::String(header.value)),
                _ => {
                    headers.insert(header.name, Value::Array(vec![Value::String(header.value)]));
                }
            }
        }
        map.insert(String::from("headers"), Value::Map(headers));
        map.insert(String::from("body"), Value::String(self.body));
        // TODO parse json value
        // let json = Parser::new(&self.body).parse().eval(&mut Context::default());
        // map.insert(String::from("json"), json);
        Value::Map(map)
    }
}

fn parse<T: std::str::FromStr + std::default::Default>(str: Option<&str>) -> T {
    match str {
        Some(str) => str.parse::<T>().unwrap_or_default(),
        None => T::default(),
    }
}

#[test]
fn test_from_message_json() {
    let message = r#"HTTP/1.1 200 OK
    Date: Sun, 21 Jul 2024 14:32:11 GMT
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
    let response = Response::from(
        BufReader::new(Stream::Mock(std::io::Cursor::new(message.as_bytes().to_owned()))),
        None::<Box<dyn FnMut()>>,
    )
    .unwrap();
    assert_eq!(200, response.status);
    assert_eq!(7, response.headers.len());
    assert_eq!("    {\n    \"origin\": \"104.28.152.141\"\n    }\n    ", response.body);
}
