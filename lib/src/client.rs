use super::Expr;
use super::http::{Method, Scheme};
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct Clients {
    pub inner: HashMap<String, Client>,
}

impl Clients {
    pub fn get(&self, name: &str) -> Option<&Client> {
        self.inner.get(name)
    }

    pub fn insert(&mut self, client: Client) -> Result<(), String> {
        if self.inner.contains_key(&client.name) {
            return Err(format!("duplicate client '{}'", client.name));
        }
        self.inner.insert(client.name.clone(), client);
        Ok(())
    }

    pub fn extend(&mut self, clients: Clients) {
        self.inner.extend(clients.inner);
    }
}

#[derive(Debug)]
pub struct Client {
    pub name: String,
    pub scheme: Scheme,
    pub host: Expr,
    pub port: Option<u16>,
    pub requests: HashMap<String, Request>,
}

impl Client {
    pub fn request(&self, name: &str) -> Option<&Request> {
        self.requests.get(name)
    }
}

#[derive(Debug)]
pub struct Request {
    pub path: Expr,
    pub method: Method,
    pub params: Vec<(Expr, Expr)>,
    pub headers: Vec<(Expr, Expr)>,
    pub body: Option<Expr>,
    pub asserts: Vec<Expr>,
}
