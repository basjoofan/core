use super::super::Client;
use super::super::Headers;
use super::super::Request;
use super::super::Response;
use super::Time;
use js_sys::Array;
use js_sys::Date;
use js_sys::Promise;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::Request as WebRequest;
use web_sys::RequestCache;
use web_sys::RequestInit;
use web_sys::RequestMode;
use web_sys::RequestRedirect;
use web_sys::Response as WebResponse;

impl Client {
    /// Send this request and wait for the record.
    pub async fn send(&self, message: &str) -> (Request, Response, Time, String) {
        let (request, content) = match Request::from(message).await {
            Ok((request, content)) => (request, content),
            Err(error) => {
                return (
                    Request::default(),
                    Response::default(),
                    Time::default(),
                    error.as_string().unwrap_or_default(),
                )
            }
        };
        let (response, time) = match fetch(&request, content).await {
            Ok(response) => response,
            Err(error) => return (request, Response::default(), Time::default(), error.as_string().unwrap_or_default()),
        };
        (request, response, time, String::default())
    }
}

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = fetch)]
    fn fetch_with_request(request: &WebRequest) -> Promise;
}

async fn fetch(request: &Request, content: Option<JsValue>) -> Result<(Response, Time), JsValue> {
    let init = RequestInit::new();
    init.set_mode(RequestMode::Cors);
    init.set_cache(RequestCache::NoStore);
    init.set_redirect(RequestRedirect::Error);
    init.set_method(request.method.as_ref());
    if let Some(content) = content {
        init.set_body(&content);
    }
    let web_request = WebRequest::new_with_str_and_init(request.url.to_string().as_str(), &init)?;
    for header in request.headers.iter() {
        web_request.headers().set(&header.name, &header.value)?;
    }
    let mut time = Time::default();
    let start = Date::now();
    time.start = start;
    let resp_value = JsFuture::from(Promise::resolve(&fetch_with_request(&web_request))).await?;
    let web_response: WebResponse = resp_value.dyn_into()?;
    let body = JsFuture::from(web_response.text()?).await?.as_string().unwrap_or_default();
    let mut headers = Headers::default();
    for header in web_response.headers().entries() {
        match header {
            Ok(header) => {
                let header = Array::from(&header);
                let name = header.shift();
                let value = header.shift();
                headers.insert(name.as_string().unwrap_or_default(), value.as_string().unwrap_or_default())
            }
            Err(error) => return Err(error),
        }
    }
    let end = Date::now();
    time.end = end;
    time.total = end - start;
    Ok((
        Response {
            version: String::from("HTTP/1.1"),
            status: web_response.status(),
            reason: web_response.status_text(),
            headers,
            body,
        },
        time,
    ))
}

#[cfg(test)]
mod tests {
    use crate::http::Client;
    use js_sys::Number;
    use wasm_bindgen::JsValue;
    use wasm_bindgen_test::*;
    use web_sys::console;

    // wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    fn setup() {
        let _ = js_sys::eval(
            r#"
        globalThis.readFileContent = async function(filePath) {
            console.log(`Mock reading file in test: ${filePath}`);
            return new TextEncoder().encode(`Test content for ${filePath}`);
        };
        "#,
        );
        let _ = js_sys::eval(
            r#"
        window.readFileContent = async function(filePath) {
            console.log(`Mock reading file in test: ${filePath}`);
            return new TextEncoder().encode(`Test content for ${filePath}`);
        };
        "#,
        );
    }

    #[wasm_bindgen_test]
    async fn test_send_message_get() {
        let message = r#"
        GET https://httpbingo.org/get
        Host: httpbingo.org"#;
        let client = Client::default();
        let (request, response, time, error) = client.send(message).await;
        console::log_2(&JsValue::from_str("error: "), &JsValue::from_str(&error));
        console::log_2(&JsValue::from_str("time.total: "), &JsValue::from(&Number::from(time.total)));
        console::log_2(&JsValue::from_str("response.body: "), &JsValue::from_str(&response.body));
        assert_eq!("GET", request.method.as_ref());
        assert_eq!(200, response.status);
        // assert_eq!(time.total, time.resolve + time.connect + time.write + time.delay + time.read);
    }

    #[wasm_bindgen_test]
    async fn test_send_message_post() {
        let message = r#"
        POST https://httpbingo.org/post
        Host: httpbingo.org
        Accept-Encoding: gzip, deflate"#;
        let client = Client::default();
        let (request, response, time, error) = client.send(message).await;
        console::log_2(&JsValue::from_str("error: "), &JsValue::from_str(&error));
        console::log_2(&JsValue::from_str("time.total: "), &JsValue::from(&Number::from(time.total)));
        console::log_2(&JsValue::from_str("response.body: "), &JsValue::from_str(&response.body));
        assert_eq!("POST", request.method.as_ref());
        assert_eq!(200, response.status);
        // assert_eq!(time.total, time.resolve + time.connect + time.write + time.delay + time.read);
    }

    #[wasm_bindgen_test]
    async fn test_send_message_post_form() {
        let message = r#"
        POST https://httpbingo.org/post
        Host: httpbingo.org
        Content-Type: application/x-www-form-urlencoded

        a: b"#;
        let client = Client::default();
        let (request, response, time, error) = client.send(message).await;
        console::log_2(&JsValue::from_str("error: "), &JsValue::from_str(&error));
        console::log_2(&JsValue::from_str("time.total: "), &JsValue::from(&Number::from(time.total)));
        console::log_2(&JsValue::from_str("response.body: "), &JsValue::from_str(&response.body));
        assert_eq!("POST", request.method.as_ref());
        assert_eq!(200, response.status);
        // assert_eq!(time.total, time.resolve + time.connect + time.write + time.delay + time.read);
    }

    #[wasm_bindgen_test]
    async fn test_send_message_post_multipart() {
        setup();
        let message = r#"
        POST https://httpbingo.org/post
        Host: httpbingo.org
        Content-Type: multipart/form-data

        a: b
        f: @src/text.txt"#;
        let client = Client::default();
        let (request, response, time, error) = client.send(message).await;
        console::log_2(&JsValue::from_str("error: "), &JsValue::from_str(&error));
        console::log_2(&JsValue::from_str("time.total: "), &JsValue::from(&Number::from(time.total)));
        console::log_2(&JsValue::from_str("response.body: "), &JsValue::from_str(&response.body));
        assert_eq!("POST", request.method.as_ref());
        assert_eq!(200, response.status);
        // assert_eq!(time.total, time.resolve + time.connect + time.write + time.delay + time.read);
    }

    #[wasm_bindgen_test]
    async fn test_send_message_post_json() {
        let message = r#"
        POST https://httpbingo.org/post
        Host: httpbingo.org
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
        console::log_2(&JsValue::from_str("error: "), &JsValue::from_str(&error));
        console::log_2(&JsValue::from_str("time.total: "), &JsValue::from(&Number::from(time.total)));
        console::log_2(&JsValue::from_str("response.body: "), &JsValue::from_str(&response.body));
        assert_eq!("POST", request.method.as_ref());
        assert_eq!(200, response.status);
        // assert_eq!(time.total, time.resolve + time.connect + time.write + time.delay + time.read);
    }
}
