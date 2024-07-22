// use crate::Body;
use crate::Error;
use crate::Header;
use crate::Headers;
use crate::Stream;
use crate::Version;
use std::io::BufRead;
use std::io::BufReader;
use std::io::Read;

pub struct Response {
    /// The response's version
    pub version: Version,
    /// The response's status
    pub status: u16,
    /// The response's reason
    pub reason: String,
    /// The response's headers
    pub headers: Headers,
    /// The response's body
    pub body: String,
}

/// Converts a message to an http request.
impl Response {
    pub fn from(mut reader: BufReader<Stream>, f: Option<impl FnMut()>) -> Result<Response, Error> {
        let mut buf = [0; 1];
        reader.read(&mut buf).map_err(|_e| Error::ReadFailed)?;
        f.map(|mut f| f());
        let mut line = String::from_utf8(buf.to_vec()).map_err(|_e| Error::InvalidVersion)?;
        reader.read_line(&mut line).map_err(|_e| Error::ReadFailed)?;
        let mut splits = line.split_whitespace();
        let version = match splits.next() {
            Some(version) => Version::try_from(version),
            None => Err(Error::InvalidVersion),
        }?;
        let status = match splits.next() {
            Some(status) => status.parse::<u16>().map_err(|_e| Error::InvalidStatus),
            None => Err(Error::InvalidStatus),
        }?;
        let reason = splits.next().unwrap_or_default().to_string();
        // headers
        let mut headers = Headers::default();
        loop {
            let mut line = String::new();
            reader.read_line(&mut line).map_err(|_e| Error::ReadFailed)?;
            if line.trim().is_empty() {
                break;
            } else {
                if let Some((name, value)) = line.split_once(':') {
                    headers.insert(Header {
                        name: name.trim().to_string(),
                        value: value.trim().to_string(),
                    });
                }
            }
        }
        let start = std::time::Instant::now();
        let mut body = String::default();
        reader.read_to_string(&mut body).map_err(|_e| Error::ReadFailed)?;
        println!("read body {:?}", start.elapsed());
        Ok(Response {
            version,
            status,
            reason,
            headers,
            body,
        })
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
