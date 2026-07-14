use super::lexer::Lexer;
use super::{Expr, Kind, Source, Token};
use crate::api::{Api, Body, Method, Request};
use crate::syntax::{Declaration, Environment, Test};
use std::collections::HashMap;

pub struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    pub fn new(text: &str) -> Self {
        let string_ends = string_ends(text);
        Self {
            tokens: Lexer::new().segment_with_string_ends(text, &string_ends),
            index: 0,
        }
    }

    pub fn parse(&mut self) -> Result<Source, String> {
        let mut source = Source::new();
        while self.peek().is_some() {
            let tags = self.parse_tags()?;
            match self.peek_kind() {
                Some(Kind::Env) => {
                    if !tags.is_empty() {
                        return Err(self.error("annotations only apply to tests"));
                    }
                    let environment = self.parse_environment()?;
                    if source
                        .environments
                        .insert(environment.name.clone(), environment.clone())
                        .is_some()
                    {
                        return Err(self.error("duplicate environment"));
                    }
                    source
                        .declarations
                        .push(Declaration::Environment(environment));
                }
                Some(Kind::Api) => {
                    if !tags.is_empty() {
                        return Err(self.error("annotations only apply to tests"));
                    }
                    let api = self.parse_api()?;
                    source.apis.insert(api.clone())?;
                    source.declarations.push(Declaration::Api(api));
                }
                Some(Kind::Test) => {
                    let test = self.parse_test(tags)?;
                    if source
                        .tests
                        .insert(test.name.clone(), test.clone())
                        .is_some()
                    {
                        return Err(self.error("duplicate test"));
                    }
                    source.declarations.push(Declaration::Test(test));
                }
                Some(_) if tags.is_empty() => source.exprs.push(self.parse_statement()?),
                Some(_) => return Err(self.error("annotation must precede a test")),
                None => break,
            }
            self.consume(Kind::Semi);
        }
        Ok(source)
    }

    fn parse_environment(&mut self) -> Result<Environment, String> {
        self.expect(Kind::Env)?;
        let name = self.ident()?;
        let fields = self.parse_fields()?;
        Ok(Environment { name, fields })
    }

    fn parse_api(&mut self) -> Result<Api, String> {
        self.expect(Kind::Api)?;
        let name = self.ident()?;
        self.expect(Kind::Lb)?;
        let mut scheme = None;
        let mut host = None;
        let mut port = None;
        let mut headers = Vec::new();
        let mut requests = HashMap::new();
        let mut api_fields = std::collections::HashSet::new();

        while !self.consume(Kind::Rb) {
            let field = self.ident()?;
            if self.consume(Kind::Lp) {
                let request = self.parse_request(field.clone())?;
                if requests.insert(field.clone(), request).is_some() {
                    return Err(self.error(format!("duplicate request '{field}'")));
                }
            } else {
                self.expect(Kind::Colon)?;
                if !api_fields.insert(field.clone()) {
                    return Err(self.error(format!("duplicate api field '{field}'")));
                }
                match field.as_str() {
                    "scheme" => scheme = Some(self.parse_expr()?),
                    "host" => host = Some(self.parse_expr()?),
                    "port" => port = Some(self.parse_expr()?),
                    "headers" => headers = self.parse_pairs()?,
                    _ => return Err(self.error(format!("unknown api field '{field}'"))),
                }
            }
            if !self.consume(Kind::Comma) && self.peek_kind() != Some(Kind::Rb) {
                return Err(self.error("expected ',' between api entries"));
            }
        }

        Ok(Api {
            name,
            scheme: scheme.ok_or_else(|| self.error("api requires scheme"))?,
            host: host.ok_or_else(|| self.error("api requires host"))?,
            port,
            headers,
            requests,
        })
    }

    fn parse_request(&mut self, name: String) -> Result<Request, String> {
        let mut params_def = Vec::new();
        while !self.consume(Kind::Rp) {
            let parameter = self.ident()?;
            self.expect(Kind::Colon)?;
            let kind = self.ident()?;
            if !matches!(
                kind.as_str(),
                "int"
                    | "integer"
                    | "float"
                    | "string"
                    | "bool"
                    | "boolean"
                    | "array"
                    | "map"
                    | "file"
            ) {
                return Err(self.error(format!("unknown parameter type '{kind}'")));
            }
            if params_def
                .iter()
                .any(|value: &crate::api::Parameter| value.name == parameter)
            {
                return Err(self.error(format!("duplicate parameter '{parameter}'")));
            }
            params_def.push(crate::api::Parameter {
                name: parameter,
                kind,
            });
            if !self.consume(Kind::Comma) {
                self.expect(Kind::Rp)?;
                break;
            }
        }
        self.expect(Kind::Lb)?;
        let mut method = None;
        let mut path = None;
        let mut params = Vec::new();
        let mut headers = Vec::new();
        let mut body = Body::None;
        let mut request_fields = std::collections::HashSet::new();
        while !self.consume(Kind::Rb) {
            let field = self.ident()?;
            self.expect(Kind::Colon)?;
            if !request_fields.insert(field.clone()) {
                return Err(self.error(format!("duplicate request field '{field}'")));
            }
            if matches!(
                field.as_str(),
                "json" | "form" | "multipart" | "text" | "file"
            ) && !matches!(body, Body::None)
            {
                return Err(self.error(format!("request '{name}' defines multiple body fields")));
            }
            match field.as_str() {
                "method" => method = Some(Method::parse(&self.ident()?)?),
                "path" => path = Some(self.parse_expr()?),
                "params" => params = self.parse_pairs()?,
                "headers" => headers = self.parse_pairs()?,
                "json" => body = Body::Json(self.parse_expr()?),
                "form" => body = Body::Form(self.parse_pairs()?),
                "multipart" => body = Body::Part(self.parse_pairs()?),
                "text" => body = Body::Text(self.parse_expr()?),
                "file" => body = Body::File(self.parse_expr()?),
                _ => return Err(self.error(format!("unknown request field '{field}'"))),
            }
            if !self.consume(Kind::Comma) && self.peek_kind() != Some(Kind::Rb) {
                return Err(self.error("expected ',' between request fields"));
            }
        }
        Ok(Request {
            name: name.clone(),
            params_def,
            path: path.ok_or_else(|| self.error(format!("request '{name}' requires path")))?,
            method: method
                .ok_or_else(|| self.error(format!("request '{name}' requires method")))?,
            params,
            headers,
            body,
        })
    }

    fn parse_test(&mut self, tags: Vec<String>) -> Result<Test, String> {
        self.expect(Kind::Test)?;
        let name = self.ident()?;
        self.expect(Kind::Lb)?;
        let mut body = Vec::new();
        while !self.consume(Kind::Rb) {
            body.push(self.parse_statement()?);
            self.expect(Kind::Semi)?;
        }
        Ok(Test { name, tags, body })
    }

    fn parse_tags(&mut self) -> Result<Vec<String>, String> {
        let mut tags = Vec::new();
        while self.consume(Kind::Tag) {
            tags.push(self.ident()?);
        }
        Ok(tags)
    }

    fn parse_statement(&mut self) -> Result<Expr, String> {
        if self.consume(Kind::Let) {
            let name = self.ident()?;
            self.expect(Kind::Assign)?;
            return Ok(Expr::Let(name, Box::new(self.parse_expr()?)));
        }
        if self.consume(Kind::Expect) {
            return Ok(Expr::Expect(Box::new(self.parse_expr()?)));
        }
        self.parse_expr()
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_binary(1)
    }

    fn parse_binary(&mut self, minimum: u8) -> Result<Expr, String> {
        let mut left = self.parse_primary()?;
        loop {
            left = match self.peek_kind() {
                Some(Kind::Dot) => {
                    self.next();
                    Expr::Field(Box::new(left), self.ident()?)
                }
                Some(Kind::Lp) => self.parse_call(left)?,
                Some(Kind::Ls) => {
                    self.next();
                    let index = self.parse_expr()?;
                    self.expect(Kind::Rs)?;
                    Expr::Index(Box::new(left), Box::new(index))
                }
                Some(kind) if precedence(&kind) >= minimum && precedence(&kind) > 0 => {
                    let token = self.next().unwrap();
                    let rule = precedence(&token.kind);
                    let right = self.parse_binary(rule + 1)?;
                    Expr::Binary(token, Box::new(left), Box::new(right))
                }
                _ => return Ok(left),
            };
        }
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        let token = self
            .next()
            .ok_or_else(|| "unexpected end of input".to_string())?;
        match token.kind {
            Kind::Ident | Kind::Env | Kind::Api => Ok(Expr::Ident(token.lite)),
            Kind::True => Ok(Expr::Boolean(true)),
            Kind::False => Ok(Expr::Boolean(false)),
            Kind::Null => Ok(Expr::Null),
            Kind::Integer => token
                .lite
                .parse()
                .map(Expr::Integer)
                .map_err(|_| self.error("invalid integer")),
            Kind::Float => token
                .lite
                .parse()
                .map(Expr::Float)
                .map_err(|_| self.error("invalid float")),
            Kind::String => Ok(Expr::String(token.lite)),
            Kind::Raw => Ok(Expr::Raw(token.lite)),
            Kind::Not | Kind::Sub => {
                let value = self.parse_binary(11)?;
                Ok(Expr::Unary(token, Box::new(value)))
            }
            Kind::Lp => {
                let expr = self.parse_expr()?;
                self.expect(Kind::Rp)?;
                Ok(Expr::Paren(Box::new(expr)))
            }
            Kind::Ls => {
                let mut values = Vec::new();
                while !self.consume(Kind::Rs) {
                    values.push(self.parse_expr()?);
                    if !self.consume(Kind::Comma) {
                        self.expect(Kind::Rs)?;
                        break;
                    }
                }
                Ok(Expr::Array(values))
            }
            Kind::Lb => Ok(Expr::Map(self.parse_fields_after_open()?)),
            _ => Err(self.error(format!("unexpected token {token}"))),
        }
    }

    fn parse_call(&mut self, function: Expr) -> Result<Expr, String> {
        self.expect(Kind::Lp)?;
        let mut arguments = Vec::new();
        while !self.consume(Kind::Rp) {
            arguments.push(self.parse_expr()?);
            if !self.consume(Kind::Comma) {
                self.expect(Kind::Rp)?;
                break;
            }
        }
        Ok(Expr::Call(Box::new(function), arguments))
    }

    fn parse_pairs(&mut self) -> Result<Vec<(Expr, Expr)>, String> {
        self.expect(Kind::Lb)?;
        let fields = self.parse_fields_after_open()?;
        Ok(fields
            .into_iter()
            .map(|(key, value)| (Expr::String(key), value))
            .collect())
    }

    fn parse_fields(&mut self) -> Result<Vec<(String, Expr)>, String> {
        self.expect(Kind::Lb)?;
        self.parse_fields_after_open()
    }

    fn parse_fields_after_open(&mut self) -> Result<Vec<(String, Expr)>, String> {
        let mut fields = Vec::new();
        while !self.consume(Kind::Rb) {
            let key = match self.next() {
                Some(Token {
                    kind: Kind::Ident | Kind::String,
                    lite,
                    ..
                }) => lite,
                Some(token) => return Err(self.error(format!("invalid field name {token}"))),
                None => return Err("unterminated object".to_string()),
            };
            self.expect(Kind::Colon)?;
            if fields.iter().any(|(existing, _)| existing == &key) {
                return Err(self.error(format!("duplicate field '{key}'")));
            }
            fields.push((key, self.parse_expr()?));
            if !self.consume(Kind::Comma) {
                self.expect(Kind::Rb)?;
                break;
            }
        }
        Ok(fields)
    }

    fn ident(&mut self) -> Result<String, String> {
        match self.next() {
            Some(Token {
                kind: Kind::Ident,
                lite,
                ..
            }) => Ok(lite),
            Some(token) => Err(self.error(format!("expected identifier, found {token}"))),
            None => Err("expected identifier, found end of input".to_string()),
        }
    }

    fn expect(&mut self, kind: Kind) -> Result<(), String> {
        match self.next() {
            Some(token) if token.kind == kind => Ok(()),
            Some(token) => Err(self.error(format!("expected {kind:?}, found {token}"))),
            None => Err(format!("expected {kind:?}, found end of input")),
        }
    }

    fn consume(&mut self, kind: Kind) -> bool {
        if self.peek_kind() == Some(kind) {
            self.index += 1;
            true
        } else {
            false
        }
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.index).cloned();
        self.index += usize::from(token.is_some());
        token
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    fn peek_kind(&self) -> Option<Kind> {
        self.peek().map(|token| token.kind.clone())
    }

    fn error(&self, message: impl AsRef<str>) -> String {
        match self.peek() {
            Some(token) => format!(
                "{}:{}: {}",
                token.span.start,
                token.span.end,
                message.as_ref()
            ),
            None => message.as_ref().to_owned(),
        }
    }
}

