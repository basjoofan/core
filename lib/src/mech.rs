use crate::api::{self, Api, Body};
use crate::native::file_path;
use crate::trans;
use crate::{Expr, Registry, Source, Value};
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;

pub trait Trans {
    fn send<'a>(
        &'a mut self,
        request: trans::Request,
    ) -> Pin<Box<dyn Future<Output = trans::Result> + Send + 'a>>;
}

#[derive(Debug, Clone)]
pub struct Report {
    pub name: String,
    pub excepts: usize,
}

pub struct Mech<'a> {
    source: &'a Source,
    env: HashMap<String, Value>,
    values: HashMap<String, Value>,
    excepts: usize,
    natives: Registry,
    secrets: Vec<String>,
}

impl<'a> Mech<'a> {
    pub fn new(source: &'a Source, environment: Option<&str>) -> Result<Self, String> {
        let env = match (source.environments.is_empty(), environment) {
            (true, None) => HashMap::new(),
            (true, Some(name)) => return Err(format!("environment '{name}' does not exist")),
            (false, None) => {
                return Err("--env is required when source defines environments".into());
            }
            (false, Some(name)) => source
                .environments
                .get(name)
                .ok_or_else(|| format!("environment '{name}' does not exist"))?
                .fields
                .iter()
                .map(|(k, v)| Ok((k.clone(), literal(v)?)))
                .collect::<Result<_, String>>()?,
        };
        Ok(Self {
            source,
            env,
            values: HashMap::new(),
            excepts: 0,
            natives: Registry::default(),
            secrets: Vec::new(),
        })
    }

    pub fn with_natives(mut self, natives: Registry) -> Self {
        self.natives = natives;
        self
    }

    pub fn interactive(source: &'a Source) -> Self {
        Self {
            source,
            env: HashMap::new(),
            values: HashMap::new(),
            excepts: 0,
            natives: Registry::default(),
            secrets: Vec::new(),
        }
    }

    pub fn with_values(mut self, values: HashMap<String, Value>) -> Self {
        self.values = values;
        self
    }

    pub fn into_values(self) -> HashMap<String, Value> {
        self.values
    }

    pub async fn run<T: Trans>(
        &mut self,
        expressions: &[Expr],
        trans: &mut T,
    ) -> Result<Value, String> {
        let result = async {
            let mut result = Value::Null;
            for expression in expressions {
                result = match expression {
                    Expr::Let(name, value) => {
                        let value = self.eval(value, trans).await?;
                        self.values.insert(name.clone(), value.clone());
                        value
                    }
                    Expr::Expect(value) => {
                        let value = self.eval(value, trans).await?;
                        if !expect_boolean(&value)? {
                            return Err(format!("expectation failed: {value}"));
                        }
                        value
                    }
                    value => self.eval(value, trans).await?,
                };
            }
            Ok(result)
        }
        .await;
        result.map_err(|error| self.redact(&error))
    }

    pub async fn run_test<T: Trans>(
        &mut self,
        name: &str,
        transport: &mut T,
    ) -> Result<Report, String> {
        let result = async {
            let test = self
                .source
                .test(name)
                .ok_or_else(|| format!("test '{name}' does not exist"))?
                .clone();
            self.values.clear();
            self.excepts = 0;
            for statement in &test.body {
                match statement {
                    Expr::Let(name, value) => {
                        let value = self.eval(value, transport).await?;
                        self.values.insert(name.clone(), value);
                    }
                    Expr::Expect(value) => {
                        let value = self.eval(value, transport).await?;
                        if !expect_boolean(&value)? {
                            return Err(format!("expectation failed: {value}"));
                        }
                        self.excepts += 1;
                    }
                    value => {
                        self.eval(value, transport).await?;
                    }
                }
            }
            Ok(Report {
                name: test.name,
                excepts: self.excepts,
            })
        }
        .await;
        result.map_err(|error| self.redact(&error))
    }

