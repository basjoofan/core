mod context;
mod evaluator;
mod http;
mod lexer;
mod native;
mod parser;
mod stat;
mod syntax;
mod token;
mod value;

use context::Assert;
use context::Record;
use syntax::Expr;
use token::Kind;
use token::Token;
use value::Value;

pub use context::Context;
pub use parser::Parser;
pub use stat::Stats;
pub use syntax::Source;

#[cfg(not(target_arch = "wasm32"))]
mod writer;
#[cfg(not(target_arch = "wasm32"))]
pub use writer::Writer;

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
pub mod tests {
    use axum::Router;
    use axum::extract::Form;
    use axum::extract::Json;
    use axum::extract::Multipart;
    use axum::extract::Query;
    use axum::http::header::HeaderMap;
    use axum::routing::get;
    use axum::routing::post;
    use serde_json::Value;
    use serde_json::json;
    use std::collections::HashMap;
    use std::net::ToSocketAddrs;
    use tokio::net::TcpListener;

    pub async fn start_server(port: u16) {
        let router = Router::new()
            .route("/get", get(handle_get))
            .route("/text", post(handle_text))
            .route("/json", post(handle_json))
            .route("/form", post(handle_form))
            .route("/multipart", post(handle_multipart));
        let addrs = ("localhost", port).to_socket_addrs().unwrap();
        for addr in addrs {
            let listener = TcpListener::bind(addr).await.unwrap();
            let router = router.to_owned();
            tokio::spawn(async move {
                axum::serve(listener, router).await.unwrap();
            });
        }
    }

    async fn handle_get(headers: HeaderMap, Query(params): Query<HashMap<String, String>>) -> Json<Value> {
        let headers = headers
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_str().unwrap().to_string()))
            .collect::<HashMap<String, String>>();
        Json(json!({ "headers": headers,"params": params}))
    }

    async fn handle_text(headers: HeaderMap, Query(params): Query<HashMap<String, String>>, text: String) -> Json<Value> {
        let headers = headers
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_str().unwrap().to_string()))
            .collect::<HashMap<String, String>>();
        Json(json!({ "headers": headers,"params": params,"text": text}))
    }

    async fn handle_json(headers: HeaderMap, Query(params): Query<HashMap<String, String>>, Json(json): Json<Value>) -> Json<Value> {
        let headers = headers
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_str().unwrap().to_string()))
            .collect::<HashMap<String, String>>();
        Json(json!({ "headers": headers,"params": params,"json": json}))
    }

    async fn handle_form(
        headers: HeaderMap,
        Query(params): Query<HashMap<String, String>>,
        Form(form): Form<HashMap<String, String>>,
    ) -> Json<Value> {
        let headers = headers
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_str().unwrap().to_string()))
            .collect::<HashMap<String, String>>();
        Json(json!({ "headers": headers,"params": params,"form": form}))
    }

    async fn handle_multipart(
        headers: HeaderMap,
        Query(params): Query<HashMap<String, String>>,
        mut multipart: Multipart,
    ) -> Json<Value> {
        let headers = headers
            .iter()
            .map(|(name, value)| (name.to_string(), value.to_str().unwrap().to_string()))
            .collect::<HashMap<String, String>>();
        let mut form = HashMap::new();
        while let Some(field) = multipart.next_field().await.unwrap() {
            let name = field.name().unwrap().to_string();
            let mut data = match field.file_name() {
                Some(file) => format!("@{file}|"),
                None => String::new(),
            };
            match field.text().await {
                Ok(text) => data.push_str(&text),
                Err(error) => data.push_str(&error.to_string()),
            }
            form.insert(name, data);
        }
        Json(json!({ "headers": headers,"params": params,"form": form}))
    }
}
