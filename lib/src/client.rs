use super::Expr;
use super::Parser as ExprParser;
use super::context::Context;
use super::http::Method;
use super::http::Scheme;
use super::http::Serializer;
use super::native;
use std::collections::HashMap;
use std::fmt;
use std::path::Path;

#[derive(Debug, Default)]
pub struct Clients {
    pub inner: HashMap<String, Client>,
}

impl Clients {
    pub fn from_str(input: &str) -> Result<Self, ParseError> {
        Yaml::new(input)?.parse_clients()
    }

    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ParseError> {
        let path = path.as_ref();
        let input = std::fs::read_to_string(path).map_err(|error| {
            ParseError::new(0, format!("failed to read {}: {error}", path.display()))
        })?;
        Self::from_str(&input)
    }

    pub fn get(&self, name: &str) -> Option<&Client> {
        self.inner.get(name)
    }

    pub fn extend(&mut self, clients: Clients) {
        self.inner.extend(clients.inner);
    }

    pub fn try_extend(&mut self, clients: Clients) -> Result<(), ParseError> {
        for name in clients.inner.keys() {
            if self.inner.contains_key(name) {
                return Err(ParseError::new(0, format!("duplicate client '{name}'")));
            }
        }
        self.extend(clients);
        Ok(())
    }
}

#[derive(Debug)]
pub struct Client {
    pub name: String,
    pub scheme: Scheme,
    pub host: String,
    pub port: Option<u16>,
    pub requests: HashMap<String, Request>,
}

impl Client {
    pub fn request(&self, name: &str) -> Option<&Request> {
        self.requests.get(name)
    }

    pub fn render(&self, request: &Request, context: &Context) -> String {
        let host = native::format_template(&self.host, context);
        let path = native::format_template(&request.path, context);
        let mut url = format!("{}://{}", self.scheme.as_ref(), host);
        if let Some(port) = self.port {
            url.push_str(&format!(":{port}"));
        }
        url.push_str(&path);
        if !request.params.is_empty() {
            let mut serializer = Serializer::new();
            for (key, value) in &request.params {
                serializer.append(key, &native::format_template(value, context));
            }
            url.push('?');
            url.push_str(&String::from_utf8_lossy(&serializer.finish()));
        }

        let mut message = format!("{} {url}\n", request.method.as_ref());
        for (key, value) in &request.headers {
            message.push_str(key);
            message.push_str(": ");
            message.push_str(&native::format_template(value, context));
            message.push('\n');
        }
        match &request.body {
            Body::None => {}
            Body::Form(items) => {
                message.push('\n');
                for (key, value) in items {
                    message.push_str(key);
                    message.push_str(": ");
                    message.push_str(&native::format_template(value, context));
                    message.push('\n');
                }
            }
            Body::Text(body) => {
                message.push('\n');
                message.push_str(&native::format_template(body, context));
            }
        }
        message
    }
}