    fn eval<'b, T: Trans + 'b>(
        &'b mut self,
        expr: &'b Expr,
        transport: &'b mut T,
    ) -> Pin<Box<dyn Future<Output = Result<Value, String>> + 'b>> {
        Box::pin(async move {
            match expr {
                Expr::Null => Ok(Value::Null),
                Expr::Integer(v) => Ok(Value::Integer(*v)),
                Expr::Float(v) => Ok(Value::Float(*v)),
                Expr::Boolean(v) => Ok(Value::Boolean(*v)),
                Expr::String(v) | Expr::Raw(v) => self.template(v, transport).await,
                Expr::Ident(name) => self
                    .values
                    .get(name)
                    .cloned()
                    .or_else(|| match name.as_str() {
                        "env" => Some(Value::Map(self.env.clone())),
                        _ => None,
                    })
                    .ok_or_else(|| format!("unknown identifier '{name}'")),
                Expr::Paren(value) => self.eval(value, transport).await,
                Expr::Array(values) => {
                    let mut out = Vec::new();
                    for v in values {
                        out.push(self.eval(v, transport).await?);
                    }
                    Ok(Value::Array(out))
                }
                Expr::Map(values) => {
                    let mut out = HashMap::new();
                    for (k, v) in values {
                        out.insert(k.clone(), self.eval(v, transport).await?);
                    }
                    Ok(Value::Map(out))
                }
                Expr::Field(left, field) => match self.eval(left, transport).await? {
                    Value::Map(values) => Ok(values.get(field).cloned().unwrap_or(Value::Null)),
                    other => Err(format!("cannot access field '{field}' on {other:?}")),
                },
                Expr::Index(left, index) => {
                    let v = self.eval(left, transport).await?;
                    let i = self.eval(index, transport).await?;
                    match (v, i) {
                        (Value::Array(a), Value::Integer(i)) if i >= 0 => {
                            Ok(a.get(i as usize).cloned().unwrap_or(Value::Null))
                        }
                        (Value::Map(m), i) => {
                            Ok(m.get(&i.to_string()).cloned().unwrap_or(Value::Null))
                        }
                        _ => Err("invalid index operation".into()),
                    }
                }
                Expr::Unary(token, value) => {
                    let v = self.eval(value, transport).await?;
                    match token.kind {
                        crate::token::Kind::Not => Ok(Value::Boolean(!expect_boolean(&v)?)),
                        crate::token::Kind::Sub => match v {
                            Value::Integer(n) => Ok(Value::Integer(-n)),
                            Value::Float(n) => Ok(Value::Float(-n)),
                            _ => Err("unary '-' requires number".into()),
                        },
                        _ => Err("unsupported unary operator".into()),
                    }
                }
                Expr::Binary(token, l, r) => {
                    let a = self.eval(l, transport).await?;
                    if token.kind == crate::token::Kind::La && !expect_boolean(&a)? {
                        return Ok(Value::Boolean(false));
                    }
                    if token.kind == crate::token::Kind::Lo && expect_boolean(&a)? {
                        return Ok(Value::Boolean(true));
                    }
                    let b = self.eval(r, transport).await?;
                    binary(&token.kind, a, b)
                }
                Expr::Call(target, args) => self.call(target, args, transport).await,
                Expr::Let(_, _) | Expr::Expect(_) => Err("statement is not an expression".into()),
            }
        })
    }

    fn call<'b, T: Trans + 'b>(
        &'b mut self,
        target: &'b Expr,
        args: &'b [Expr],
        transport: &'b mut T,
    ) -> Pin<Box<dyn Future<Output = Result<Value, String>> + 'b>> {
        Box::pin(async move {
            if let Expr::Ident(name) = target {
                let mut arguments = Vec::new();
                for argument in args {
                    arguments.push(self.eval(argument, transport).await?);
                }
                if let Some(output) = self.natives.call(name, arguments) {
                    let output = output?;
                    if output.sensitive {
                        self.secrets.push(output.value.to_string());
                    }
                    return Ok(output.value);
                }
            }
            if let Expr::Field(api_expr, request_name) = target
                && let Expr::Ident(api_name) = api_expr.as_ref()
                && let Some(api) = self.source.apis.get(api_name).cloned()
            {
                let request = api
                    .request(request_name)
                    .cloned()
                    .ok_or_else(|| format!("unknown request '{api_name}.{request_name}'"))?;
                return self.request(&api, &request, args, transport).await;
            }
            if let Expr::Field(inner, method) = target
                && method == "contains"
                && let Expr::Call(header, args1) = inner.as_ref()
                && let Expr::Field(response, name) = header.as_ref()
                && name == "header"
            {
                let response = self.eval(response, transport).await?;
                let header_name = self.single_string(args1, transport).await?;
                let sought = self.single_string(args, transport).await?;
                let values = match response {
                    Value::Map(m) => m.get("headers").cloned(),
                    _ => None,
                }
                .ok_or("header is not available")?;
                let found = match values {
                    Value::Map(m) => m.get(&header_name.to_ascii_lowercase()).cloned(),
                    _ => None,
                };
                return Ok(Value::Boolean(match found {
                    Some(Value::Array(v)) => v.iter().any(|v| v.to_string().contains(&sought)),
                    Some(v) => v.to_string().contains(&sought),
                    None => false,
                }));
            }
            if let Expr::Field(response, method) = target
                && method == "header"
            {
                let response = self.eval(response, transport).await?;
                let header_name = self.single_string(args, transport).await?;
                let headers = match response {
                    Value::Map(values) => values.get("headers").cloned(),
                    _ => None,
                }
                .ok_or("header is not available")?;
                return match headers {
                    Value::Map(values) => Ok(values
                        .get(&header_name.to_ascii_lowercase())
                        .cloned()
                        .unwrap_or(Value::Null)),
                    _ => Err("headers are invalid".into()),
                };
            }
            Err(format!("unknown call '{target}'"))
        })
    }

    async fn single_string<T: Trans>(
        &mut self,
        args: &[Expr],
        t: &mut T,
    ) -> Result<String, String> {
        if args.len() != 1 {
            return Err("function requires one argument".into());
        };
        Ok(self.eval(&args[0], t).await?.to_string())
    }
    async fn request<T: Trans>(
        &mut self,
        api: &Api,
        request: &api::Request,
        args: &[Expr],
        t: &mut T,
    ) -> Result<Value, String> {
        if args.len() != request.params_def.len() {
            return Err(format!(
                "{} expects {} arguments",
                request.name,
                request.params_def.len()
            ));
        }
        let mut saved = HashMap::new();
        for (arg, param) in args.iter().zip(&request.params_def) {
            let v = self.eval(arg, t).await?;
            type_check(&v, &param.kind)?;
            saved.insert(param.name.clone(), v);
        }
        let previous = std::mem::replace(&mut self.values, saved);
        let result = async {
            let scheme = match &api.scheme {
                Expr::Ident(value) => value.clone(),
                value => self.eval(value, t).await?.to_string(),
            };
            if !matches!(scheme.as_str(), "http" | "https") {
                return Err(format!("unsupported scheme '{scheme}'"));
            }
            let host = self.eval(&api.host, t).await?.to_string();
            let port = match &api.port {
                Some(Expr::Field(base, field))
                    if matches!(base.as_ref(), Expr::Ident(name) if name == "env")
                        && !self.env.contains_key(field) =>
                {
                    None
                }
                Some(v) => {
                    let port = self.eval(v, t).await?.to_string();
                    port.parse::<u16>()
                        .map_err(|_| format!("invalid port '{port}'"))?;
                    Some(port)
                }
                None => None,
            };
            let path = match &request.path {
                Expr::String(value) | Expr::Raw(value) => self.path_template(value, t).await?,
                value => self.eval(value, t).await?.to_string(),
            };
            let mut url = format!("{scheme}://{host}");
            if let Some(port) = port {
                url.push(':');
                url.push_str(&port)
            };
            url.push_str(&path);
            let query = self.pairs(&request.params, t).await?;
            if !query.is_empty() {
                url.push(if url.contains('?') { '&' } else { '?' });
                url.push_str(&encode_pairs(&query));
            }
            let mut headers = self.pairs(&api.headers, t).await?;
            merge_headers(&mut headers, self.pairs(&request.headers, t).await?);
            let (body, implicit) = self.body(&request.body, t).await?;
            if let Some((name, value)) = implicit {
                if !headers
                    .iter()
                    .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
                {
                    headers.push((name, value));
                }
            }
            if matches!(request.body, Body::Text(_) | Body::File(_))
                && !headers
                    .iter()
                    .any(|(key, _)| key.eq_ignore_ascii_case("content-type"))
            {
                return Err("text and file requests require Content-Type".into());
            }
            if request.method.as_ref() == "QUERY"
                && !headers
                    .iter()
                    .any(|(k, _)| k.eq_ignore_ascii_case("content-type"))
            {
                return Err("QUERY requests require Content-Type".into());
            }
            let result = t
                .send(trans::Request {
                    method: request.method.as_ref().to_owned(),
                    url,
                    headers: headers
                        .into_iter()
                        .map(|(name, value)| trans::Header { name, value })
                        .collect(),
                    body,
                })
                .await;
            if !result.error.is_empty() {
                return Err(self.redact(&result.error));
            };
            response_value(result)
        }
        .await;
        self.values = previous;
        result
    }
    async fn pairs<T: Trans>(
        &mut self,
        pairs: &[(Expr, Expr)],
        t: &mut T,
    ) -> Result<Vec<(String, String)>, String> {
        let mut out = Vec::new();
        for (k, v) in pairs {
            let key = self.eval(k, t).await?.to_string();
            match self.eval(v, t).await? {
                Value::Array(a) => {
                    for x in a {
                        out.push((key.clone(), scalar_text(x)?))
                    }
                }
                x => out.push((key, scalar_text(x)?)),
            }
        }
        Ok(out)
    }
    async fn body<T: Trans>(
        &mut self,
        body: &Body,
        t: &mut T,
    ) -> Result<(Option<trans::Content>, Option<(String, String)>), String> {
        match body {
            Body::None => Ok((None, None)),
            Body::Json(v) => Ok((
                Some(trans::Content::Text(self.eval(v, t).await?.to_json())),
                Some(("Content-Type".into(), "application/json".into())),
            )),
            Body::Form(p) => Ok((
                Some(trans::Content::Text(encode_pairs(&self.pairs(p, t).await?))),
                Some((
                    "Content-Type".into(),
                    "application/x-www-form-urlencoded".into(),
                )),
            )),
            Body::Part(p) => {
                let mut parts = Vec::new();
                for (key, value) in p {
                    let key = self.eval(key, t).await?.to_string();
                    match self.eval(value, t).await? {
                        value if file_path(&value).is_some() => {
                            parts.push(format!("{key}: @{}", file_path(&value).unwrap()))
                        }
                        Value::Array(values) => {
                            for value in values {
                                if let Some(path) = file_path(&value) {
                                    parts.push(format!("{key}: @{path}"));
                                } else {
                                    parts.push(format!("{key}: {}", scalar_text(value)?));
                                }
                            }
                        }
                        value => parts.push(format!("{key}: {}", scalar_text(value)?)),
                    }
                }
                Ok((
                    Some(trans::Content::Text(parts.join("\n"))),
                    Some(("Content-Type".into(), "multipart/form-data".into())),
                ))
            }
            Body::Text(v) => match self.eval(v, t).await? {
                Value::String(value) if !value.starts_with("\0file:") => {
                    Ok((Some(trans::Content::Text(value)), None))
                }
                value => Err(format!("text request body requires string, got {value:?}")),
            },
            Body::File(v) => match self.eval(v, t).await? {
                value if file_path(&value).is_some() => {
                    let path = file_path(&value).unwrap();
                    Ok((Some(trans::Content::File(path.to_owned())), None))
                }
                value => Err(format!(
                    "file request body requires file(...), got {value:?}"
                )),
            },
        }
    }
    async fn template<T: Trans>(&mut self, text: &str, t: &mut T) -> Result<Value, String> {
        let mut out = String::new();
        let mut rest = text;
        while let Some(i) = rest.find("\\(") {
            out.push_str(&rest[..i]);
            let after = &rest[i + 2..];
            let end = interpolation_end(after).ok_or("unterminated interpolation")?;
            let expr = crate::Parser::new(&after[..end])
                .parse()?
                .exprs
                .into_iter()
                .next()
                .ok_or("empty interpolation")?;
            let value = self.eval(&expr, t).await?;
            match &value {
                Value::String(value) => {
                    out.push_str(value.strip_prefix("\0file:").unwrap_or(value))
                }
                value => out.push_str(&value.to_string()),
            }
            rest = &after[end + 1..];
        }
        out.push_str(rest);
        Ok(Value::String(out))
    }

    async fn path_template<T: Trans>(&mut self, text: &str, t: &mut T) -> Result<String, String> {
        let mut output = String::new();
        let mut rest = text;
        while let Some(index) = rest.find("\\(") {
            output.push_str(&rest[..index]);
            let after = &rest[index + 2..];
            let end = interpolation_end(after).ok_or("unterminated interpolation")?;
            let expression = crate::Parser::new(&after[..end])
                .parse()?
                .exprs
                .into_iter()
                .next()
                .ok_or("empty interpolation")?;
            output.push_str(&encode_path_segment(
                &self.eval(&expression, t).await?.to_string(),
            ));
            rest = &after[end + 1..];
        }
        output.push_str(rest);
        Ok(output)
    }

    fn redact(&self, text: &str) -> String {
        self.secrets
            .iter()
            .filter(|secret| !secret.is_empty())
            .fold(text.to_owned(), |text, secret| {
                text.replace(secret, "[REDACTED]")
            })
    }
}

