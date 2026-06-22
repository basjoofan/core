use super::Expr;
use std::collections::HashMap;

#[derive(Clone, Debug, Default)]
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

#[derive(Clone, Debug)]
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

#[derive(Clone, Debug)]
pub struct Request {
    pub path: Expr,
    pub method: Method,
    pub params: Vec<(Expr, Expr)>,
    pub headers: Vec<(Expr, Expr)>,
    pub body: Option<Expr>,
    pub asserts: Vec<Expr>,
}

macro_rules! protocol_type {
    ($type: ident, $default: ident => $default_value: literal, $($name: ident => $value: literal),+ $(,)?) => {
        #[derive(Clone, Debug, Default, Eq, PartialEq)]
        pub enum $type {
            #[default]
            $default,
            $($name),+
        }

        impl From<&str> for $type {
            fn from(value: &str) -> Self {
                match value {
                    $default_value => Self::$default,
                    $($value => Self::$name,)+
                    _ => Self::default(),
                }
            }
        }

        impl AsRef<str> for $type {
            fn as_ref(&self) -> &str {
                match self {
                    Self::$default => $default_value,
                    $(Self::$name => $value,)+
                }
            }
        }
    };
}

protocol_type!(Scheme, Http => "http", Https => "https");
protocol_type!(
    Method,
    Get => "GET",
    Post => "POST",
    Put => "PUT",
    Patch => "PATCH",
    Delete => "DELETE",
    Options => "OPTIONS",
    Head => "HEAD",
    Trace => "TRACE",
    Connect => "CONNECT",
);
