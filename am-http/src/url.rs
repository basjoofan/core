use crate::Scheme;

pub struct Url {
    pub scheme: Scheme,
    pub host: String,
    pub port: u16,
    pub path: String,
}

impl std::convert::From<&str> for Url {
    fn from(str: &str) -> Self {
        let mut splits = str.split("://");
        let scheme = Scheme::from(splits.next());
        let (host, port, path) = match splits.next() {
            Some(str) => {
                let (host, path) = if let Some(i) = str.find('/') {
                    (&str[..i], &str[i..])
                } else {
                    (str, "/")
                };
                let (host, port) = if let Some(j) = host.find(':') {
                    (&host[..j], host[j..].parse::<u16>().ok())
                } else {
                    (host, None)
                };
                (host.to_string(), port, path.to_string())
            }
            None => ("localhost".to_string(), None, "/".to_string()),
        };
        let port = match port {
            Some(port) => port,
            None => match scheme {
                Scheme::Http => 80,
                Scheme::Https => 443,
            },
        };
        Url {
            scheme,
            host,
            port,
            path,
        }
    }
}

impl std::convert::From<Option<&str>> for Url {
    fn from(str: Option<&str>) -> Self {
        match str {
            Some(str) => Self::from(str),
            None => Self::default(),
        }
    }
}

impl Default for Url {
    fn default() -> Self {
        Self {
            scheme: Scheme::default(),
            host: "localhost".to_string(),
            port: 80,
            path: "/".to_string(),
        }
    }
}

impl std::fmt::Display for Url {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}://{}{}", self.scheme, self.host, self.path)
    }
}