fn string_ends(text: &str) -> HashMap<usize, usize> {
    let bytes = text.as_bytes();
    let mut ends = HashMap::new();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if bytes[index] == b'`' {
            index += 1;
            while index < bytes.len() {
                if bytes[index] == b'\\' && bytes.get(index + 1) == Some(&b'`') {
                    index += 2;
                    continue;
                }
                if bytes[index] == b'`' {
                    index += 1;
                    break;
                }
                index += 1;
            }
            continue;
        }
        if bytes[index] != b'"' {
            index += 1;
            continue;
        }

        let start = index;
        index += 1;
        while index < bytes.len() {
            match bytes[index] {
                b'\\' if bytes.get(index + 1) == Some(&b'(') => {
                    index += 2;
                    let mut depth = 1;
                    let mut quote = None;
                    let mut escaped = false;
                    while index < bytes.len() && depth > 0 {
                        let byte = bytes[index];
                        if let Some(active) = quote {
                            if escaped {
                                escaped = false;
                            } else if byte == b'\\' {
                                escaped = true;
                            } else if byte == active {
                                quote = None;
                            }
                        } else {
                            match byte {
                                b'"' | b'`' => quote = Some(byte),
                                b'(' => depth += 1,
                                b')' => depth -= 1,
                                _ => {}
                            }
                        }
                        index += 1;
                    }
                }
                b'\\' => index = (index + 2).min(bytes.len()),
                b'"' => {
                    ends.insert(start, index);
                    index += 1;
                    break;
                }
                _ => index += 1,
            }
        }
    }
    ends
}

