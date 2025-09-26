use super::super::Client;
use super::super::Headers;
use super::super::Request;
use super::super::Response;
use super::Time;
use js_sys::Array;
use js_sys::Date;
use js_sys::Promise;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;
use web_sys::Request as WebRequest;
use web_sys::RequestInit;
use web_sys::Response as WebResponse;

impl Client {
    /// Send this request and wait for the record.
    pub async fn send(&self, message: &str) -> (Request, Response, Time, String) {
        let (request, content) = match Request::from(message) {
            Ok((request, content)) => (request, content),
            Err(error) => return (Request::default(), Response::default(), Time::default(), error.to_string()),
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
    let opts = RequestInit::new();
    opts.set_method(request.method.as_ref());
    if let Some(content) = content {
        opts.set_body(&content);
    }
    let web_request = WebRequest::new_with_str_and_init(request.url.to_string().as_str(), &opts)?;
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
            version: String::from("HTTP"),
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

    wasm_bindgen_test::wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    async fn default_headers() {
        let message = r#"
        GET https://httpbin.org/get
        Host: httpbin.org"#;
        let client = Client::default();
        let (request, response, time, error) = client.send(message).await;
        assert_eq!("GET", request.method.as_ref());
        assert_eq!(200, response.status);
        // assert_eq!(time.total, time.resolve + time.connect + time.write + time.delay + time.read);
        console::log_2(&JsValue::from_str("error: "), &JsValue::from_str(&error));
        console::log_2(&JsValue::from_str("time.total: "), &JsValue::from(&Number::from(time.total)));
        console::log_2(&JsValue::from_str("response.body: "), &JsValue::from_str(&response.body));
    }
}
