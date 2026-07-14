use super::Expr;
use std::collections::HashMap;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Apis {
    pub inner: HashMap<String, Api>,
}

impl Apis {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get(&self, name: &str) -> Option<&Api> {
        self.inner.get(name)
    }

    pub fn insert(&mut self, api: Api) -> Result<(), String> {
        if self.inner.contains_key(&api.name) {
            return Err(format!("duplicate api '{}'", api.name));
        }
        self.inner.insert(api.name.clone(), api);
        Ok(())
    }

    pub fn extend(&mut self, apis: Apis) {
        self.inner.extend(apis.inner);
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Api {
    pub name: String,
    pub scheme: Expr,
    pub host: Expr,
    pub port: Option<Expr>,
    pub headers: Vec<(Expr, Expr)>,
    pub requests: HashMap<String, Request>,
}

impl Api {
    pub fn request(&self, name: &str) -> Option<&Request> {
        self.requests.get(name)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Request {
    pub name: String,
    pub params_def: Vec<Parameter>,
    pub path: Expr,
    pub method: Method,
    pub params: Vec<(Expr, Expr)>,
    pub headers: Vec<(Expr, Expr)>,
    pub body: Body,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub kind: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Body {
    None,
    Json(Expr),
    Form(Vec<(Expr, Expr)>),
    Part(Vec<(Expr, Expr)>),
    Text(Expr),
    File(Expr),
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
    Query => "QUERY",
);

impl Method {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "OPTIONS" | "HEAD" | "TRACE"
            | "CONNECT" | "QUERY" => Ok(Self::from(value)),
            _ => Err(format!("unknown HTTP method '{value}'")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Method;

    #[test]
    fn parses_every_documented_method_and_rejects_unknown_values() {
        for method in [
            "GET", "QUERY", "POST", "PUT", "PATCH", "DELETE", "OPTIONS", "HEAD", "TRACE", "CONNECT",
        ] {
            assert_eq!(Method::parse(method).unwrap().as_ref(), method);
        }
        assert!(
            Method::parse("FETCH")
                .unwrap_err()
                .contains("unknown HTTP method")
        );
    }
}