fn literal(expr: &Expr) -> Result<Value, String> {
    match expr {
        Expr::String(v) | Expr::Raw(v) => Ok(Value::String(v.clone())),
        Expr::Integer(v) => Ok(Value::Integer(*v)),
        Expr::Float(v) => Ok(Value::Float(*v)),
        Expr::Boolean(v) => Ok(Value::Boolean(*v)),
        Expr::Ident(v) => Ok(Value::String(v.clone())),
        _ => Err("environment values must be literals".into()),
    }
}

fn interpolation_end(text: &str) -> Option<usize> {
    let mut depth = 1;
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in text.char_indices() {
        if let Some(active) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == active {
                quote = None;
            }
            continue;
        }
        match character {
            '"' | '`' => quote = Some(character),
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}
fn expect_boolean(value: &Value) -> Result<bool, String> {
    match value {
        Value::Boolean(value) => Ok(*value),
        value => Err(format!("expected boolean, got {value:?}")),
    }
}
fn type_check(v: &Value, k: &str) -> Result<(), String> {
    let ok = match (k, v) {
        ("int" | "integer", Value::Integer(_))
        | ("float", Value::Float(_))
        | ("string", Value::String(_))
        | ("bool" | "boolean", Value::Boolean(_))
        | ("array", Value::Array(_))
        | ("map", Value::Map(_)) => true,
        ("file", value) => file_path(value).is_some(),
        _ => false,
    };
    ok.then_some(())
        .ok_or_else(|| format!("expected {k}, got {v:?}"))
}
fn binary(k: &crate::token::Kind, a: Value, b: Value) -> Result<Value, String> {
    use crate::token::Kind::*;
    match k {
        Eq => Ok(Value::Boolean(a == b)),
        Ne => Ok(Value::Boolean(a != b)),
        Add => a + b,
        Sub => a - b,
        Mul => a * b,
        Div => a / b,
        Rem => a % b,
        Bx => a ^ b,
        Bo => a | b,
        Ba => a & b,
        Sl => a << b,
        Sr => a >> b,
        Lt => Ok(Value::Boolean(a < b)),
        Gt => Ok(Value::Boolean(a > b)),
        Le => Ok(Value::Boolean(a <= b)),
        Ge => Ok(Value::Boolean(a >= b)),
        La => Ok(Value::Boolean(expect_boolean(&a)? && expect_boolean(&b)?)),
        Lo => Ok(Value::Boolean(expect_boolean(&a)? || expect_boolean(&b)?)),
        _ => Err("unsupported binary operator".into()),
    }
}
fn encode(s: &str) -> String {
    s.bytes()
        .map(|b| match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (b as char).to_string()
            }
            b' ' => "+".into(),
            _ => format!("%{b:02X}"),
        })
        .collect()
}
fn encode_path_segment(value: &str) -> String {
    value
        .bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}
