use super::super::mime;
use super::super::Headers;
use super::super::Method;
use super::super::Request;
use super::super::Url;
use super::super::Version;
use js_sys::Array;
use js_sys::Promise;
use js_sys::Uint8Array;
use std::path::Path;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::JsValue;
use wasm_bindgen_futures::JsFuture;
use web_sys::Blob;
use web_sys::BlobPropertyBag;
use web_sys::FormData;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_name = readFileContent)]
    pub fn read_file_content(path: &str) -> Promise;
}

impl Request {
    /// Converts a message to an http request.
    pub async fn from(message: &str, base: &str) -> Result<(Request, Option<JsValue>), JsValue> {
        let mut lines = message.trim().lines();
        if let Some(line) = lines.next() {
            let mut splits = line.split_whitespace();
            let method = Method::from(splits.next());
            let url = Url::from(splits.next());
            let version = Version::from(splits.next());
            let mut content_type = None;
            let mut headers = Headers::default();
            for line in lines.by_ref() {
                if line.trim().is_empty() {
                    break;
                } else if let Some((name, value)) = line.trim().split_once(':') {
                    let name = name.trim();
                    let value = value.trim();
                    if content_type.is_none() && name.to_lowercase() == "content-type" {
                        content_type = Some(value);
                    }
                    if name.to_lowercase() != "content-type" || value.to_lowercase() != "multipart/form-data" {
                        headers.insert(name.to_string(), value.to_string());
                    }
                }
            }
            let mut body = String::default();
            let mut content = None;
            match content_type {
                Some("application/x-www-form-urlencoded") => {
                    let mut serializer = form_urlencoded::Serializer::new(String::default());
                    for line in lines.by_ref() {
                        if let Some((name, value)) = line.trim().split_once(':') {
                            serializer.append_pair(name, value);
                            body.push_str(line);
                        }
                    }
                    let bytes = serializer.finish().into_bytes();
                    content = Some(JsValue::from(Uint8Array::from(bytes.as_slice())));
                }
                Some("multipart/form-data") => {
                    let form = FormData::new()?;
                    for line in lines.by_ref() {
                        if let Some((name, value)) = line.trim().split_once(':') {
                            let (name, value) = (name.trim(), value.trim());
                            if value.starts_with("@") {
                                let path = Path::new(base).join(&value[1..value.len()]);
                                let uint8_array = match &path.as_os_str().to_str() {
                                    Some(path) => JsFuture::from(Promise::resolve(&read_file_content(path))).await?,
                                    None => return Err(JsValue::from_str("file path is not valid")),
                                };
                                let array = Array::new();
                                array.push(&JsValue::from(uint8_array));
                                let blob = match mime::from_path(&path) {
                                    Some(mime) => {
                                        let properties = BlobPropertyBag::new();
                                        properties.set_type(mime.as_ref());
                                        Blob::new_with_u8_array_sequence_and_options(&array, &properties)
                                    }
                                    None => Blob::new_with_u8_array_sequence(&array),
                                }?;
                                match path.file_name().and_then(|os_str| os_str.to_str()) {
                                    Some(file_name) => form.append_with_blob_and_filename(name, &blob, file_name)?,
                                    None => form.append_with_blob(name, &blob)?,
                                };
                            } else {
                                form.append_with_str(name, value)?;
                            }
                            body.push_str(line);
                        }
                    }
                    content = Some(JsValue::from(form));
                }
                _ => {
                    body = String::from_iter(lines);
                    if !body.trim().is_empty() {
                        content = Some(JsValue::from_str(&body));
                    }
                }
            }
            Ok((
                Request {
                    method,
                    url,
                    version,
                    headers,
                    body,
                },
                content,
            ))
        } else {
            Ok((Request::default(), None))
        }
    }
}