#[derive(Debug)]
pub struct Request {
    pub path: String,
    pub method: Method,
    pub params: Vec<(String, String)>,
    pub headers: Vec<(String, String)>,
    pub body: Body,
    pub asserts: Vec<Expr>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Body {
    None,
    Form(Vec<(String, String)>),
    Text(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    pub line: usize,
    pub reason: String,
}

impl ParseError {
    fn new(line: usize, reason: impl Into<String>) -> Self {
        Self {
            line,
            reason: reason.into(),
        }
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 {
            f.write_str(&self.reason)
        } else {
            write!(f, "line {}: {}", self.line, self.reason)
        }
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug)]
struct Line {
    number: usize,
    indent: usize,
    raw: String,
    text: String,
}

struct Yaml {
    lines: Vec<Line>,
    index: usize,
}

impl Yaml {
    fn new(input: &str) -> Result<Self, ParseError> {
        let mut lines = Vec::new();
        for (index, raw) in input.lines().enumerate() {
            let line = raw.strip_suffix('\r').unwrap_or(raw).trim_end();
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let indent = count_indent(line).ok_or_else(|| {
                ParseError::new(index + 1, "tabs are not allowed for indentation")
            })?;
            lines.push(Line {
                number: index + 1,
                indent,
                raw: line.to_string(),
                text: line[indent..].trim_end().to_string(),
            });
        }
        Ok(Self { lines, index: 0 })
    }

    fn parse_clients(mut self) -> Result<Clients, ParseError> {
        let mut clients = Clients::default();
        self.skip_document_separators();
        while !self.is_eof() {
            let start_line = self.current().map(|line| line.number).unwrap_or(1);
            let client = self.parse_client(start_line)?;
            if clients.inner.contains_key(&client.name) {
                return Err(ParseError::new(
                    start_line,
                    format!("duplicate client '{}'", client.name),
                ));
            }
            clients.inner.insert(client.name.clone(), client);
            self.skip_document_separators();
        }
        Ok(clients)
    }

    fn parse_client(&mut self, start_line: usize) -> Result<Client, ParseError> {
        let mut name = None;
        let mut scheme = None;
        let mut host = None;
        let mut port = None;
        let mut requests = None;

        while let Some(line) = self.current() {
            if self.is_document_separator() {
                break;
            }
            if line.indent != 0 {
                return Err(ParseError::new(
                    line.number,
                    "expected a top-level client field",
                ));
            }

            let (key, value) = parse_key_value(line, "top-level client field")?;
            let line_number = line.number;
            match key {
                "name" => {
                    ensure_inline_value(line_number, key, value)?;
                    name = Some(value.to_string());
                    self.index += 1;
                }
                "scheme" => {
                    ensure_inline_value(line_number, key, value)?;
                    scheme = Some(parse_scheme(line_number, value)?);
                    self.index += 1;
                }
                "host" => {
                    ensure_inline_value(line_number, key, value)?;
                    host = Some(value.to_string());
                    self.index += 1;
                }
                "port" => {
                    ensure_inline_value(line_number, key, value)?;
                    port = Some(value.parse::<u16>().map_err(|_| {
                        ParseError::new(line_number, format!("invalid port '{value}'"))
                    })?);
                    self.index += 1;
                }
                "requests" => {
                    if !value.is_empty() {
                        return Err(ParseError::new(
                            line_number,
                            "requests must be a nested list",
                        ));
                    }
                    let indent = line.indent;
                    self.index += 1;
                    requests = Some(self.parse_requests(indent)?);
                }
                _ => {
                    return Err(ParseError::new(
                        line_number,
                        format!("unknown client field '{key}'"),
                    ));
                }
            }
        }

        let name =
            name.ok_or_else(|| ParseError::new(start_line, "missing required field 'name'"))?;
        let scheme =
            scheme.ok_or_else(|| ParseError::new(start_line, "missing required field 'scheme'"))?;
        let host =
            host.ok_or_else(|| ParseError::new(start_line, "missing required field 'host'"))?;
        let requests = requests
            .ok_or_else(|| ParseError::new(start_line, "missing required field 'requests'"))?;

        Ok(Client {
            name,
            scheme,
            host,
            port,
            requests,
        })
    }

    fn parse_requests(
        &mut self,
        parent_indent: usize,
    ) -> Result<HashMap<String, Request>, ParseError> {
        let list_indent = self.expect_child_indent(parent_indent, "request list")?;
        if list_indent != parent_indent + 2 {
            return Err(ParseError::new(
                self.current().map(|line| line.number).unwrap_or(1),
                format!("invalid indentation, expected {} spaces", parent_indent + 2),
            ));
        }
        let mut requests = HashMap::new();

        while let Some(line) = self.current() {
            if line.indent <= parent_indent || self.is_document_separator() {
                break;
            }
            if line.indent != list_indent {
                return Err(ParseError::new(
                    line.number,
                    format!("invalid indentation, expected {list_indent} spaces"),
                ));
            }
            let rest = list_item(line, "request entry")?;
            let (name, value) = split_key_value(rest).ok_or_else(|| {
                ParseError::new(line.number, "request entry must be a single-key map")
            })?;
            if !value.is_empty() {
                return Err(ParseError::new(
                    line.number,
                    "request entry must contain nested fields",
                ));
            }
            let request_name = name.to_string();
            if requests.contains_key(&request_name) {
                return Err(ParseError::new(
                    line.number,
                    format!("duplicate request '{request_name}'"),
                ));
            }
            let request_line = line.number;
            self.index += 1;
            let request = self.parse_request(list_indent, &request_name, request_line)?;
            requests.insert(request_name, request);
        }

        if requests.is_empty() {
            let line = self.previous_line_number();
            return Err(ParseError::new(
                line,
                "requests must contain at least one entry",
            ));
        }

        Ok(requests)
    }

    fn parse_request(
        &mut self,
        parent_indent: usize,
        request_name: &str,
        start_line: usize,
    ) -> Result<Request, ParseError> {
        let mut path = None;
        let mut method = None;
        let mut params = Vec::new();
        let mut headers = Vec::new();
        let mut body = Body::None;
        let mut asserts = Vec::new();

        while let Some(line) = self.current() {
            if line.indent <= parent_indent || self.is_document_separator() {
                break;
            }
            if line.indent != parent_indent + 4 {
                return Err(ParseError::new(
                    line.number,
                    format!("invalid indentation in request '{request_name}'"),
                ));
            }
            let (key, value) = parse_key_value(line, "request field")?;
            let line_number = line.number;
            match key {
                "path" => {
                    ensure_inline_value(line_number, key, value)?;
                    path = Some(value.to_string());
                    self.index += 1;
                }
                "method" => {
                    ensure_inline_value(line_number, key, value)?;
                    method = Some(parse_method(line_number, value)?);
                    self.index += 1;
                }
                "params" => {
                    if !value.is_empty() {
                        return Err(ParseError::new(line_number, "params must be a nested list"));
                    }
                    let indent = line.indent;
                    self.index += 1;
                    params = self.parse_key_value_list(indent, "params")?;
                }
                "headers" => {
                    if !value.is_empty() {
                        return Err(ParseError::new(
                            line_number,
                            "headers must be a nested list",
                        ));
                    }
                    let indent = line.indent;
                    self.index += 1;
                    headers = self.parse_key_value_list(indent, "headers")?;
                }
                "body" => {
                    let indent = line.indent;
                    let body_value = value.to_string();
                    self.index += 1;
                    body = self.parse_body(indent, line_number, &body_value)?;
                }
                "asserts" => {
                    if !value.is_empty() {
                        return Err(ParseError::new(
                            line_number,
                            "asserts must be a nested list",
                        ));
                    }
                    let indent = line.indent;
                    self.index += 1;
                    asserts = self.parse_assert_list(indent)?;
                }
                _ => {
                    return Err(ParseError::new(
                        line_number,
                        format!("unknown request field '{key}'"),
                    ));
                }
            }
        }

        Ok(Request {
            path: path.ok_or_else(|| {
                ParseError::new(
                    start_line,
                    format!("request '{request_name}' is missing 'path'"),
                )
            })?,
            method: method.ok_or_else(|| {
                ParseError::new(
                    start_line,
                    format!("request '{request_name}' is missing 'method'"),
                )
            })?,
            params,
            headers,
            body,
            asserts,
        })
    }

    fn parse_key_value_list(
        &mut self,
        parent_indent: usize,
        field: &str,
    ) -> Result<Vec<(String, String)>, ParseError> {
        let Some(list_indent) = self.child_indent(parent_indent) else {
            return Ok(Vec::new());
        };
        if list_indent != parent_indent + 2 {
            return Err(ParseError::new(
                self.current().map(|line| line.number).unwrap_or(1),
                format!(
                    "invalid indentation in {field}, expected {} spaces",
                    parent_indent + 2
                ),
            ));
        }
        let mut pairs = Vec::new();
        while let Some(line) = self.current() {
            if line.indent <= parent_indent || self.is_document_separator() {
                break;
            }
            if line.indent != list_indent {
                return Err(ParseError::new(
                    line.number,
                    format!("invalid indentation in {field}, expected {list_indent} spaces"),
                ));
            }
            let rest = list_item(line, "asserts")?;
            let (key, value) = split_key_value(rest).ok_or_else(|| {
                ParseError::new(
                    line.number,
                    format!("{field} entry must be a single-key map"),
                )
            })?;
            pairs.push((key.to_string(), value.to_string()));
            self.index += 1;
        }
        Ok(pairs)
    }

    fn parse_assert_list(&mut self, parent_indent: usize) -> Result<Vec<Expr>, ParseError> {
        let Some(list_indent) = self.child_indent(parent_indent) else {
            return Ok(Vec::new());
        };
        if list_indent != parent_indent + 2 {
            return Err(ParseError::new(
                self.current().map(|line| line.number).unwrap_or(1),
                format!(
                    "invalid indentation in asserts, expected {} spaces",
                    parent_indent + 2
                ),
            ));
        }
        let mut values = Vec::new();
        while let Some(line) = self.current() {
            if line.indent <= parent_indent || self.is_document_separator() {
                break;
            }
            if line.indent != list_indent {
                return Err(ParseError::new(
                    line.number,
                    format!("invalid indentation in asserts, expected {list_indent} spaces"),
                ));
            }
            let rest = list_item(line, "asserts")?;
            if rest.is_empty() {
                return Err(ParseError::new(
                    line.number,
                    "asserts entry must not be empty",
                ));
            }
            values.push(parse_assert_expr(line.number, rest)?);
            self.index += 1;
        }
        Ok(values)
    }

    fn parse_body(
        &mut self,
        parent_indent: usize,
        line_number: usize,
        value: &str,
    ) -> Result<Body, ParseError> {
        if value == "|" {
            return Ok(Body::Text(self.collect_text_block(parent_indent)?));
        }
        if !value.is_empty() {
            return Ok(Body::Text(value.to_string()));
        }
        let Some(line) = self.current() else {
            return Ok(Body::None);
        };
        if line.indent <= parent_indent || self.is_document_separator() {
            return Ok(Body::None);
        }
        if line.text.starts_with("- ") {
            return Ok(Body::Form(
                self.parse_key_value_list(parent_indent, "body")?,
            ));
        }
        if line.indent == parent_indent + 2 {
            return Ok(Body::Text(self.collect_text_block(parent_indent)?));
        }
        Err(ParseError::new(
            line_number,
            "body must be a nested key-value list or text block",
        ))
    }

    fn collect_text_block(&mut self, parent_indent: usize) -> Result<String, ParseError> {
        let block_indent = self.expect_child_indent(parent_indent, "text block")?;
        if block_indent != parent_indent + 2 {
            return Err(ParseError::new(
                self.current().map(|line| line.number).unwrap_or(1),
                format!(
                    "invalid indentation in text block, expected {} spaces",
                    parent_indent + 2
                ),
            ));
        }
        let mut text = String::new();
        while let Some(line) = self.current() {
            if line.indent <= parent_indent || self.is_document_separator() {
                break;
            }
            if line.indent < block_indent {
                return Err(ParseError::new(
                    line.number,
                    format!(
                        "invalid indentation in text block, expected at least {block_indent} spaces"
                    ),
                ));
            }
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&line.raw[block_indent..]);
            self.index += 1;
        }
        Ok(text)
    }

    fn expect_child_indent(&self, parent_indent: usize, what: &str) -> Result<usize, ParseError> {
        self.child_indent(parent_indent).ok_or_else(|| {
            ParseError::new(
                self.previous_line_number(),
                format!("{what} must contain nested entries"),
            )
        })
    }

    fn child_indent(&self, parent_indent: usize) -> Option<usize> {
        self.current().and_then(|line| {
            if line.indent > parent_indent && !self.is_document_separator() {
                Some(line.indent)
            } else {
                None
            }
        })
    }

    fn current(&self) -> Option<&Line> {
        self.lines.get(self.index)
    }

    fn is_eof(&self) -> bool {
        self.index >= self.lines.len()
    }

    fn is_document_separator(&self) -> bool {
        self.current()
            .is_some_and(|line| line.indent == 0 && line.text == "---")
    }

    fn skip_document_separators(&mut self) {
        while self.is_document_separator() {
            self.index += 1;
        }
    }

    fn previous_line_number(&self) -> usize {
        self.index
            .checked_sub(1)
            .and_then(|index| self.lines.get(index))
            .map(|line| line.number)
            .unwrap_or(1)
    }
}

fn count_indent(line: &str) -> Option<usize> {
    let mut count = 0;
    for byte in line.bytes() {
        match byte {
            b' ' => count += 1,
            b'\t' => return None,
            _ => break,
        }
    }
    Some(count)
}

fn parse_key_value<'a>(line: &'a Line, what: &str) -> Result<(&'a str, &'a str), ParseError> {
    split_key_value(&line.text)
        .ok_or_else(|| ParseError::new(line.number, format!("{what} must be 'key: value'")))
}