fn precedence(kind: &Kind) -> u8 {
    match kind {
        Kind::Lo => 1,
        Kind::La => 2,
        Kind::Bo => 3,
        Kind::Bx => 4,
        Kind::Ba => 5,
        Kind::Eq | Kind::Ne => 6,
        Kind::Lt | Kind::Gt | Kind::Le | Kind::Ge => 7,
        Kind::Sl | Kind::Sr => 8,
        Kind::Add | Kind::Sub => 9,
        Kind::Mul | Kind::Div | Kind::Rem => 10,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_v1_declarations() {
        let source = Parser::new(
            r#"
            env local { scheme: http, host: "localhost" }
            api user {
                scheme: env.scheme,
                host: env.host,
                get(id: int) { method: GET, path: "/users/\(id)" },
                text() { method: POST, path: "/text", text: `hello` },
                upload(source: file) { method: POST, path: "/files", file: source }
            }
            @smoke test getUser { let response = user.get(7); expect response.status == 200; }
        "#,
        )
        .parse()
        .unwrap();
        assert_eq!(source.environments.len(), 1);
        assert_eq!(source.apis.inner.len(), 1);
        assert_eq!(source.tests.len(), 1);
        assert!(matches!(
            source.apis.get("user").unwrap().requests["text"].body,
            Body::Text(_)
        ));
        assert!(matches!(
            source.apis.get("user").unwrap().requests["upload"].body,
            Body::File(_)
        ));
        assert!(matches!(
            source.apis.get("user").unwrap().requests["get"].body,
            Body::None
        ));
    }

    #[test]
    fn parses_unary_expressions() {
        let source = Parser::new("expect !false; let value = -1 * 2;")
            .parse()
            .unwrap();
        assert!(matches!(source.exprs[0], Expr::Expect(_)));
        let Expr::Let(_, value) = &source.exprs[1] else {
            panic!("expected let")
        };
        assert!(matches!(value.as_ref(), Expr::Binary(_, _, _)));
    }

    #[test]
    fn escaped_raw_backtick_does_not_corrupt_later_strings() {
        let source = Parser::new(
            r##"let raw = `before \` after`; let value = "prefix \("quoted") suffix";"##,
        )
        .parse()
        .unwrap();
        assert_eq!(source.exprs.len(), 2);
        let Expr::Let(_, value) = &source.exprs[1] else {
            panic!("expected let")
        };
        assert_eq!(
            value.as_ref(),
            &Expr::String(r#"prefix \("quoted") suffix"#.into())
        );
    }

    #[test]
    fn rejects_multiple_request_bodies() {
        let error = Parser::new(
            r#"api example {
            scheme: http, host: "localhost",
            create() { method: POST, path: "/", json: {}, text: "duplicate" }
        }"#,
        )
        .parse()
        .err()
        .unwrap();
        assert!(error.contains("multiple body fields"), "{error}");
    }

    #[test]
    fn parses_all_documented_parameter_types() {
        let source = Parser::new(
            r#"api typed {
                scheme: http,
                host: "localhost",
                send(a: integer, b: float, c: boolean, d: string, e: array, f: map) {
                    method: POST, path: "/", json: { values: e, object: f }
                }
            }"#,
        )
        .parse()
        .unwrap();
        assert_eq!(
            source.apis.get("typed").unwrap().requests["send"]
                .params_def
                .len(),
            6
        );
    }

    #[test]
    fn rejects_missing_test_semicolon_and_entry_comma() {
        let semicolon = Parser::new("test flow { expect true }")
            .parse()
            .unwrap_err();
        assert!(semicolon.contains("expected Semi"), "{semicolon}");

        let comma = Parser::new(r#"api example { scheme: http host: "localhost" }"#)
            .parse()
            .unwrap_err();
        assert!(comma.contains("expected ','"), "{comma}");
    }

    #[test]
    fn rejects_duplicate_parameters_and_fields() {
        let parameter = Parser::new(
            r#"api example { scheme: http, host: "localhost", get(id: int, id: int) { method: GET, path: "/" } }"#,
        )
        .parse()
        .unwrap_err();
        assert!(
            parameter.contains("duplicate parameter 'id'"),
            "{parameter}"
        );

        let field = Parser::new("env local { host: \"one\", host: \"two\" }")
            .parse()
            .unwrap_err();
        assert!(field.contains("duplicate field 'host'"), "{field}");
    }
}
