use crate::Error;
use crate::Request;
use crate::Response;
use crate::Stream;
use std::io::BufReader;
use std::time::Duration;
use std::time::Instant;

#[derive(Default)]
pub struct Client {
    connect_tiomeout: Option<Duration>,
    read_tiomeout: Option<Duration>,
}

#[derive(Debug)]
pub struct Time {
    pub start: Instant,
    pub end: Instant,
    pub total: Duration,
    pub resolve: Duration,
    pub connect: Duration,
    pub write: Duration,
    pub read: Duration,
    pub delay: Duration,
}

impl Client {
    /// Send this request and wait for the result.
    pub fn send<'a>(&'a self, message: &'a str) -> Result<(Request, Response, Time), Error> {
        let mut request = Request::try_from(message)?;
        let start = Instant::now();
        let mut stream = Stream::connect(&request.url, self.connect_tiomeout, self.read_tiomeout)?;
        let resolve = stream.resolve();
        let connect = start.elapsed() - resolve;
        request.write(&mut stream)?;
        let read = Instant::now();
        let mut delay = Duration::default();
        let response = Response::from(BufReader::new(stream), Some(|| delay = read.elapsed()))?;
        let read = read.elapsed() - delay;
        let end = Instant::now();
        let total = end - start;
        let write = total - resolve - connect - read - delay;
        Ok((
            request,
            response,
            Time {
                start,
                end,
                total,
                resolve,
                connect,
                write,
                read,
                delay,
            },
        ))
    }
}

#[test]
fn test_send_message_get() {
    let message = r#"
    GET https://httpbin.org/get
    Host: httpbin.org
    Connection: close"#;
    let client = Client::default();
    let (request, response, time) = client.send(message).unwrap();
    assert_eq!("GET", request.method.as_ref());
    assert_eq!(200, response.status);
    assert_eq!(
        time.total,
        time.resolve + time.connect + time.write + time.delay + time.read
    );
    println!("{:?}", time);
}

#[test]
fn test_send_message_post() {
    let message = r#"
    POST https://httpbin.org/post
    Host: httpbin.org
    Connection: close"#;
    let client = Client::default();
    let (request, response, time) = client.send(message).unwrap();
    assert_eq!("POST", request.method.as_ref());
    assert_eq!(200, response.status);
    assert_eq!(
        time.total,
        time.resolve + time.connect + time.write + time.delay + time.read
    );
    println!("{:?}", time);
}

#[test]
fn test_send_message_post_form() {
    let message = r#"
    POST http://httpbin.org/post
    Host: httpbin.org
    Content-Type: application/x-www-form-urlencoded
    Connection: close

    a: b
    "#;
    let client = Client::default();
    let (request, response, time) = client.send(message).unwrap();
    assert_eq!("POST", request.method.as_ref());
    assert_eq!(200, response.status);
    assert_eq!(
        time.total,
        time.resolve + time.connect + time.write + time.delay + time.read
    );
    println!("{:?}", time);
    println!("{:?}", response.body);
}

#[test]
fn test_send_message_post_multipart() {
    let message = r#"
    POST http://httpbin.org/post
    Host: httpbin.org
    Content-Type: multipart/form-data
    Connection: close

    a: b
    f: @src/lib.rs
    "#;
    let client = Client::default();
    let (request, response, time) = client.send(message).unwrap();
    assert_eq!("POST", request.method.as_ref());
    assert_eq!(200, response.status);
    assert_eq!(
        time.total,
        time.resolve + time.connect + time.write + time.delay + time.read
    );
    println!("{:?}", time);
    println!("{:?}", response.body);
}

#[test]
fn test_send_message_post_json() {
    let message = r#"
    POST http://httpbin.org/post
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
    let (request, response, time) = client.send(message).unwrap();
    assert_eq!("POST", request.method.as_ref());
    assert_eq!(200, response.status);
    assert_eq!(
        time.total,
        time.resolve + time.connect + time.write + time.delay + time.read
    );
    println!("{:?}", time);
    println!("{:?}", response.body);
}
