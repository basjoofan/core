use super::Scheme;

#[derive(Debug)]
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
                    (&host[..j], host[j + 1..].parse::<u16>().ok())
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
        Url { scheme, host, port, path }
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
        match (&self.scheme, &self.port) {
            (Scheme::Http, 80) | (Scheme::Https, 443) => write!(f, "{}://{}{}", self.scheme, self.host, self.path),
            _ => write!(f, "{}://{}:{}{}", self.scheme, self.host, self.port, self.path),
        }
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

const TABLE: &[u8; 768] = b"\
      %00%01%02%03%04%05%06%07%08%09%0A%0B%0C%0D%0E%0F\
      %10%11%12%13%14%15%16%17%18%19%1A%1B%1C%1D%1E%1F\
      %20%21%22%23%24%25%26%27%28%29%2A%2B%2C%2D%2E%2F\
      %30%31%32%33%34%35%36%37%38%39%3A%3B%3C%3D%3E%3F\
      %40%41%42%43%44%45%46%47%48%49%4A%4B%4C%4D%4E%4F\
      %50%51%52%53%54%55%56%57%58%59%5A%5B%5C%5D%5E%5F\
      %60%61%62%63%64%65%66%67%68%69%6A%6B%6C%6D%6E%6F\
      %70%71%72%73%74%75%76%77%78%79%7A%7B%7C%7D%7E%7F\
      %80%81%82%83%84%85%86%87%88%89%8A%8B%8C%8D%8E%8F\
      %90%91%92%93%94%95%96%97%98%99%9A%9B%9C%9D%9E%9F\
      %A0%A1%A2%A3%A4%A5%A6%A7%A8%A9%AA%AB%AC%AD%AE%AF\
      %B0%B1%B2%B3%B4%B5%B6%B7%B8%B9%BA%BB%BC%BD%BE%BF\
      %C0%C1%C2%C3%C4%C5%C6%C7%C8%C9%CA%CB%CC%CD%CE%CF\
      %D0%D1%D2%D3%D4%D5%D6%D7%D8%D9%DA%DB%DC%DD%DE%DF\
      %E0%E1%E2%E3%E4%E5%E6%E7%E8%E9%EA%EB%EC%ED%EE%EF\
      %F0%F1%F2%F3%F4%F5%F6%F7%F8%F9%FA%FB%FC%FD%FE%FF\
      ";

pub struct Serializer {
    target: Vec<u8>,
}

impl Serializer {
    pub fn new() -> Self {
        Self { target: Vec::new() }
    }

    pub fn append(&mut self, name: &str, value: &str) {
        self.percent_encode(name);
        self.target.push(b'=');
        self.percent_encode(value);
        self.target.push(b'&');
    }

    pub fn finish(mut self) -> Vec<u8> {
        self.target.pop();
        self.target
    }

    fn percent_encode(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'*' | b'-' | b'.' | b'_' => self.target.push(byte),
                _ => {
                    let index = (byte as usize) * 3;
                    self.target.extend_from_slice(&TABLE[index..index + 3])
                }
            }
        }
    }
}

#[test]
fn test_serializer() {
    let tests = vec![
        (vec![("a", "b"), ("c", "d")], "a=b&c=d"),
        (vec![("chinese", "中文")], "chinese=%E4%B8%AD%E6%96%87"),
        (vec![("a", "b c")], "a=b%20c"),
    ];

    for (test, expected) in tests {
        let mut serializer = Serializer::new();
        for (name, value) in test {
            serializer.append(name, value);
        }
        let encoded = serializer.finish();

        assert_eq!(String::from_utf8(encoded).unwrap(), expected);
    }
}
