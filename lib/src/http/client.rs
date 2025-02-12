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
            connect_tiomeout: Duration::from_secs(200000000),
        }
    }
}

impl Client {
    /// Send this request and wait for the record.
    pub async fn send(&self, message: &str) -> (Request, Response, Time, String) {
        let (mut request, content) = Request::from(message);
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
    let message = r#"
    GET https://www.baidu.com
    Host: www.baidu.com
    Connection: close"#;
    let client = Client::default();
    let (request, response, time, error) = client.send(message).await;
    println!("error: {}", error);
    assert_eq!("GET", request.method.as_ref());
    assert_eq!(200, response.status);
    assert_eq!(time.total, time.resolve + time.connect + time.write + time.delay + time.read);
    println!("{:?}", time.total);
    println!("{:?}", response.body);
}

#[tokio::test]
async fn test_send_message_post() {
    let message = r#"
    POST https://httpbin.org/post
    Host: httpbin.org
    Accept-Encoding: gzip, deflate
    Connection: close"#;
    let client = Client::default();
    let (request, response, time, error) = client.send(message).await;
    println!("error: {}", error);
    println!("request: {:?}", request);
    assert_eq!("POST", request.method.as_ref());
    assert_eq!(200, response.status);
    assert_eq!(time.total, time.resolve + time.connect + time.write + time.delay + time.read);
    println!("{:?}", time.total);
    println!("{:?}", response.body);
}

#[tokio::test]
async fn test_send_message_post_form() {
    let message = r#"
    POST https://httpbin.org/post
    Host: httpbin.org
    Content-Type: application/x-www-form-urlencoded
    Connection: close

    a: b"#;
    let client = Client::default();
    let (request, response, time, error) = client.send(message).await;
    println!("error: {}", error);
    println!("request: {:?}", request);
    assert_eq!("POST", request.method.as_ref());
    assert_eq!(200, response.status);
    assert_eq!(time.total, time.resolve + time.connect + time.write + time.delay + time.read);
    println!("{:?}", time.total);
    println!("{:?}", response.body);
}

#[tokio::test]
async fn test_send_message_post_multipart() {
    let message = r#"
    POST https://httpbin.org/post
    Host: httpbin.org
    Content-Type: multipart/form-data
    Connection: close

    a: b
    f: @src/lib.rs"#;
    let client = Client::default();
    let (request, response, time, error) = client.send(message).await;
    println!("error: {}", error);
    assert_eq!("POST", request.method.as_ref());
    assert_eq!(200, response.status);
    assert_eq!(time.total, time.resolve + time.connect + time.write + time.delay + time.read);
    println!("{:?}", time.total);
    println!("{:?}", response.body);
}

#[tokio::test]
async fn test_send_message_post_json() {
    let message = r#"
    POST https://httpbin.org/post
    Host: httpbin.org
    Content-Type: application/json
    Connection: close

    {
        "name": "John Doe",
        "age": 43,
        "address": {
            "street": "10 Downing Street",
            "city": "London"
        },
        "phones": [
            "+44 1234567",
            "+44 2345678"
        ]
    }
    "#;
    let client = Client::default();
    let (request, response, time, error) = client.send(message).await;
    println!("error: {}", error);
    assert_eq!("POST", request.method.as_ref());
    assert_eq!(200, response.status);
    assert_eq!(time.total, time.resolve + time.connect + time.write + time.delay + time.read);
    println!("{:?}", time.total);
    println!("{:?}", response.body);
}
