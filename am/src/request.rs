use super::record::Request;
use super::record::Response;
use reqwest::header::CONTENT_TYPE;
use reqwest::multipart::Form;
use reqwest::multipart::Part;
use reqwest::Body;
use reqwest::Client;
use reqwest::Method;
use reqwest::Version;
use std::error::Error;
use std::path::Path;
use std::time;
use std::time::Duration;
use tokio::fs::File;
use tokio_util::codec::BytesCodec;
use tokio_util::codec::FramedRead;
use uuid::Uuid;

pub async fn send(
    client: Client,
    message: &str,
    name: String,
) -> Result<(Duration, Request, Response), Box<dyn Error>> {
    let mut lines = message.trim().lines();
    let (method, url, version) = from_line(lines.next());
    let mut builder = client.request(method, url).version(version);
    let mut content_type = None;
    while let Some(line) = lines.next() {
        if line.trim().is_empty() {
            break;
        } else {
            if let Some((key, value)) = line.trim().split_once(':') {
                builder = builder.header(key.trim(), value.trim());
                if key.trim() == CONTENT_TYPE {
                    content_type = Some(value.trim());
                }
            }
        }
    }
    let mut fields = Vec::new();
    let mut content = String::default();
    builder = match content_type {
        Some("application/x-www-form-urlencoded") => {
            while let Some(line) = lines.next() {
                if let Some((key, value)) = line.trim().split_once(':') {
                    fields.push((key.trim().to_owned(), value.trim().to_owned()));
                }
            }
            builder.form(&fields)
        }
        Some("multipart/form-data") => {
            let mut parts = Form::new();
            while let Some(line) = lines.next() {
                if let Some((key, value)) = line.trim().split_once(':') {
                    let (key, value) = (key.trim().to_owned(), value.trim().to_owned());
                    fields.push((key.clone(), value.clone()));
                    if value.starts_with('@') {
                        let path = Path::new(&value[1..value.len()]);
                        let file = File::open(path).await?;
                        let stream = FramedRead::new(file, BytesCodec::new());
                        let mut part = Part::stream(Body::wrap_stream(stream));
                        if let Some(file_name) = path.file_name() {
                            if let Some(file_name) = file_name.to_str() {
                                part = part.file_name(String::from(file_name));
                            }
                        }
                        parts = parts.part(key, part);
                    } else {
                        parts = parts.text(key, value);
                    }
                }
            }
            builder.multipart(parts)
        }
        _ => {
            content = String::from_iter(lines);
            builder.body(content.clone())
        }
    };
    let request = builder.build()?;
    let request_clone = clone_request(&request, fields, content, name);
    let start_instant = time::Instant::now();
    let response = client.execute(request).await?;
    let version = format!("{:?}", response.version());
    let status = response.status().as_u16();
    let reason = response.status().canonical_reason().unwrap_or_default().to_string();
    let headers = response
        .headers()
        .iter()
        .map(|(k, v)| {
            (
                format!("{}", k),
                format!("{}", v.to_str().unwrap_or(String::default().as_str())),
            )
        })
        .collect();
    let body = response.text().await?;
    let end_instant = time::Instant::now();
    let duration = end_instant - start_instant;
    Ok((
        duration,
        request_clone,
        Response {
            version,
            status,
            reason,
            headers,
            body,
        },
    ))
}

fn clone_request(request: &reqwest::Request, fields: Vec<(String, String)>, content: String, name: String) -> Request {
    let id = Uuid::now_v7().to_string();
    Request {
        id,
        name,
        method: request.method().as_str().to_string(),
        url: request.url().to_string(),
        version: format!("{:?}", request.version()),
        headers: request
            .headers()
            .iter()
            .map(|(k, v)| {
                (
                    format!("{}", k),
                    format!("{}", v.to_str().unwrap_or(String::default().as_str())),
                )
            })
            .collect(),
        fields: fields,
        content: content,
    }
}

fn from_line(line: Option<&str>) -> (Method, &str, Version) {
    let (mut method, mut url, mut version) = (Method::GET, "https://localhost", Version::HTTP_11);
    if let Some(line) = line {
        let mut splits = line.split_whitespace();
        method = method_from_str(splits.next());
        if let Some(url_str) = splits.next() {
            url = url_str;
        }
        version = version_from_str(splits.next());
    };
    (method, url, version)
}

fn method_from_str(str: Option<&str>) -> Method {
    if let Some(str) = str {
        match Method::from_bytes(str.as_bytes()) {
            Ok(method) => method,
            _ => Method::GET,
        }
    } else {
        Method::GET
    }
}

fn version_from_str(str: Option<&str>) -> Version {
    if let Some(str) = str {
        match str.to_uppercase().as_str() {
            "HTTP/0.9" => Version::HTTP_09,
            "HTTP/1.0" => Version::HTTP_10,
            "HTTP/1.1" => Version::HTTP_11,
            "HTTP/2.0" => Version::HTTP_2,
            "HTTP/3.0" => Version::HTTP_3,
            _ => Version::HTTP_11,
        }
    } else {
        Version::HTTP_11
    }
}

#[test]
fn test_get() {
    let name = String::from("name");
    let message = r#"
    GET http://httpbin.org/get
    Host: httpbin.org"#;
    let client = Client::new();
    let (_, _, response) = tokio_test::block_on(send(client, message, name)).unwrap();
    println!("body:{}", response.body)
}

#[test]
fn test_post() {
    let name = String::from("name");
    let message = r#"
    POST http://httpbin.org/post
    Host: httpbin.org"#;
    let client = Client::new();
    let (_, _, response) = tokio_test::block_on(send(client, message, name)).unwrap();
    println!("body:{}", response.body)
}

#[test]
fn test_post_form() {
    let name = String::from("name");
    let message = r#"
    POST http://httpbin.org/post
    Host: httpbin.org
    Content-Type: application/x-www-form-urlencoded

    a: b
    "#;
    let client = Client::new();
    let (_, _, response) = tokio_test::block_on(send(client, message, name)).unwrap();
    println!("body:{}", response.body)
}

#[test]
fn test_post_multipart() {
    let name = String::from("name");
    let message = r#"
    POST http://httpbin.org/post
    Host: httpbin.org
    Content-Type: multipart/form-data

    a: b
    f: @src/lib.rs
    "#;
    let client = Client::new();
    let (_, _, response) = tokio_test::block_on(send(client, message, name)).unwrap();
    println!("body:{}", response.body)
}

#[test]
fn test_post_json() {
    let name = String::from("name");
    let message = r#"
    POST http://httpbin.org/post
    Host: httpbin.org
    Content-Type: application/json

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
    let client = Client::new();
    let (_, _, response) = tokio_test::block_on(send(client, message, name)).unwrap();
    println!("body:{}", response.body)
}