fn encode_pairs(p: &[(String, String)]) -> String {
    p.iter()
        .map(|(k, v)| format!("{}={}", encode(k), encode(v)))
        .collect::<Vec<_>>()
        .join("&")
}
fn scalar_text(value: Value) -> Result<String, String> {
    match value {
        Value::Array(_) | Value::Map(_) => Err("field values must be scalar".into()),
        value if file_path(&value).is_some() => Err("file value is not valid in this field".into()),
        value => Ok(value.to_string()),
    }
}
fn merge_headers(base: &mut Vec<(String, String)>, extra: Vec<(String, String)>) {
    base.retain(|(name, _)| {
        !extra
            .iter()
            .any(|(extra_name, _)| name.eq_ignore_ascii_case(extra_name))
    });
    base.extend(extra);
}
fn response_value(r: trans::Result) -> Result<Value, String> {
    let is_json = r.response.headers.iter().any(|h| {
        h.name.eq_ignore_ascii_case("content-type") && h.value.to_ascii_lowercase().contains("json")
    });
    let mut headers: HashMap<String, Value> = HashMap::new();
    for h in r.response.headers {
        headers
            .entry(h.name.to_ascii_lowercase())
            .or_insert_with(|| Value::Array(Vec::new()));
        if let Some(Value::Array(v)) = headers.get_mut(&h.name.to_ascii_lowercase()) {
            v.push(Value::String(h.value));
        }
    }
    let request_headers = r
        .request
        .headers
        .iter()
        .fold(HashMap::new(), |mut values, header| {
            values.insert(
                header.name.to_ascii_lowercase(),
                Value::String(header.value.clone()),
            );
            values
        });
    let request = Value::Map(HashMap::from([
        ("method".into(), Value::String(r.request.method.clone())),
        ("url".into(), Value::String(r.request.url.clone())),
        ("headers".into(), Value::Map(request_headers)),
    ]));
    let mut m = HashMap::from([
        ("status".into(), Value::Integer(r.response.status as i64)),
        ("body".into(), Value::String(r.response.body.clone())),
        ("headers".into(), Value::Map(headers)),
        (
            "duration".into(),
            Value::Integer(r.timing.total.as_millis() as i64),
        ),
        ("request".into(), request),
    ]);
    if is_json {
        let value = Value::from_json(&r.response.body)
            .map_err(|error| format!("invalid JSON response: {error}"))?;
        m.insert("json".into(), value);
    }
    Ok(Value::Map(m))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Mock {
        requests: Vec<trans::Request>,
    }
    impl Trans for Mock {
        fn send<'a>(
            &'a mut self,
            request: trans::Request,
        ) -> Pin<Box<dyn Future<Output = trans::Result> + Send + 'a>> {
            Box::pin(async move {
                self.requests.push(request.clone());
                trans::Result {
                    request,
                    response: crate::Response {
                        status: 201,
                        headers: vec![
                            trans::Header {
                                name: "content-type".into(),
                                value: "application/json".into(),
                            },
                            trans::Header {
                                name: "x-repeat".into(),
                                value: "one".into(),
                            },
                            trans::Header {
                                name: "X-Repeat".into(),
                                value: "two".into(),
                            },
                        ],
                        body: r#"{"id":7,"name":"Gauss"}"#.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }
            })
        }
    }

    #[tokio::test]
    async fn executes_api_call_template_and_json_response() {
        let source = crate::Parser::new(r#"env local { scheme: http, host: "example.test" }
api user { scheme: env.scheme, host: env.host, create(name: string, age: int) { method: POST, path: "/users/\(name)/\(age)", json: { name: name, age: age } } }
test flow { let created = user.create("Gauss", 6); expect created.status == 201; expect created.json.name == "Gauss"; }"#).parse().unwrap();
        let mut mech = Mech::new(&source, Some("local")).unwrap();
        let mut trans = Mock::default();
        let result = mech.run_test("flow", &mut trans).await.unwrap();
        assert_eq!(result.excepts, 2);
        assert_eq!(trans.requests[0].url, "http://example.test/users/Gauss/6");
    }

    #[tokio::test]
    async fn evaluates_or_and_unary_operators() {
        let source = crate::Parser::new(
            "test logic { expect false || true; expect !false; expect -1 < 0; }",
        )
        .parse()
        .unwrap();
        let mut mech = Mech::new(&source, None).unwrap();
        let report = mech.run_test("logic", &mut Mock::default()).await.unwrap();
        assert_eq!(report.excepts, 3);
    }

    #[tokio::test]
    async fn defaults_missing_environment_port_and_encodes_path_segments() {
        let source = crate::Parser::new(
            r#"
            env staging { scheme: https, host: "example.test" }
            api user { scheme: env.scheme, host: env.host, port: env.port,
                get(id: string) { method: GET, path: "/users/\(id)" }
            }
            test flow { let response = user.get("a/b c"); expect response.status == 201; }
        "#,
        )
        .parse()
        .unwrap();
        let mut mech = Mech::new(&source, Some("staging")).unwrap();
        let mut trans = Mock::default();
        mech.run_test("flow", &mut trans).await.unwrap();
        assert_eq!(
            trans.requests[0].url,
            "https://example.test/users/a%2Fb%20c"
        );
    }

    #[tokio::test]
    async fn type_checks_arrays_maps_and_boolean_expectations() {
        let source = crate::Parser::new(
            r#"api typed { scheme: http, host: "example.test",
                send(items: array, object: map) { method: POST, path: "/", json: object }
            }
            test flow { let response = typed.send([1, 2], { value: true }); expect response.status == 201; }"#,
        )
        .parse()
        .unwrap();
        Mech::new(&source, None)
            .unwrap()
            .run_test("flow", &mut Mock::default())
            .await
            .unwrap();

        let source = crate::Parser::new("test flow { expect 1; }")
            .parse()
            .unwrap();
        let error = Mech::new(&source, None)
            .unwrap()
            .run_test("flow", &mut Mock::default())
            .await
            .unwrap_err();
        assert!(error.contains("expected boolean"), "{error}");
    }

    #[tokio::test]
    async fn exposes_repeated_headers_and_request_metadata() {
        let source = crate::Parser::new(
            r#"api user { scheme: http, host: "example.test", get() { method: GET, path: "/" } }
            test flow {
                let response = user.get();
                expect response.header("X-Repeat")[0] == "one";
                expect response.header("x-repeat")[1] == "two";
                expect response.header("X-Repeat").contains("tw");
                expect response.request.method == "GET";
                expect response.missing == null;
                expect response.header("missing") == null;
            }"#,
        )
        .parse()
        .unwrap();
        let report = Mech::new(&source, None)
            .unwrap()
            .run_test("flow", &mut Mock::default())
            .await
            .unwrap();
        assert_eq!(report.excepts, 6);
    }

    #[tokio::test]
    async fn returns_null_for_missing_lookups_and_rejects_invalid_access() {
        let source = crate::Parser::new(
            r#"test flow {
                let value = { items: [1], object: {} };
                expect value.missing == null;
                expect value.items[1] == null;
                expect value.object["missing"] == null;
            }"#,
        )
        .parse()
        .unwrap();
        let report = Mech::new(&source, None)
            .unwrap()
            .run_test("flow", &mut Mock::default())
            .await
            .unwrap();
        assert_eq!(report.excepts, 3);

        let source = crate::Parser::new("test flow { expect 1.missing == null; }")
            .parse()
            .unwrap();
        let error = Mech::new(&source, None)
            .unwrap()
            .run_test("flow", &mut Mock::default())
            .await
            .unwrap_err();
        assert!(error.contains("cannot access field 'missing'"), "{error}");
    }

    #[tokio::test]
    async fn rejects_unsupported_scheme_and_invalid_json_response() {
        let source = crate::Parser::new(
            r#"api user { scheme: ftp, host: "example.test", get() { method: GET, path: "/" } }
            test flow { user.get(); }"#,
        )
        .parse()
        .unwrap();
        let error = Mech::new(&source, None)
            .unwrap()
            .run_test("flow", &mut Mock::default())
            .await
            .unwrap_err();
        assert!(error.contains("unsupported scheme 'ftp'"), "{error}");

        struct InvalidJson;
        impl Trans for InvalidJson {
            fn send<'a>(
                &'a mut self,
                request: trans::Request,
            ) -> Pin<Box<dyn Future<Output = trans::Result> + Send + 'a>> {
                Box::pin(async move {
                    trans::Result {
                        request,
                        response: crate::Response {
                            status: 200,
                            headers: vec![trans::Header {
                                name: "content-type".into(),
                                value: "application/json".into(),
                            }],
                            body: "{".into(),
                            ..Default::default()
                        },
                        ..Default::default()
                    }
                })
            }
        }
        let source = crate::Parser::new(
            r#"api user { scheme: http, host: "example.test", get() { method: GET, path: "/" } }
            test flow { user.get(); }"#,
        )
        .parse()
        .unwrap();
        let error = Mech::new(&source, None)
            .unwrap()
            .run_test("flow", &mut InvalidJson)
            .await
            .unwrap_err();
        assert!(error.contains("invalid JSON response"), "{error}");
    }

    #[tokio::test]
    async fn redacts_sensitive_native_values_from_transport_errors() {
        fn sensitive(_: Vec<Value>) -> Result<crate::Output, String> {
            Ok(crate::Output {
                value: Value::String("super-secret-value".into()),
                sensitive: true,
            })
        }
        struct Failure;
        impl Trans for Failure {
            fn send<'a>(
                &'a mut self,
                request: trans::Request,
            ) -> Pin<Box<dyn Future<Output = trans::Result> + Send + 'a>> {
                Box::pin(async move {
                    trans::Result {
                        request,
                        error: "transport rejected super-secret-value".into(),
                        ..Default::default()
                    }
                })
            }
        }
        let source = crate::Parser::new(
            r#"api user { scheme: http, host: "example.test", headers: { Authorization: sensitive() },
                get() { method: GET, path: "/" }
            }
            test flow { user.get(); }"#,
        )
        .parse()
        .unwrap();
        let mut natives = crate::Registry::default();
        natives.register("sensitive", sensitive);
        let error = Mech::new(&source, None)
            .unwrap()
            .with_natives(natives)
            .run_test("flow", &mut Failure)
            .await
            .unwrap_err();
        assert_eq!(error, "transport rejected [REDACTED]");

        let source = crate::Parser::new("test flow { sensitive() + 1; }")
            .parse()
            .unwrap();
        let mut natives = crate::Registry::default();
        natives.register("sensitive", sensitive);
        let error = Mech::new(&source, None)
            .unwrap()
            .with_natives(natives)
            .run_test("flow", &mut Mock::default())
            .await
            .unwrap_err();
        assert!(!error.contains("super-secret-value"), "{error}");
        assert!(error.contains("[REDACTED]"), "{error}");
    }

    #[tokio::test]
    async fn interpolates_escaped_and_indented_strings() {
        let source = crate::Parser::new(
            "test strings {\n\
                expect \"line\\n\\(1 + 1)\" == \"line\\n2\";\n\
                expect `\n                    first\n                      second\n                ` == \"first\\n  second\";\n\
                expect `a\\`b` == \"a`b\";\n\
            }",
        )
        .parse()
        .unwrap();
        let report = Mech::new(&source, None)
            .unwrap()
            .run_test("strings", &mut Mock::default())
            .await
            .unwrap();
        assert_eq!(report.excepts, 3);
    }

    #[tokio::test]
    async fn prepares_all_body_kinds_and_merges_headers() {
        let source = crate::Parser::new(
            r#"api bodies {
                scheme: http,
                host: "example.test",
                headers: { "X-Base": "base", "X-Override": "api", "X-Multi": ["a", "b"] },
                jsonBody() { method: POST, path: "/json", headers: { "X-Multi": ["request-a", "request-b"] }, json: { value: 1, items: [1, 2] } },
                formBody() { method: POST, path: "/form", form: { tag: ["a", "b"] } },
                partBody() { method: POST, path: "/part", multipart: { note: ["hello", "world"], upload: [file("/tmp/a.txt"), file("/tmp/b.txt")] } },
                textBody() { method: POST, path: "/text", headers: { "Content-Type": "text/plain", "x-override": "request" }, text: "hello" },
                fileBody() { method: POST, path: "/file", headers: { "Content-Type": "application/octet-stream" }, file: file("/tmp/a.bin") }
            }
            test flow {
                bodies.jsonBody();
                bodies.formBody();
                bodies.partBody();
                bodies.textBody();
                bodies.fileBody();
            }"#,
        )
        .parse()
        .unwrap();
        let mut transport = Mock::default();
        Mech::new(&source, None)
            .unwrap()
            .run_test("flow", &mut transport)
            .await
            .unwrap();
        assert_eq!(transport.requests.len(), 5);
        assert!(
            matches!(&transport.requests[0].body, Some(trans::Content::Text(value)) if value == "{\"items\":[1,2],\"value\":1}")
        );
        assert!(
            transport.requests[0]
                .headers
                .iter()
                .any(|header| header.name.eq_ignore_ascii_case("content-type")
                    && header.value == "application/json")
        );
        assert!(
            matches!(&transport.requests[1].body, Some(trans::Content::Text(value)) if value == "tag=a&tag=b")
        );
        assert!(
            matches!(&transport.requests[2].body, Some(trans::Content::Text(value)) if value.contains("upload: @/tmp/a.txt") && value.contains("upload: @/tmp/b.txt"))
        );
        assert!(
            matches!(&transport.requests[2].body, Some(trans::Content::Text(value)) if value.matches("note:").count() == 2)
        );
        assert!(
            matches!(&transport.requests[3].body, Some(trans::Content::Text(value)) if value == "hello")
        );
        assert!(
            matches!(&transport.requests[4].body, Some(trans::Content::File(value)) if value == "/tmp/a.bin")
        );
        let override_headers: Vec<_> = transport.requests[3]
            .headers
            .iter()
            .filter(|header| header.name.eq_ignore_ascii_case("x-override"))
            .collect();
        assert_eq!(override_headers.len(), 1);
        assert_eq!(override_headers[0].value, "request");
        let repeated: Vec<_> = transport.requests[0]
            .headers
            .iter()
            .filter(|header| header.name.eq_ignore_ascii_case("x-multi"))
            .map(|header| header.value.as_str())
            .collect();
        assert_eq!(repeated, ["request-a", "request-b"]);
    }

    #[tokio::test]
    async fn encodes_query_pairs_and_requires_content_type_for_raw_bodies() {
        let source = crate::Parser::new(
            r#"api user { scheme: https, host: "example.test",
                search() { method: GET, path: "/search", params: { "q key": "a/b c", tag: ["a", "b"] } }
            }
            test flow { user.search(); }"#,
        )
        .parse()
        .unwrap();
        let mut transport = Mock::default();
        Mech::new(&source, None)
            .unwrap()
            .run_test("flow", &mut transport)
            .await
            .unwrap();
        assert_eq!(
            transport.requests[0].url,
            "https://example.test/search?q+key=a%2Fb+c&tag=a&tag=b"
        );

        for body in ["text: \"hello\"", "file: file(\"/tmp/a\")"] {
            let source = crate::Parser::new(&format!(
                "api user {{ scheme: http, host: \"example.test\", send() {{ method: POST, path: \"/\", {body} }} }} test flow {{ user.send(); }}"
            ))
            .parse()
            .unwrap();
            let error = Mech::new(&source, None)
                .unwrap()
                .run_test("flow", &mut Mock::default())
                .await
                .unwrap_err();
            assert!(error.contains("require Content-Type"), "{error}");
        }

        let source = crate::Parser::new(
            r#"api user { scheme: http, host: "example.test",
                send() { method: POST, path: "/", params: { invalid: { nested: true } } }
            }
            test flow { user.send(); }"#,
        )
        .parse()
        .unwrap();
        let error = Mech::new(&source, None)
            .unwrap()
            .run_test("flow", &mut Mock::default())
            .await
            .unwrap_err();
        assert!(error.contains("must be scalar"), "{error}");

        let source = crate::Parser::new(
            r#"api user { scheme: http, host: "example.test",
                send() { method: POST, path: "/", headers: { "Content-Type": "text/plain" }, text: 42 }
            }
            test flow { user.send(); }"#,
        )
        .parse()
        .unwrap();
        let error = Mech::new(&source, None)
            .unwrap()
            .run_test("flow", &mut Mock::default())
            .await
            .unwrap_err();
        assert!(error.contains("requires string"), "{error}");
    }
}
