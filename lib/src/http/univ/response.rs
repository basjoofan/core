use super::super::Headers;
use super::super::Response;
use super::Stream;
use tokio::io::AsyncBufReadExt;
use tokio::io::AsyncReadExt;
use tokio::io::BufReader;
use tokio::io::ReadHalf;

impl Response {
    /// Converts a stream to an http response.
    pub async fn from(
        reader: &mut ReadHalf<Stream>,
        callback: Option<impl FnMut()>,
    ) -> Result<Response, std::io::Error> {
        let mut reader = BufReader::new(reader);
        let mut buf = Vec::with_capacity(1);
        reader.read_exact(&mut buf).await?;
        if let Some(mut callback) = callback {
            callback()
        }
        let mut line = String::from_utf8(buf).unwrap_or_default();
        reader.read_line(&mut line).await?;
        let mut splits = line.split_whitespace();
        let version = parse::<String>(splits.next());
        let status = parse::<u16>(splits.next());
        let reason = parse::<String>(splits.next());
        // headers
        let mut headers = Headers::default();
        let mut length = None;
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).await?;
            if line.trim().is_empty() {
                break;
            } else if let Some((name, value)) = line.split_once(':') {
                headers.insert(name.trim().to_string(), value.trim().to_string());
                if length.is_none() && name.trim().to_lowercase() == "content-length" {
                    length = value.trim().parse::<usize>().ok();
                }
            }
        }
        let mut body = Vec::new();
        match length {
            Some(length) => {
                body.resize(length, u8::MIN);
                reader.read_exact(&mut body).await?
            }
            None => reader.read_to_end(&mut body).await?,
        };
        // TODO other encoding
        let body = match String::from_utf8(body) {
            Ok(body) => body,
            Err(error) => error.to_string(),
        };
        Ok(Response {
            version,
            status,
            reason,
            headers,
            body,
        })
    }
}

fn parse<T: std::str::FromStr + std::default::Default>(str: Option<&str>) -> T {
    match str {
        Some(str) => str.parse::<T>().unwrap_or_default(),
        None => T::default(),
    }
}

#[tokio::test]
async fn test_from_message_read_exact() {
    let message = r#"HTTP/1.1 200 OK
Date: Sun, 17 Feb 2025 07:22:38 GMT
Content-Type: application/json
Content-Length: 30
Connection: close
Server: gunicorn/19.9.0
Access-Control-Allow-Origin: *
Access-Control-Allow-Credentials: true

{
  "origin": "122.9.3.166"
}
"#;
    let stream = Stream::Mock(std::io::Cursor::new(message.as_bytes().to_owned()));
    let (mut reader, _) = tokio::io::split(stream);
    let response = Response::from(&mut reader, None::<Box<dyn FnMut()>>)
        .await
        .unwrap();
    assert_eq!(200, response.status);
    assert_eq!(7, response.headers.len());
    assert_eq!("{\n  \"origin\": \"122.9.3.166\"\n}\n", response.body);
    assert_eq!(
        Some(&super::super::Value::Map(
            std::collections::HashMap::from_iter(vec![(
                String::from("origin"),
                super::super::Value::String(String::from("122.9.3.166"))
            )])
        )),
        response.to_map().get("json")
    )
}

#[tokio::test]
async fn test_from_message_read_to_end() {
    let message = r#"HTTP/1.1 200 OK
Date: Sun, 17 Feb 2025 07:22:38 GMT
Content-Type: application/json
Content-Length: 3a
Connection: close
Server: gunicorn/19.9.0
Access-Control-Allow-Origin: *
Access-Control-Allow-Credentials: true

{
  "origin": "122.9.3.166"
}
"#;
    let stream = Stream::Mock(std::io::Cursor::new(message.as_bytes().to_owned()));
    let (mut reader, _) = tokio::io::split(stream);
    let response = Response::from(&mut reader, None::<Box<dyn FnMut()>>)
        .await
        .unwrap();
    assert_eq!(200, response.status);
    assert_eq!(7, response.headers.len());
    assert_eq!("{\n  \"origin\": \"122.9.3.166\"\n}\n", response.body);
    assert_eq!(
        Some(&super::super::Value::Map(
            std::collections::HashMap::from_iter(vec![(
                String::from("origin"),
                super::super::Value::String(String::from("122.9.3.166"))
            )])
        )),
        response.to_map().get("json")
    )
}