fn split_key_value(text: &str) -> Option<(&str, &str)> {
    let (key, value) = text.split_once(':')?;
    let key = key.trim();
    if key.is_empty() {
        return None;
    }
    Some((key, value.trim()))
}

fn list_item<'a>(line: &'a Line, field: &str) -> Result<&'a str, ParseError> {
    line.text
        .strip_prefix("- ")
        .map(str::trim)
        .ok_or_else(|| ParseError::new(line.number, format!("{field} entry must start with '- '")))
}

fn ensure_inline_value(line: usize, key: &str, value: &str) -> Result<(), ParseError> {
    if value.is_empty() {
        Err(ParseError::new(line, format!("{key} must have a value")))
    } else {
        Ok(())
    }
}

fn parse_scheme(line: usize, value: &str) -> Result<Scheme, ParseError> {
    match value {
        "http" | "https" => Ok(Scheme::from(value)),
        _ => Err(ParseError::new(line, format!("unknown scheme '{value}'"))),
    }
}

fn parse_method(line: usize, value: &str) -> Result<Method, ParseError> {
    match value {
        "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "OPTIONS" | "HEAD" | "TRACE" | "CONNECT" => {
            Ok(Method::from(value))
        }
        _ => Err(ParseError::new(line, format!("unknown method '{value}'"))),
    }
}

