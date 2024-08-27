use super::Scheme;

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
                    (&host[..j], (&host[j + 1..]).parse::<u16>().ok())
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

#[test]
fn test_url_with_ip() {
    let str = "http://IP_ADDRESS";
    let url = Url::from(str);
    assert_eq!(url.scheme, Scheme::Http);
    assert_eq!(url.host, "IP_ADDRESS");
    assert_eq!(url.port, 80);
    assert_eq!(url.path, "/");
}

#[test]
fn test_url_with_ip_port() {
    let str = "http://IP_ADDRESS:52831";
    let url = Url::from(str);
    assert_eq!(url.scheme, Scheme::Http);
    assert_eq!(url.host, "IP_ADDRESS");
    assert_eq!(url.port, 52831);
    assert_eq!(url.path, "/");
}

#[test]
fn test_url_with_ip_path() {
    let str = "http://IP_ADDRESS/hello";
    let url = Url::from(str);
    assert_eq!(url.scheme, Scheme::Http);
    assert_eq!(url.host, "IP_ADDRESS");
    assert_eq!(url.port, 80);
    assert_eq!(url.path, "/hello");
}

#[test]
fn test_url_with_ip_port_path() {
    let str = "http://IP_ADDRESS:52831/hello/world";
    let url = Url::from(str);
    assert_eq!(url.scheme, Scheme::Http);
    assert_eq!(url.host, "IP_ADDRESS");
    assert_eq!(url.port, 52831);
    assert_eq!(url.path, "/hello/world");
}

#[test]
fn test_url_with_ip_path_query() {
    let str = "http://IP_ADDRESS/hello/world?a=b";
    let url = Url::from(str);
    assert_eq!(url.scheme, Scheme::Http);
    assert_eq!(url.host, "IP_ADDRESS");
    assert_eq!(url.port, 80);
    assert_eq!(url.path, "/hello/world?a=b");
}

#[test]
fn test_url_with_ip_port_path_query() {
    let str = "http://IP_ADDRESS:52831/hello/world?a=b";
    let url = Url::from(str);
    assert_eq!(url.scheme, Scheme::Http);
    assert_eq!(url.host, "IP_ADDRESS");
    assert_eq!(url.port, 52831);
    assert_eq!(url.path, "/hello/world?a=b");
}

#[test]
fn test_url_with_ip_path_query_fragment() {
    let str = "http://IP_ADDRESS/hello/world?a=b#fragment";
    let url = Url::from(str);
    assert_eq!(url.scheme, Scheme::Http);
    assert_eq!(url.host, "IP_ADDRESS");
    assert_eq!(url.port, 80);
    assert_eq!(url.path, "/hello/world?a=b#fragment");
}

#[test]
fn test_url_with_ip_port_path_query_fragment() {
    let str = "http://IP_ADDRESS:52831/hello/world?a=b#fragment";
    let url = Url::from(str);
    assert_eq!(url.scheme, Scheme::Http);
    assert_eq!(url.host, "IP_ADDRESS");
    assert_eq!(url.port, 52831);
    assert_eq!(url.path, "/hello/world?a=b#fragment");
}