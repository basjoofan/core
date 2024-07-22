use crate::Error;

pub enum Method {
    Options,
    Get,
    Post,
    Put,
    Delete,
    Head,
    Trace,
    Connect,
    Patch,
}

/// Converts a &str to an http method.
impl std::convert::TryFrom<&str> for Method {
    type Error = Error;
    fn try_from(str: &str) -> Result<Self, Self::Error> {
        match str {
            "OPTIONS" => Ok(Method::Options),
            "GET" => Ok(Method::Get),
            "POST" => Ok(Method::Post),
            "PUT" => Ok(Method::Put),
            "DELETE" => Ok(Method::Delete),
            "HEAD" => Ok(Method::Head),
            "TRACE" => Ok(Method::Trace),
            "CONNECT" => Ok(Method::Connect),
            "PATCH" => Ok(Method::Patch),
            _ => Err(Error::InvalidMethod),
        }
    }
}

/// Return a &str representation of the http method
impl std::convert::AsRef<str> for Method {
    fn as_ref(&self) -> &str {
        match self {
            Method::Options => "OPTIONS",
            Method::Get => "GET",
            Method::Post => "POST",
            Method::Put => "PUT",
            Method::Delete => "DELETE",
            Method::Head => "HEAD",
            Method::Trace => "TRACE",
            Method::Connect => "CONNECT",
            Method::Patch => "PATCH",
        }
    }
}

impl std::fmt::Debug for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_ref())
    }
}

impl Default for Method {
    fn default() -> Method {
        Method::Get
    }
}

#[test]
fn test_from_str() {
    let method = Method::try_from("GET");
    println!("method:{:?}", method)
}