fn parse_assert_expr(line: usize, value: &str) -> Result<Expr, ParseError> {
    let mut parser = ExprParser::new(value);
    let source = parser
        .parse()
        .map_err(|error| ParseError::new(line, format!("invalid assert expression: {error}")))?;
    let mut exprs = source.exprs.into_iter();
    let expr = exprs
        .next()
        .ok_or_else(|| ParseError::new(line, "assert expression must not be empty"))?;
    if exprs.next().is_some() {
        return Err(ParseError::new(
            line,
            "assert expression must contain exactly one expression",
        ));
    }
    Ok(expr)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_single_get_request() {
        let clients = Clients::from_str(
            r#"
name: user
scheme: https
host: httpbin.org
requests:
  - get:
      path: /get
      method: GET
      headers:
        - a: b
      params:
        - key: value
        - variable: hello \(variable)
      asserts:
        - status == 200
"#,
        )
        .unwrap();

        let client = clients.get("user").unwrap();
        let request = client.request("get").unwrap();
        assert_eq!("user", client.name);
        assert_eq!("https", client.scheme.as_ref());
        assert_eq!("httpbin.org", client.host);
        assert_eq!("GET", request.method.as_ref());
        assert_eq!("/get", request.path);
        assert_eq!(vec![("a".to_string(), "b".to_string())], request.headers);
        assert_eq!(
            vec![
                ("key".to_string(), "value".to_string()),
                ("variable".to_string(), r#"hello \(variable)"#.to_string()),
            ],
            request.params
        );
        assert_eq!(
            vec!["status == 200".to_string()],
            request
                .asserts
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn parse_multiple_documents() {
        let clients = Clients::from_str(
            r#"
name: user
scheme: https
host: httpbin.org
requests:
  - get:
      path: /get
      method: GET
---
name: test
scheme: https
host: httpbin.org
requests:
  - getIp:
      path: /ip
      method: GET
"#,
        )
        .unwrap();

        assert!(clients.get("user").is_some());
        assert!(clients.get("test").is_some());
    }

    #[test]
    fn parse_ignores_standalone_comments() {
        let clients = Clients::from_str(
            r#"
# primary API client
name: user
scheme: https
host: httpbin.org
requests:
  # read request
  - get:
      path: /get
      method: GET
      # status contract
      asserts:
        - status == 200
"#,
        )
        .unwrap();

        let client = clients.get("user").unwrap();
        assert!(client.request("get").is_some());
    }

    #[test]
    fn parse_form_body() {
        let clients = Clients::from_str(
            r#"
name: user
scheme: https
host: httpbin.org
requests:
  - postForm:
      path: /post
      method: POST
      headers:
        - Content-Type: application/x-www-form-urlencoded
      body:
        - c: d
"#,
        )
        .unwrap();

        let request = clients.get("user").unwrap().request("postForm").unwrap();
        assert_eq!(
            Body::Form(vec![("c".to_string(), "d".to_string())]),
            request.body
        );
    }

    #[test]
    fn parse_multipart_body() {
        let clients = Clients::from_str(
            r#"
name: user
scheme: https
host: httpbin.org
requests:
  - postMultipart:
      path: /post
      method: POST
      headers:
        - Content-Type: multipart/form-data
      body:
        - c: d
        - f: lib.rs
"#,
        )
        .unwrap();

        let request = clients
            .get("user")
            .unwrap()
            .request("postMultipart")
            .unwrap();
        assert_eq!(
            Body::Form(vec![
                ("c".to_string(), "d".to_string()),
                ("f".to_string(), "lib.rs".to_string()),
            ]),
            request.body
        );
    }

    #[test]
    fn parse_inline_json_body_as_text() {
        let clients = Clients::from_str(
            r#"
name: user
scheme: https
host: httpbin.org
requests:
  - postJson:
      path: /post
      method: POST
      body:
        {
          "name": "Gauss",
          "phones": ["+86 13098767890", "+86 15876567890"],
        }
"#,
        )
        .unwrap();

        let request = clients.get("user").unwrap().request("postJson").unwrap();
        assert_eq!(
            Body::Text(
                r#"{
  "name": "Gauss",
  "phones": ["+86 13098767890", "+86 15876567890"],
}"#
                .to_string()
            ),
            request.body
        );
    }

    #[test]
    fn parse_literal_json_body_as_text() {
        let clients = Clients::from_str(
            r#"
name: test
scheme: https
host: httpbin.org
requests:
  - postJsonUseLiteralStyle:
      path: /post
      method: POST
      body: |
        {
          "name": "Gauss",
          "phones": ["+86 13098767890", "+86 15876567890"],
        }
"#,
        )
        .unwrap();

        let request = clients
            .get("test")
            .unwrap()
            .request("postJsonUseLiteralStyle")
            .unwrap();
        assert_eq!(
            Body::Text(
                r#"{
  "name": "Gauss",
  "phones": ["+86 13098767890", "+86 15876567890"],
}"#
                .to_string()
            ),
            request.body
        );
    }

    #[test]
    fn unknown_method_reports_line() {
        let error = Clients::from_str(
            r#"
name: user
scheme: https
host: httpbin.org
requests:
  - get:
      path: /get
      method: FETCH
"#,
        )
        .unwrap_err();

        assert_eq!(8, error.line);
        assert!(error.reason.contains("unknown method"));
    }

    #[test]
    fn missing_host_reports_structure_error() {
        let error = Clients::from_str(
            r#"
name: user
scheme: https
requests:
  - get:
      path: /get
      method: GET
"#,
        )
        .unwrap_err();

        assert!(error.reason.contains("missing required field 'host'"));
    }

    #[test]
    fn headers_reject_non_map_entries() {
        let error = Clients::from_str(
            r#"
name: user
scheme: https
host: httpbin.org
requests:
  - get:
      path: /get
      method: GET
      headers:
        - not-a-map
"#,
        )
        .unwrap_err();

        assert_eq!(10, error.line);
        assert!(
            error
                .reason
                .contains("headers entry must be a single-key map")
        );
    }

    #[test]
    fn invalid_indentation_reports_line() {
        let error = Clients::from_str(
            r#"
name: user
scheme: https
host: httpbin.org
requests:
   - get:
      path: /get
      method: GET
"#,
        )
        .unwrap_err();

        assert_eq!(6, error.line);
        assert!(error.reason.contains("invalid indentation"));
    }

    #[test]
    fn duplicate_request_reports_line() {
        let error = Clients::from_str(
            r#"
name: user
scheme: https
host: httpbin.org
requests:
  - get:
      path: /get
      method: GET
  - get:
      path: /ip
      method: GET
"#,
        )
        .unwrap_err();

        assert_eq!(9, error.line);
        assert!(error.reason.contains("duplicate request 'get'"));
    }
}
