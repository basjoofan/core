use super::Request;
use super::Response;
use super::Stream;
use super::Time;
use std::time::Duration;
use std::time::Instant;
use std::time::SystemTime;
use tokio::io::split;

pub struct Client {
    connect_tiomeout: Duration,
    // TODO read_tiomeout: Option<Duration>,
}

impl Default for Client {
    fn default() -> Self {
        Self {
            connect_tiomeout: Duration::from_secs(120),
        }
    }
}

impl Client {
    /// Send this request and wait for the record.
    pub async fn send(&self, message: &str) -> (Request, Response, Time, String) {
        let (mut request, content) = match Request::from(message).await {
            Ok((request, content)) => (request, content),
            Err(error) => return (Request::default(), Response::default(), Time::default(), error.to_string()),
        };
        let mut time = Time::default();
        let start = Instant::now();
        let stream = match Stream::connect(&request.url, self.connect_tiomeout).await {
            Ok(stream) => stream,
            Err(error) => return (request, Response::default(), time, error.to_string()),
        };
        time.resolve = stream.resolve();
        time.connect = start.elapsed() - time.resolve;
        let (mut reader, mut writer) = split(stream);
        if let Err(error) = request.write(&mut writer, content).await {
            return (request, Response::default(), time, error.to_string());
        };
        let read = Instant::now();
        let response = match Response::from(&mut reader, Some(|| time.delay = read.elapsed())).await {
            Ok(response) => response,
            Err(error) => return (request, Response::default(), time, error.to_string()),
        };
        time.read = read.elapsed() - time.delay;
        let end = Instant::now();
        time.total = end - start;
        time.end = SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default();
        time.write = time.total - time.resolve - time.connect - time.read - time.delay;
        (request, response, time, String::default())
    }
}

#[tokio::test]
async fn test_send_message_get() {
    crate::tests::start_server(30001).await;
    let message = r#"
    GET http://127.0.0.1:30001/get
    Host: 127.0.0.1"#;
    let client = Client::default();
    let (request, response, time, error) = client.send(message).await;
    println!("error: {error}");
    assert_eq!("GET", request.method.as_ref());
    assert_eq!(200, response.status);
    assert_eq!(time.total, time.resolve + time.connect + time.write + time.delay + time.read);
    println!("{:?}", time.total);
    println!("{:?}", response.body);
}

#[tokio::test]
async fn test_send_message_post() {
    crate::tests::start_server(30002).await;
    let message = r#"
    POST http://127.0.0.1:30002/text
    Host: 127.0.0.1
    Accept-Encoding: gzip, deflate"#;
    let client = Client::default();
    let (request, response, time, error) = client.send(message).await;
    println!("error: {error}");
    println!("request: {request:?}");
    assert_eq!("POST", request.method.as_ref());
    assert_eq!(200, response.status);
    assert_eq!(time.total, time.resolve + time.connect + time.write + time.delay + time.read);
    println!("{:?}", time.total);
    println!("{:?}", response.body);
}

#[tokio::test]
async fn test_send_message_post_form() {
    crate::tests::start_server(30003).await;
    let message = r#"
    POST http://127.0.0.1:30003/form
    Host: 127.0.0.1
    Content-Type: application/x-www-form-urlencoded

    a: b"#;
    let client = Client::default();
    let (request, response, time, error) = client.send(message).await;
    println!("error: {error}");
    println!("request: {request:?}");
    assert_eq!("POST", request.method.as_ref());
    assert_eq!(200, response.status);
    assert_eq!(time.total, time.resolve + time.connect + time.write + time.delay + time.read);
    println!("{:?}", time.total);
    println!("{:?}", response.body);
}

#[tokio::test]
async fn test_send_message_post_multipart() {
    crate::tests::start_server(30004).await;
    let message = r#"
    POST http://127.0.0.1:30004/multipart
    Host: 127.0.0.1
    Content-Type: multipart/form-data

    a: b
    f: @src/lib.rs"#;
    let client = Client::default();
    let (request, response, time, error) = client.send(message).await;
    println!("error: {error}");
    println!("request: {request:?}");
    println!("response: {response:?}");
    assert_eq!("POST", request.method.as_ref());
    assert_eq!(200, response.status);
    assert_eq!(time.total, time.resolve + time.connect + time.write + time.delay + time.read);
    println!("{:?}", time.total);
    println!("{:?}", response.body);
}

#[tokio::test]
async fn test_send_message_post_json() {
    crate::tests::start_server(30005).await;
    let message = r#"
    POST http://127.0.0.1:30005/json
    Host: 127.0.0.1
    Content-Type: application/json

    {
        "name": "Gauss",
        "age": 6,
        "address": {
            "street": "19 Hear Sea Street",
            "city": "DaLian"
        },
        "phones": [
            "+86 13098767890",
            "+86 15876567890"
        ]
    }
    "#;
    let client = Client::default();
    let (request, response, time, error) = client.send(message).await;
    println!("error: {error}");
    assert_eq!("POST", request.method.as_ref());
    assert_eq!(200, response.status);
    assert_eq!(time.total, time.resolve + time.connect + time.write + time.delay + time.read);
    println!("{:?}", time.total);
    println!("{:?}", response.body);
}
