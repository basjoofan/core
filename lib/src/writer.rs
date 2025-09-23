use crate::Record;
use std::fmt::Write;
use tokio::io::AsyncWrite;
use tokio::io::AsyncWriteExt;

pub struct Writer<W> {
    w: W,
}

impl<W: AsyncWrite + Unpin> Writer<W> {
    pub fn new(w: W) -> Self {
        Writer { w }
    }

    pub async fn write(&mut self, records: &[Record], name: &str, task: u32, number: u32) {
        let mut buffer = String::new();
        for (order, record) in records.iter().enumerate() {
            let _ = write!(buffer, r#"{{"#);
            let _ = write!(buffer, r#""name": "{}", "#, name);
            let _ = write!(buffer, r#""task": {}, "#, task);
            let _ = write!(buffer, r#""number": {}, "#, number);
            let _ = write!(buffer, r#""order": {}, "#, order);
            let _ = write!(buffer, r#""time_start": {}, "#, record.time.start.as_nanos());
            let _ = write!(buffer, r#""time_end": {}, "#, record.time.end.as_nanos());
            let _ = write!(buffer, r#""time_total": {}, "#, record.time.total.as_nanos());
            let _ = write!(buffer, r#""time_resolve": {}, "#, record.time.resolve.as_nanos());
            let _ = write!(buffer, r#""time_connect": {}, "#, record.time.connect.as_nanos());
            let _ = write!(buffer, r#""time_write": {}, "#, record.time.write.as_nanos());
            let _ = write!(buffer, r#""time_delay": {}, "#, record.time.delay.as_nanos());
            let _ = write!(buffer, r#""time_read": {}, "#, record.time.read.as_nanos());
            let _ = write!(buffer, r#""request_name": "{}", "#, record.name);
            let _ = write!(buffer, r#""request_method": "{}", "#, record.request.method);
            let _ = write!(buffer, r#""request_url": "{}", "#, record.request.url);
            let _ = write!(buffer, r#""request_version": "{}", "#, record.request.version);
            let _ = write!(buffer, r#""request_headers": ["#);
            let mut headers = record.request.headers.iter().peekable();
            while let Some(header) = headers.next() {
                let _ = write!(buffer, r#"["{}", "{}"]"#, header.name, header.value);
                if headers.peek().is_some() {
                    let _ = write!(buffer, r#", "#);
                }
            }
            let _ = write!(buffer, r#"], "#);
            let _ = write!(buffer, r#""request_body": "{}", "#, record.request.body);
            let _ = write!(buffer, r#""response_version": "{}", "#, record.response.version);
            let _ = write!(buffer, r#""response_status": {}, "#, record.response.status);
            let _ = write!(buffer, r#""response_reason": "{}", "#, record.response.reason);
            let _ = write!(buffer, r#""response_headers": ["#);
            let mut headers = record.response.headers.iter().peekable();
            while let Some(header) = headers.next() {
                let _ = write!(buffer, r#"["{}", "{}"]"#, header.name, header.value);
                if headers.peek().is_some() {
                    let _ = write!(buffer, r#", "#);
                }
            }
            let _ = write!(buffer, r#"], "#);
            let _ = write!(buffer, r#""response_body": "{}", "#, record.response.body);
            let _ = write!(buffer, r#""asserts": ["#);
            let mut asserts = record.asserts.iter().peekable();
            while let Some(assert) = asserts.next() {
                let _ = write!(buffer, r#"{{"#);
                let _ = write!(buffer, r#""expr": "{}", "#, assert.expr);
                let _ = write!(buffer, r#""left": "{}", "#, assert.left);
                let _ = write!(buffer, r#""compare": "{}", "#, assert.compare);
                let _ = write!(buffer, r#""right": "{}", "#, assert.right);
                let _ = write!(buffer, r#""result": {}"#, assert.result);
                let _ = write!(buffer, r#"}},"#);
                if asserts.peek().is_some() {
                    let _ = write!(buffer, r#", "#);
                }
            }
            let _ = write!(buffer, r#"], "#);
            let _ = writeln!(buffer, r#""error": "{}"}}"#, record.error);
        }
        let _ = self.w.write(buffer.as_bytes()).await;
        let _ = self.w.flush().await;
    }
}

#[tokio::test]
async fn test_writer() {
    let mut writer = Writer::new(Vec::new());
    let record = Record {
        name: "test".to_string(),
        time: crate::http::Time::default(),
        request: crate::http::Request::default(),
        response: crate::http::Response::default(),
        asserts: Vec::new(),
        error: String::default(),
    };
    writer.write(&vec![record], "test", 0, 0).await;
    let encoded = writer.w;
    print!("{}", String::from_utf8_lossy(&encoded));
    let record = serde_json::from_slice::<serde_json::Value>(&encoded).unwrap();
    println!("{:?}", record);
    assert_eq!(record["name"].as_str(), Some("test"));
    assert_eq!(record["request_headers"].as_array().unwrap().len(), 0);

    let mut writer = Writer::new(Vec::new());
    let mut request_headers = crate::http::Headers::default();
    request_headers.insert(String::from("a"), String::from("b"));
    request_headers.insert(String::from("a"), String::from("c"));
    let mut response_headers = crate::http::Headers::default();
    response_headers.insert(String::from("d"), String::from("e"));
    response_headers.insert(String::from("d"), String::from("f"));
    response_headers.insert(String::from("g"), String::from("h"));
    let record = Record {
        name: "test".to_string(),
        time: crate::http::Time::default(),
        request: crate::http::Request {
            method: crate::http::Method::Get,
            url: crate::http::Url::from("http://localhost:8080"),
            version: crate::http::Version::Http11,
            headers: request_headers,
            body: String::default(),
        },
        response: crate::http::Response {
            version: String::from("HTTP/1.1"),
            status: 200,
            reason: String::from("OK"),
            headers: response_headers,
            body: String::default(),
        },
        asserts: Vec::new(),
        error: String::default(),
    };
    writer.write(&vec![record], "test", 0, 0).await;
    let encoded = writer.w;
    print!("{}", String::from_utf8_lossy(&encoded));
    let record = serde_json::from_slice::<serde_json::Value>(&encoded).unwrap();
    println!("{:?}", record);
    assert_eq!(record["name"].as_str(), Some("test"));
    assert_eq!(record["request_headers"].as_array().unwrap().len(), 2);
    assert_eq!(record["request_headers"][0][0].as_str(), Some("a"));
    assert_eq!(record["request_headers"][0][1].as_str(), Some("b"));
    assert_eq!(record["request_headers"][1][0].as_str(), Some("a"));
    assert_eq!(record["request_headers"][1][1].as_str(), Some("c"));
    assert_eq!(record["response_headers"].as_array().unwrap().len(), 3);
    assert_eq!(record["response_headers"][0][0].as_str(), Some("d"));
    assert_eq!(record["response_headers"][0][1].as_str(), Some("e"));
    assert_eq!(record["response_headers"][1][0].as_str(), Some("d"));
    assert_eq!(record["response_headers"][1][1].as_str(), Some("f"));
    assert_eq!(record["response_headers"][2][0].as_str(), Some("g"));
    assert_eq!(record["response_headers"][2][1].as_str(), Some("h"));
}
