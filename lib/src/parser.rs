use super::Expr;
use super::Kind;
use super::Source;
use super::Token;
use super::lexer::Lexer;
use crate::client::Clients;
use crate::client::{Client, Request};
use crate::client::{Method, Scheme};
use std::collections::{HashMap, HashSet};

pub struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

fn normalize_object_keys(expr: Expr) -> Expr {
    match expr {
        Expr::Map(pairs) => Expr::Map(
            pairs
                .into_iter()
                .map(|(key, value)| {
                    let key = match key {
                        Expr::Ident(name) => Expr::String(name),
                        key => normalize_object_keys(key),
                    };
                    (key, normalize_object_keys(value))
                })
                .collect(),
        ),
        Expr::Array(values) => Expr::Array(values.into_iter().map(normalize_object_keys).collect()),
        expr => expr,
    }
}

impl Parser {
    pub fn new(text: &str) -> Parser {
        Parser {
            tokens: Lexer::new().segment(text),
            index: usize::MIN,
        }
    }

    fn next(&mut self) {
        self.index += 1;
    }

    fn current(&self) -> &Token {
        &self.tokens[self.index]
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.index + 1)
    }

    fn peek_equal(&self, kind: Kind) -> bool {
        matches!(self.peek(), Some(peek) if peek.kind == kind)
    }

    fn peek_expect(&mut self, kind: Kind) -> Result<(), String> {
        if let Some(peek) = self.peek() {
            if peek.kind == kind {
                self.next();
                Ok(())
            } else {
                Err(format!("token expect {:?} but found {:?}", kind, peek.kind))
            }
        } else {
            Err(format!("token expect {kind:?} but found none"))
        }
    }

    fn current_expect_ident(&self, field: &str) -> Result<String, String> {
        if matches!(&self.current().kind, Kind::Ident) {
            Ok(self.parse_current_string())
        } else {
            Err(format!("{field} must be an identifier"))
        }
    }

    fn current_rule(&self) -> u8 {
        self.current().rule()
    }

    fn peek_rule(&self) -> u8 {
        if let Some(peek) = self.peek() {
            peek.rule()
        } else {
            u8::MIN
        }
    }

    fn error(&self, error: String) -> String {
        let token = self.current();
        format!("{}:{}: {error}", token.span.start, token.span.end)
    }

    pub fn parse(&mut self) -> Result<Source, String> {
        let mut exprs = Vec::new();
        let mut functions = HashMap::new();
        let mut tests = HashMap::new();
        let mut clients = Clients::new();
        while self.index < self.tokens.len() {
            match self.current().kind {
                Kind::Client => {
                    let client = self
                        .parse_client_literal()
                        .map_err(|error| self.error(error))?;
                    clients.insert(client).map_err(|error| self.error(error))?;
                }
                Kind::Function => {
                    if let Expr::Function(name, params, block) = self
                        .parse_function_literal()
                        .map_err(|error| self.error(error))?
                    {
                        functions.insert(name, (params, block));
                    }
                }
                Kind::Test => {
                    let (name, block) = self
                        .parse_test_literal()
                        .map_err(|error| self.error(error))?;
                    tests.insert(name, block);
                }
                _ => exprs.push(
                    self.parse_expr(u8::MIN)
                        .map_err(|error| self.error(error))?,
                ),
            }
            if self.peek_equal(Kind::Semi) {
                self.next();
            }
            self.next();
        }
        Ok(Source {
            exprs,
            functions,
            clients,
            tests,
        })
    }

    fn parse_expr(&mut self, mut rule: u8) -> Result<Expr, String> {
        let mut left = match self.current().kind {
            Kind::Ident => self.parse_ident_expr(),
            Kind::Integer => self.parse_integer_literal()?,
            Kind::Float => self.parse_float_literal()?,
            Kind::True | Kind::False => self.parse_boolean_literal()?,
            Kind::String => self.parse_string_literal(),
            Kind::Let => {
                rule = u8::MAX;
                self.parse_let_expr()?
            }
            Kind::Not | Kind::Sub => self.parse_unary_expr()?,
            Kind::Lp => self.parse_paren_expr()?,
            Kind::If => self.parse_if_expr()?,
            Kind::Break => self.parse_break_expr()?,
            Kind::Continue => self.parse_continue_expr()?,
            Kind::Loop => self.parse_loop_expr()?,
            Kind::While => self.parse_while_expr()?,
            Kind::For => self.parse_for_expr()?,
            Kind::Open | Kind::Close => self.parse_range_expr(None)?,
            Kind::Ls => self.parse_array_literal()?,
            Kind::Lb => self.parse_map_literal()?,
            _ => Err(format!("parse expr error: {}", self.current()))?,
        };
        while !self.peek_equal(Kind::Semi) && rule < self.peek_rule() {
            left = match self.peek() {
                Some(Token {
                    kind: Kind::Add, ..
                })
                | Some(Token {
                    kind: Kind::Sub, ..
                })
                | Some(Token {
                    kind: Kind::Mul, ..
                })
                | Some(Token {
                    kind: Kind::Div, ..
                })
                | Some(Token {
                    kind: Kind::Rem, ..
                })
                | Some(Token { kind: Kind::Bx, .. })
                | Some(Token { kind: Kind::Bo, .. })
                | Some(Token { kind: Kind::Ba, .. })
                | Some(Token { kind: Kind::Sl, .. })
                | Some(Token { kind: Kind::Sr, .. })
                | Some(Token { kind: Kind::La, .. })
                | Some(Token { kind: Kind::Lo, .. })
                | Some(Token { kind: Kind::Lt, .. })
                | Some(Token { kind: Kind::Gt, .. })
                | Some(Token { kind: Kind::Le, .. })
                | Some(Token { kind: Kind::Ge, .. })
                | Some(Token { kind: Kind::Eq, .. })
                | Some(Token { kind: Kind::Ne, .. }) => {
                    self.next();
                    self.parse_binary_expr(left)?
                }
                Some(Token {
                    kind: Kind::Open, ..
                })
                | Some(Token {
                    kind: Kind::Close, ..
                }) => {
                    self.next();
                    self.parse_range_expr(Some(left))?
                }
                Some(Token { kind: Kind::Lp, .. }) => {
                    self.next();
                    self.parse_call_expr(left)?
                }
                Some(Token { kind: Kind::Ls, .. }) => {
                    self.next();
                    self.parse_index_expr(left)?
                }
                Some(Token {
                    kind: Kind::Dot, ..
                }) => {
                    self.next();
                    self.parse_field_expr(left)?
                }
                _ => left,
            };
        }
        Ok(left)
    }

    fn parse_ident_expr(&self) -> Expr {
        Expr::Ident(self.parse_current_string())
    }

    fn parse_current_string(&self) -> String {
        self.current().lite.to_owned()
    }

    fn parse_integer_literal(&self) -> Result<Expr, String> {
        let token = self.current();
        match token.lite.parse::<i64>() {
            Ok(integer) => Ok(Expr::Integer(integer)),
            Err(_) => Err(format!("parse integer error: {token}")),
        }
    }

    fn parse_float_literal(&self) -> Result<Expr, String> {
        let token = self.current();
        match token.lite.parse::<f64>() {
            Ok(float) => Ok(Expr::Float(float)),
            Err(_) => Err(format!("parse float error: {token}")),
        }
    }

    fn parse_boolean_literal(&self) -> Result<Expr, String> {
        let token = self.current();
        match token.kind {
            Kind::True => Ok(Expr::Boolean(true)),
            Kind::False => Ok(Expr::Boolean(false)),
            _ => Err(format!("parse boolean error: {token}")),
        }
    }

    fn parse_string_literal(&self) -> Expr {
        Expr::String(self.parse_current_string())
    }

    fn parse_let_expr(&mut self) -> Result<Expr, String> {
        self.peek_expect(Kind::Ident)?;
        let name = self.parse_current_string();
        self.peek_expect(Kind::Assign)?;
        self.next();
        let value = self.parse_expr(u8::MIN)?;
        if self.peek_equal(Kind::Semi) {
            self.next();
        }
        Ok(Expr::Let(name, Box::new(value)))
    }

    fn parse_unary_expr(&mut self) -> Result<Expr, String> {
        let token = self.current().clone();
        let mut rule = self.current_rule();
        (token.kind == Kind::Sub).then(|| {
            rule += 2;
        });
        self.next();
        let right = self.parse_expr(rule)?;
        Ok(Expr::Unary(token, Box::new(right)))
    }

    fn parse_binary_expr(&mut self, left: Expr) -> Result<Expr, String> {
        let token = self.current().clone();
        let rule = self.current_rule();
        self.next();
        let right = self.parse_expr(rule)?;
        Ok(Expr::Binary(token, Box::new(left), Box::new(right)))
    }

    fn parse_paren_expr(&mut self) -> Result<Expr, String> {
        self.next();
        let expr = self.parse_expr(u8::MIN)?;
        self.peek_expect(Kind::Rp)?;
        Ok(Expr::Paren(Box::new(expr)))
    }

    fn parse_if_expr(&mut self) -> Result<Expr, String> {
        self.peek_expect(Kind::Lp)?;
        self.next();
        let condition = self.parse_expr(u8::MIN)?;
        self.peek_expect(Kind::Rp)?;
        let consequence = self.parse_block_expr()?;
        let mut alternative = Vec::new();
        if self.peek_equal(Kind::Else) {
            self.next();
            alternative = self.parse_block_expr()?;
        }
        Ok(Expr::If(Box::new(condition), consequence, alternative))
    }

    fn parse_break_expr(&mut self) -> Result<Expr, String> {
        let value = if self.peek_starts_expr() {
            self.next();
            Some(Box::new(self.parse_expr(u8::MIN)?))
        } else {
            None
        };
        Ok(Expr::Break(value))
    }

    fn parse_continue_expr(&self) -> Result<Expr, String> {
        Ok(Expr::Continue)
    }

    fn parse_loop_expr(&mut self) -> Result<Expr, String> {
        let body = self.parse_block_expr()?;
        Ok(Expr::Loop(body))
    }

    fn parse_while_expr(&mut self) -> Result<Expr, String> {
        self.peek_expect(Kind::Lp)?;
        self.next();
        let condition = self.parse_expr(u8::MIN)?;
        self.peek_expect(Kind::Rp)?;
        let body = self.parse_block_expr()?;
        Ok(Expr::While(Box::new(condition), body))
    }

    fn parse_for_expr(&mut self) -> Result<Expr, String> {
        let mut bindings = Vec::new();
        loop {
            self.peek_expect(Kind::Ident)?;
            bindings.push(self.parse_current_string());
            if self.peek_equal(Kind::In) {
                break;
            }
            self.peek_expect(Kind::Comma)?;
        }
        self.peek_expect(Kind::In)?;
        self.next();
        let iterator = self.parse_expr(u8::MIN)?;
        let body = self.parse_block_expr()?;
        Ok(Expr::For(bindings, Box::new(iterator), body))
    }

    fn parse_range_expr(&mut self, start: Option<Expr>) -> Result<Expr, String> {
        let half = self.current().kind == Kind::Close;
        let end = if self.peek_starts_range_end() {
            let rule = self.current_rule();
            self.next();
            Some(Box::new(self.parse_expr(rule)?))
        } else {
            None
        };
        Ok(Expr::Range(start.map(Box::new), end, half))
    }

    fn peek_starts_expr(&self) -> bool {
        matches!(
            self.peek().map(|token| &token.kind),
            Some(
                Kind::Ident
                    | Kind::Integer
                    | Kind::Float
                    | Kind::True
                    | Kind::False
                    | Kind::String
                    | Kind::Let
                    | Kind::Not
                    | Kind::Sub
                    | Kind::Lp
                    | Kind::If
                    | Kind::Break
                    | Kind::Continue
                    | Kind::Loop
                    | Kind::While
                    | Kind::For
                    | Kind::Open
                    | Kind::Close
                    | Kind::Ls
                    | Kind::Lb
            )
        )
    }

    fn peek_starts_range_end(&self) -> bool {
        self.peek_starts_expr() && !self.peek_equal(Kind::Lb)
    }

    fn parse_call_expr(&mut self, function: Expr) -> Result<Expr, String> {
        let arguments = self.parse_expr_list(Kind::Rp)?;
        Ok(Expr::Call(Box::new(function), arguments))
    }

    fn parse_expr_list(&mut self, end: Kind) -> Result<Vec<Expr>, String> {
        let mut exprs = Vec::new();
        while !self.peek_equal(end.clone()) {
            self.next();
            exprs.push(self.parse_expr(u8::MIN)?);
            if !self.peek_equal(end.clone()) {
                self.peek_expect(Kind::Comma)?;
            }
        }
        self.peek_expect(end)?;
        Ok(exprs)
    }

    fn parse_array_literal(&mut self) -> Result<Expr, String> {
        let items = self.parse_expr_list(Kind::Rs)?;
        Ok(Expr::Array(items))
    }

    fn parse_map_literal(&mut self) -> Result<Expr, String> {
        let mut pairs = Vec::new();
        while !self.peek_equal(Kind::Rb) {
            self.next();
            let key = self.parse_expr(u8::MIN)?;
            self.peek_expect(Kind::Colon)?;
            self.next();
            let value = self.parse_expr(u8::MIN)?;
            pairs.push((key, value));
            if !self.peek_equal(Kind::Rb) {
                self.peek_expect(Kind::Comma)?;
            }
        }
        self.peek_expect(Kind::Rb)?;
        Ok(Expr::Map(pairs))
    }

    fn parse_client_literal(&mut self) -> Result<Client, String> {
        self.peek_expect(Kind::Ident)?;
        let name = self.parse_current_string();
        self.peek_expect(Kind::Lb)?;

        let mut fields = HashSet::new();
        let mut scheme = None;
        let mut host = None;
        let mut port = None;
        let mut requests = None;

        while !self.peek_equal(Kind::Rb) {
            self.next();
            let field = self.parse_object_field_name("client")?;
            if !fields.insert(field.clone()) {
                return Err(format!("duplicate client field '{field}'"));
            }
            self.peek_expect(Kind::Colon)?;
            self.next();
            match field.as_str() {
                "scheme" => {
                    let value = self.current_expect_ident("scheme")?;
                    scheme = Some(match value.as_str() {
                        "http" | "https" => Scheme::from(value.as_str()),
                        _ => return Err(format!("unknown scheme '{value}'")),
                    });
                }
                "host" => host = Some(self.parse_expr(u8::MIN)?),
                "port" => {
                    let token = self.current();
                    if !matches!(&token.kind, Kind::Integer) {
                        return Err("client port must be an integer".to_string());
                    }
                    port = Some(
                        token
                            .lite
                            .parse::<u16>()
                            .map_err(|_| format!("invalid port '{}'", token))?,
                    );
                }
                "requests" => requests = Some(self.parse_requests_literal()?),
                _ => return Err(format!("unknown client field '{field}'")),
            }
            self.consume_object_separator(Kind::Rb)?;
        }
        self.peek_expect(Kind::Rb)?;

        let scheme = scheme.ok_or_else(|| "missing required client field 'scheme'".to_string())?;
        let host = host.ok_or_else(|| "missing required client field 'host'".to_string())?;
        let requests =
            requests.ok_or_else(|| "missing required client field 'requests'".to_string())?;
        if requests.is_empty() {
            return Err("requests must contain at least one entry".to_string());
        }
        Ok(Client {
            name,
            scheme,
            host,
            port,
            requests,
        })
    }

    fn parse_requests_literal(&mut self) -> Result<HashMap<String, Request>, String> {
        if self.current().kind != Kind::Lb {
            return Err("requests must be an object".to_string());
        }
        let mut requests = HashMap::new();
        while !self.peek_equal(Kind::Rb) {
            self.next();
            let name = self.parse_object_field_name("request")?;
            if requests.contains_key(&name) {
                return Err(format!("duplicate request '{name}'"));
            }
            self.peek_expect(Kind::Colon)?;
            self.next();
            let request = self.parse_request_literal(&name)?;
            requests.insert(name, request);
            self.consume_object_separator(Kind::Rb)?;
        }
        self.peek_expect(Kind::Rb)?;
        Ok(requests)
    }

    fn parse_request_literal(&mut self, request_name: &str) -> Result<Request, String> {
        if self.current().kind != Kind::Lb {
            return Err(format!("request '{request_name}' must be an object"));
        }
        let mut fields = HashSet::new();
        let mut path = None;
        let mut method = None;
        let mut params = Vec::new();
        let mut headers = Vec::new();
        let mut body = None;
        let mut asserts = Vec::new();

        while !self.peek_equal(Kind::Rb) {
            self.next();
            let field = self.parse_object_field_name("request")?;
            if !fields.insert(field.clone()) {
                return Err(format!(
                    "duplicate field '{field}' in request '{request_name}'"
                ));
            }
            self.peek_expect(Kind::Colon)?;
            self.next();
            match field.as_str() {
                "path" => path = Some(self.parse_expr(u8::MIN)?),
                "method" => {
                    let value = self.current_expect_ident("method")?;
                    method = Some(match value.as_str() {
                        "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "OPTIONS" | "HEAD"
                        | "TRACE" | "CONNECT" => Method::from(value.as_str()),
                        _ => return Err(format!("unknown method '{value}'")),
                    });
                }
                "headers" => headers = self.parse_pairs_literal("headers")?,
                "params" => params = self.parse_pairs_literal("params")?,
                "body" => body = Some(normalize_object_keys(self.parse_expr(u8::MIN)?)),
                "asserts" => {
                    let Expr::Array(values) = self.parse_expr(u8::MIN)? else {
                        return Err("asserts must be an array".to_string());
                    };
                    asserts = values;
                }
                _ => return Err(format!("unknown request field '{field}'")),
            }
            self.consume_object_separator(Kind::Rb)?;
        }
        self.peek_expect(Kind::Rb)?;

        Ok(Request {
            path: path.ok_or_else(|| format!("request '{request_name}' is missing 'path'"))?,
            method: method
                .ok_or_else(|| format!("request '{request_name}' is missing 'method'"))?,
            params,
            headers,
            body,
            asserts,
        })
    }

    fn parse_pairs_literal(&mut self, field: &str) -> Result<Vec<(Expr, Expr)>, String> {
        let Expr::Array(entries) = self.parse_expr(u8::MIN)? else {
            return Err(format!("{field} must be an array of key-value pairs"));
        };
        entries
            .into_iter()
            .map(|entry| match entry {
                Expr::Array(mut pair) if pair.len() == 2 => {
                    let value = pair.pop().unwrap();
                    let key = pair.pop().unwrap();
                    Ok((key, value))
                }
                _ => Err(format!(
                    "each {field} entry must contain exactly two values"
                )),
            })
            .collect()
    }

    fn parse_object_field_name(&self, object: &str) -> Result<String, String> {
        match self.current().kind {
            Kind::Ident | Kind::String => Ok(self.parse_current_string()),
            _ => Err(format!(
                "{object} field name must be an identifier or string"
            )),
        }
    }

    fn consume_object_separator(&mut self, end: Kind) -> Result<(), String> {
        if !self.peek_equal(end) {
            self.peek_expect(Kind::Comma)?;
        }
        Ok(())
    }

    fn parse_index_expr(&mut self, left: Expr) -> Result<Expr, String> {
        self.next();
        let index = self.parse_expr(u8::MIN)?;
        self.peek_expect(Kind::Rs)?;
        Ok(Expr::Index(Box::new(left), Box::new(index)))
    }

    fn parse_field_expr(&mut self, left: Expr) -> Result<Expr, String> {
        self.peek_expect(Kind::Ident)?;
        let field = self.parse_current_string();
        Ok(Expr::Field(Box::new(left), field))
    }

    fn parse_block_expr(&mut self) -> Result<Vec<Expr>, String> {
        let mut exprs = Vec::new();
        self.peek_expect(Kind::Lb)?;
        while !self.peek_equal(Kind::Rb) {
            self.next();
            exprs.push(self.parse_expr(u8::MIN)?);
            if self.peek_equal(Kind::Semi) {
                self.next();
            }
        }
        self.peek_expect(Kind::Rb)?;
        Ok(exprs)
    }

    fn parse_function_literal(&mut self) -> Result<Expr, String> {
        self.peek_expect(Kind::Ident)?;
        let name = self.parse_current_string();
        self.peek_expect(Kind::Lp)?;
        let mut params = Vec::new();
        while !self.peek_equal(Kind::Rp) {
            self.peek_expect(Kind::Ident)?;
            params.push(self.parse_current_string());
            if !self.peek_equal(Kind::Rp) {
                self.peek_expect(Kind::Comma)?;
            }
        }
        self.peek_expect(Kind::Rp)?;
        let block = self.parse_block_expr()?;
        Ok(Expr::Function(name, params, block))
    }

    fn parse_test_literal(&mut self) -> Result<(String, Vec<Expr>), String> {
        self.peek_expect(Kind::Ident)?;
        let name = self.parse_current_string();
        let block = self.parse_block_expr()?;
        Ok((name, block))
    }
}

#[test]
fn test_parse_let_expr() {
    let tests = vec![
        ("let x =  5;", 1, "let x 5"),
        ("let x =  5 let y = 6", 2, "let x 5"),
        ("let y  = true;", 1, "let y true"),
        ("let  foobar = y;", 1, "let foobar y"),
        ("let i = 0; [1][i];", 2, "let i 0"),
    ];
    for (text, len, expected) in tests {
        match Parser::new(text).parse() {
            Ok(Source { exprs, .. }) => {
                println!("{}", exprs.len());
                println!("{exprs:?}");
                assert!(exprs.len() == len);
                if let Some(expr) = exprs.first() {
                    println!("{expr}");
                    if let Expr::Let(name, value) = expr {
                        let parsed = format!("let {name} {value}");
                        println!("{expected}={parsed}");
                        assert!(expected == parsed);
                    } else {
                        unreachable!("let expr parse failed")
                    }
                } else {
                    unreachable!("exprs expr none")
                }
            }
            Err(error) => {
                unreachable!("{}", error)
            }
        }
    }
}

#[test]
fn test_parse_ident_expr() {
    let text = "foobar;";
    if let Ok(Source { exprs, .. }) = Parser::new(text).parse() {
        assert!(exprs.len() == 1);
        if let Some(expr) = exprs.first() {
            println!("{expr}");
            if let Expr::Ident(value) = expr {
                assert!(value == "foobar");
            } else {
                unreachable!("ident expr parse failed")
            }
        } else {
            unreachable!("exprs expr none")
        }
    }
}

#[test]
fn test_parse_integer_literal() {
    let text = "5;";
    if let Ok(Source { exprs, .. }) = Parser::new(text).parse() {
        assert!(exprs.len() == 1);
        if let Some(expr) = exprs.first() {
            println!("{expr}");
            if let Expr::Integer(value) = *expr {
                assert!(value == 5);
            } else {
                unreachable!("integer literal parse failed")
            }
        } else {
            unreachable!("exprs expr none")
        }
    }
}

#[test]
fn test_parse_float_literal() {
    let text = "3.14159265358979323846264338327950288;";
    if let Ok(Source { exprs, .. }) = Parser::new(text).parse() {
        assert!(exprs.len() == 1);
        if let Some(expr) = exprs.first() {
            println!("expr:{expr}");
            if let Expr::Float(value) = *expr {
                println!("value:{value}");
                assert!(value == core::f64::consts::PI);
            } else {
                unreachable!("float literal parse failed")
            }
        } else {
            unreachable!("exprs expr none")
        }
    }
}

#[test]
fn test_parse_boolean_literal() {
    let tests = vec![("true;", true), ("false;", false)];
    for (text, expected) in tests {
        if let Ok(Source { exprs, .. }) = Parser::new(text).parse() {
            assert!(exprs.len() == 1);
            if let Some(expr) = exprs.first() {
                println!("{expr}");
                if let Expr::Boolean(value) = *expr {
                    assert!(value == expected);
                } else {
                    unreachable!("boolean literal parse failed")
                }
            } else {
                unreachable!("exprs expr none")
            }
        }
    }
}

#[test]
fn test_parse_string_literal() {
    let text = r#""hello world";"#;
    if let Ok(Source { exprs, .. }) = Parser::new(text).parse() {
        assert!(exprs.len() == 1);
        if let Some(expr) = exprs.first() {
            println!("{expr}");
            if let Expr::String(value) = expr {
                assert!(value == "hello world");
            } else {
                unreachable!("string literal parse failed")
            }
        } else {
            unreachable!("exprs expr none")
        }
    }
}

#[test]
fn test_parse_unary_expr() {
    let tests = vec![
        ("!5;", "!", "5"),
        ("-15;", "-", "15"),
        ("~5;", "~", "5"),
        ("!foobar;", "!", "foobar"),
        ("-foobar;", "-", "foobar"),
        ("!true;", "!", "true"),
        ("!false;", "!", "false"),
    ];
    for (text, expected_operator, expected_right) in tests {
        if let Ok(Source { exprs, .. }) = Parser::new(text).parse() {
            assert!(exprs.len() == 1);
            if let Some(expr) = exprs.first() {
                println!("{expr}");
                if let Expr::Unary(token, right) = expr {
                    assert!(expected_operator == token.to_string());
                    assert!(expected_right == right.to_string());
                } else {
                    unreachable!("unary expr parse failed")
                }
            } else {
                unreachable!("exprs expr none")
            }
        }
    }
}

#[test]
fn test_parse_binary_expr() {
    let tests = vec![
        ("5 + 5;", "5", "+", "5"),
        ("5 - 5;", "5", "-", "5"),
        ("5 * 5;", "5", "*", "5"),
        ("5 / 5;", "5", "/", "5"),
        ("5 > 5;", "5", ">", "5"),
        ("5 < 5;", "5", "<", "5"),
        ("5 >= 5;", "5", ">=", "5"),
        ("5 <= 5;", "5", "<=", "5"),
        ("5 == 5;", "5", "==", "5"),
        ("5 != 5;", "5", "!=", "5"),
        ("foobar + barfoo;", "foobar", "+", "barfoo"),
        ("foobar - barfoo;", "foobar", "-", "barfoo"),
        ("foobar * barfoo;", "foobar", "*", "barfoo"),
        ("foobar / barfoo;", "foobar", "/", "barfoo"),
        ("foobar > barfoo;", "foobar", ">", "barfoo"),
        ("foobar < barfoo;", "foobar", "<", "barfoo"),
        ("foobar == barfoo;", "foobar", "==", "barfoo"),
        ("foobar != barfoo;", "foobar", "!=", "barfoo"),
        ("true == true", "true", "==", "true"),
        ("true != false", "true", "!=", "false"),
        ("false == false", "false", "==", "false"),
        ("1^0", "1", "^", "0"),
        ("1&0", "1", "&", "0"),
        ("1|0", "1", "|", "0"),
        ("true&&false", "true", "&&", "false"),
        ("false||true", "false", "||", "true"),
    ];
    for (text, expected_left, expected_operator, expected_right) in tests {
        match Parser::new(text).parse() {
            Ok(Source { exprs, .. }) => {
                assert!(exprs.len() == 1);
                if let Some(expr) = exprs.first() {
                    println!("{expr}");
                    if let Expr::Binary(token, left, right) = expr {
                        assert!(expected_left == left.to_string());
                        assert!(expected_operator == token.to_string());
                        assert!(expected_right == right.to_string());
                    } else {
                        unreachable!("binary expr parse failed")
                    }
                } else {
                    unreachable!("exprs expr none")
                }
            }
            Err(error) => {
                unreachable!("exprs expr error:{}", error);
            }
        }
    }
}

#[test]
fn test_parse_operator_rule() {
    let tests = vec![
        ("-a * b", "((-a) * b)"),
        ("!-a", "(!(-a))"),
        ("a + b + c", "((a + b) + c)"),
        ("a + b - c", "((a + b) - c)"),
        ("a * b * c", "((a * b) * c)"),
        ("a * b / c", "((a * b) / c)"),
        ("a + b / c", "(a + (b / c))"),
        ("a + b * c + d / e - f", "(((a + (b * c)) + (d / e)) - f)"),
        ("3 + 4; -5 * 6", "(3 + 4)((-5) * 6)"),
        ("5 > 4 == 3 < 4", "((5 > 4) == (3 < 4))"),
        ("5 < 4 != 3 > 4", "((5 < 4) != (3 > 4))"),
        (
            "3 + 4 * 5 == 3 * 1 + 4 * 5",
            "((3 + (4 * 5)) == ((3 * 1) + (4 * 5)))",
        ),
        ("true", "true"),
        ("false", "false"),
        ("3 > 5 == false", "((3 > 5) == false)"),
        ("3 < 5 == true", "((3 < 5) == true)"),
        ("1 + (2 + 3) + 4", "((1 + (2 + 3)) + 4)"),
        ("(5 + 5) * 2", "((5 + 5) * 2)"),
        ("2 / (5 + 5)", "(2 / (5 + 5))"),
        ("(5 + 5) * 2 * (5 + 5)", "(((5 + 5) * 2) * (5 + 5))"),
        ("-(5 + 5)", "(-(5 + 5))"),
        ("!(true == true)", "(!(true == true))"),
        ("a + add(b * c) + d", "((a + add((b * c))) + d)"),
        (
            "add(a, b, 1, 2 * 3, 4 + 5, add(6, 7 * 8))",
            "add(a, b, 1, (2 * 3), (4 + 5), add(6, (7 * 8)))",
        ),
        (
            "add(a + b + c * d / f + g)",
            "add((((a + b) + ((c * d) / f)) + g))",
        ),
        (
            "a * [1, 2, 3, 4][b * c] * d",
            "((a * ([1, 2, 3, 4][(b * c)])) * d)",
        ),
        (
            "add(a * b[2], b[1], 2 * [1, 2][1])",
            "add((a * (b[2])), (b[1]), (2 * ([1, 2][1])))",
        ),
        ("!add()", "(!add())"),
        ("-add()", "(-add())"),
        ("!array[1]", "(!(array[1]))"),
        ("-left.field", "(-left.field)"),
        ("3 > 2 && 2 > 1", "((3 > 2) && (2 > 1))"),
        ("a || b * c", "(a || (b * c))"),
        ("a && b < c", "(a && (b < c))"),
        ("b + c || a", "((b + c) || a)"),
        ("b < c & a", "((b < c) & a)"),
    ];
    for (text, expected) in tests {
        match Parser::new(text).parse() {
            Ok(Source { exprs, .. }) => {
                let actual: String = exprs.iter().fold(String::new(), |mut output, e| {
                    use std::fmt::Write;
                    let _ = write!(output, "{e:?}");
                    output
                });
                println!("{actual}=={expected}");
                assert_eq!(actual, expected);
            }
            Err(error) => {
                unreachable!("exprs expr error:{}", error);
            }
        }
    }
}

#[test]
fn test_parse_if_expr() {
    let text = "if (x < y) { x }";
    if let Ok(Source { exprs, .. }) = Parser::new(text).parse() {
        assert!(exprs.len() == 1);
        if let Some(expr) = exprs.first() {
            println!("{expr}");
            if let Expr::If(condition, consequence, alternative) = expr {
                assert!(condition.to_string() == "x < y");
                assert!(consequence[0].to_string() == "x");
                assert!(alternative.is_empty())
            } else {
                unreachable!("if expr parse failed")
            }
        } else {
            unreachable!("exprs expr none")
        }
    }
}

#[test]
fn test_parse_if_else_expr() {
    let text = "if (x < y) { z;x } else { y }";
    if let Ok(Source { exprs, .. }) = Parser::new(text).parse() {
        assert!(exprs.len() == 1);
        if let Some(expr) = exprs.first() {
            println!("{expr}");
            if let Expr::If(condition, consequence, alternative) = expr {
                assert!(condition.to_string() == "x < y");
                assert!(consequence[0].to_string() == "z");
                assert!(consequence[1].to_string() == "x");
                assert!(alternative[0].to_string() == "y")
            } else {
                unreachable!("if expr parse failed")
            }
        } else {
            unreachable!("exprs expr none")
        }
    }
}

#[test]
fn test_parse_array_literal_empty() {
    let text = "[]";
    if let Ok(Source { exprs, .. }) = Parser::new(text).parse() {
        assert!(exprs.len() == 1);
        if let Some(expr) = exprs.first() {
            println!("{expr}");
            if let Expr::Array(items) = expr {
                assert!(items.is_empty());
            } else {
                unreachable!("array literal parse failed")
            }
        } else {
            unreachable!("exprs expr none")
        }
    }
}

#[test]
fn test_parse_array_literal() {
    let text = "[1, 2 * 2, 3 + 3]";
    if let Ok(Source { exprs, .. }) = Parser::new(text).parse() {
        assert!(exprs.len() == 1);
        if let Some(expr) = exprs.first() {
            println!("{expr}");
            if let Expr::Array(items) = expr {
                assert!(items.len() == 3);
                assert!(items[0].to_string() == "1");
                assert!(items[1].to_string() == "2 * 2");
                assert!(items[2].to_string() == "3 + 3");
            } else {
                unreachable!("array literal parse failed")
            }
        } else {
            unreachable!("exprs expr none")
        }
    }
}

#[test]
fn test_parse_index_expr() {
    let text = "myArray[1 + 1]";
    if let Ok(Source { exprs, .. }) = Parser::new(text).parse() {
        assert!(exprs.len() == 1);
        if let Some(expr) = exprs.first() {
            println!("{expr}");
            if let Expr::Index(left, index) = expr {
                assert!(left.to_string() == "myArray");
                assert!(index.to_string() == "1 + 1");
            } else {
                unreachable!("index expr parse failed")
            }
        } else {
            unreachable!("exprs expr none")
        }
    }
}

#[test]
fn test_parse_field_expr() {
    let text = "left.field";
    if let Ok(Source { exprs, .. }) = Parser::new(text).parse() {
        assert!(exprs.len() == 1);
        if let Some(expr) = exprs.first() {
            println!("{expr}");
            if let Expr::Field(left, field) = expr {
                assert!(left.to_string() == "left");
                assert!(field == "field");
            } else {
                unreachable!("field expr parse failed")
            }
        } else {
            unreachable!("exprs expr none")
        }
    }
}

#[test]
fn test_parse_map_literal_empty() {
    let text = "{}";
    if let Ok(Source { exprs, .. }) = Parser::new(text).parse() {
        assert!(exprs.len() == 1);
        if let Some(expr) = exprs.first() {
            println!("{expr}");
            if let Expr::Map(pairs) = expr {
                assert!(pairs.is_empty());
            } else {
                unreachable!("map literal parse failed")
            }
        } else {
            unreachable!("exprs expr none")
        }
    }
}

#[test]
fn test_parse_map_literal_one_element() {
    let text = "{1: true}";
    if let Ok(Source { exprs, .. }) = Parser::new(text).parse() {
        assert!(exprs.len() == 1);
        if let Some(expr) = exprs.first() {
            println!("{expr}");
            if let Expr::Map(pairs) = expr {
                assert!(pairs.len() == 1);
                assert!(
                    pairs
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect::<Vec<(String, String)>>()
                        == [("1".to_string(), "true".to_string())]
                );
            } else {
                unreachable!("map literal parse failed")
            }
        } else {
            unreachable!("exprs expr none")
        }
    }
}

#[test]
fn test_parse_map_literal_string_key() {
    let text = r#"{"one": 1, "two": 2, "three": 3}"#;
    let expected = vec![
        (String::from("\"one\""), String::from("1")),
        (String::from("\"two\""), String::from("2")),
        (String::from("\"three\""), String::from("3")),
    ];
    if let Ok(Source { exprs, .. }) = Parser::new(text).parse() {
        assert!(exprs.len() == 1);
        if let Some(expr) = exprs.first() {
            println!("{expr}");
            if let Expr::Map(pairs) = expr {
                assert!(
                    pairs
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect::<Vec<(String, String)>>()
                        == expected
                );
            } else {
                unreachable!("map literal parse failed")
            }
        } else {
            unreachable!("exprs expr none")
        }
    }
}

#[test]
fn test_parse_map_literal_boolean_key() {
    let text = r#"{true: 1, false: 2}"#;
    let expected = vec![
        (String::from("true"), String::from("1")),
        (String::from("false"), String::from("2")),
    ];
    if let Ok(Source { exprs, .. }) = Parser::new(text).parse() {
        assert!(exprs.len() == 1);
        if let Some(expr) = exprs.first() {
            println!("{expr}");
            if let Expr::Map(pairs) = expr {
                assert!(
                    pairs
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect::<Vec<(String, String)>>()
                        == expected
                );
            } else {
                unreachable!("map literal parse failed")
            }
        } else {
            unreachable!("exprs expr none")
        }
    }
}

#[test]
fn test_parse_map_literal_integer_key() {
    let text = r#"{1: 1, 2: 2, 3: 3}"#;
    let expected = vec![
        (String::from("1"), String::from("1")),
        (String::from("2"), String::from("2")),
        (String::from("3"), String::from("3")),
    ];
    if let Ok(Source { exprs, .. }) = Parser::new(text).parse() {
        assert!(exprs.len() == 1);
        if let Some(expr) = exprs.first() {
            println!("{expr}");
            if let Expr::Map(pairs) = expr {
                assert!(
                    pairs
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect::<Vec<(String, String)>>()
                        == expected
                );
            } else {
                unreachable!("map literal parse failed")
            }
        } else {
            unreachable!("exprs expr none")
        }
    }
}

#[test]
fn test_parse_map_literal_with_expr() {
    let text = r#"{"one": 0 + 1, "two": 10 - 8, "three": 15 / 5}"#;
    let expected = vec![
        (String::from("\"one\""), String::from("0 + 1")),
        (String::from("\"two\""), String::from("10 - 8")),
        (String::from("\"three\""), String::from("15 / 5")),
    ];
    if let Ok(Source { exprs, .. }) = Parser::new(text).parse() {
        assert!(exprs.len() == 1);
        if let Some(expr) = exprs.first() {
            println!("{expr}");
            if let Expr::Map(pairs) = expr {
                assert!(
                    pairs
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect::<Vec<(String, String)>>()
                        == expected
                );
            } else {
                unreachable!("map literal parse failed")
            }
        } else {
            unreachable!("exprs expr none")
        }
    }
}

#[test]
fn test_parse_call_expr() {
    let text = "add(1, 2 * 3, 4 + 5);";
    if let Ok(Source { exprs, .. }) = Parser::new(text).parse() {
        assert!(exprs.len() == 1);
        if let Some(expr) = exprs.first() {
            println!("{expr}");
            if let Expr::Call(function, arguments) = expr {
                assert!(function.to_string() == "add");
                assert!(arguments[0].to_string() == "1");
                assert!(arguments[1].to_string() == "2 * 3");
                assert!(arguments[2].to_string() == "4 + 5");
            } else {
                unreachable!("call expr parse failed")
            }
        } else {
            unreachable!("exprs expr none")
        }
    }
}

#[test]
fn test_parse_client_request_call_expr() {
    let text = "user.getIp();";
    let Source { exprs, .. } = Parser::new(text).parse().unwrap();
    assert!(exprs.len() == 1);
    if let Some(Expr::Call(function, arguments)) = exprs.first() {
        assert!(function.to_string() == "user.getIp");
        assert!(arguments.is_empty());
    } else {
        unreachable!("client request call expr parse failed")
    }
}

#[test]
fn test_parse_native_client_definition() {
    let source = Parser::new(
        r#"
        client user {
            scheme: https,
            host: "example.com",
            port: 8443,
            requests: {
                get: {
                    path: "/get",
                    method: GET,
                    headers: [["a", "b"]],
                    params: [["tag", "a"], ["tag", "b"]],
                    asserts: [status == 200],
                },
            },
        }
        "#,
    )
    .parse()
    .unwrap();
    let client = source.clients.get("user").unwrap();
    assert_eq!(client.scheme.as_ref(), "https");
    assert_eq!(client.port, Some(8443));
    let request = client.request("get").unwrap();
    assert_eq!(request.headers.len(), 1);
    assert_eq!(request.params.len(), 2);
    assert_eq!(request.asserts.len(), 1);
}

#[test]
fn test_native_client_definition_validation() {
    let cases = [
        (
            r#"client user { scheme: ftp, host: "example.com", requests: {} }"#,
            "unknown scheme 'ftp'",
        ),
        (
            r#"client user { scheme: https, host: "example.com", requests: { get: { path: "/" } } }"#,
            "request 'get' is missing 'method'",
        ),
        (
            r#"client user { scheme: https, host: "example.com", requests: { get: { path: "/", method: GET, headers: [["a"]] } } }"#,
            "each headers entry must contain exactly two values",
        ),
        (
            r#"client user { scheme: https, host: "a", requests: { get: { path: "/", method: GET } } } client user { scheme: https, host: "b", requests: { get: { path: "/", method: GET } } }"#,
            "duplicate client 'user'",
        ),
    ];
    for (input, expected) in cases {
        let error = match Parser::new(input).parse() {
            Ok(_) => panic!("expected client definition to fail"),
            Err(error) => error,
        };
        assert!(
            error.contains(expected),
            "{error} did not contain {expected}"
        );
    }
}

#[test]
fn test_parse_call_expr_argument() {
    let tests = vec![
        ("add();", "add", vec![]),
        ("add(1);", "add", vec!["1"]),
        ("add(1, 2 * 3, 4 + 5);", "add", vec!["1", "2 * 3", "4 + 5"]),
    ];
    for (text, function_name, expected) in tests {
        if let Ok(Source { exprs, .. }) = Parser::new(text).parse() {
            assert!(exprs.len() == 1);
            if let Some(expr) = exprs.first() {
                println!("{expr}");
                if let Expr::Call(function, arguments) = expr {
                    assert!(function.to_string() == function_name);
                    assert!(
                        arguments
                            .iter()
                            .map(|a| a.to_string())
                            .collect::<Vec<String>>()
                            == expected
                    );
                } else {
                    unreachable!("call expr parse failed")
                }
            } else {
                unreachable!("exprs expr none")
            }
        }
    }
}

#[test]
fn test_parse_loop_exprs() {
    let tests = vec![
        ("break;", "break"),
        ("break 5;", "break 5"),
        ("continue;", "continue"),
        ("loop { break 5 }", "loop { break 5 }"),
        ("while (true) { break 5 }", "while (true) { break 5 }"),
        ("for x in 1..3 { x }", "for x in 1..3 { x }"),
        (
            "for index, item in [1, 2] { item }",
            "for index, item in [1, 2] { item }",
        ),
        (
            "for key, value in {\"a\": 1} { value }",
            "for key, value in {\"a\": 1} { value }",
        ),
        (
            "for index, key, value in {\"a\": 1} { value }",
            "for index, key, value in {\"a\": 1} { value }",
        ),
    ];
    for (text, expected) in tests {
        let Source { exprs, .. } = Parser::new(text).parse().unwrap();
        assert_eq!(exprs.len(), 1);
        assert_eq!(exprs[0].to_string(), expected);
    }
}

#[test]
fn test_parse_range_exprs() {
    let tests = vec![
        ("1..2", "1..2"),
        ("1..", "1.."),
        ("..2", "..2"),
        ("1..=2", "1..=2"),
        ("..=2", "..=2"),
        ("1 + 1..4", "1 + 1..4"),
    ];
    for (text, expected) in tests {
        let Source { exprs, .. } = Parser::new(text).parse().unwrap();
        assert_eq!(exprs.len(), 1);
        assert_eq!(exprs[0].to_string(), expected);
    }
}

#[test]
fn test_parse_function_literal() {
    let tests = vec![
        (
            "fn add(x, y) { x + y }",
            "add",
            vec!["x", "y"],
            "x + y",
            "fn add(x, y) { x + y }",
        ),
        (
            "fn add_one(x) { x + 1;x }",
            "add_one",
            vec!["x"],
            "x + 1",
            "fn add_one(x) { x + 1;x }",
        ),
        (
            "fn addTwo() { 1 + 2;3 }",
            "addTwo",
            vec![],
            "1 + 2",
            "fn addTwo() { 1 + 2;3 }",
        ),
    ];
    for (text, expected_name, expected_parameters, expected_body, expected_display) in tests {
        match Parser::new(text).parse_function_literal() {
            Ok(Expr::Function(name, parameters, body)) => {
                assert!(name == expected_name);
                assert!(parameters == expected_parameters);
                assert!(body[0].to_string() == expected_body);
                assert_eq!(
                    Expr::Function(name, parameters, body).to_string(),
                    expected_display
                );
            }
            Ok(expr) => {
                unreachable!("function literal parsed as {expr:?}")
            }
            Err(error) => {
                unreachable!("{}", error)
            }
        }

        match Parser::new(text).parse() {
            Ok(Source { functions, .. }) => {
                if let Some((name, (parameters, body))) = functions.into_iter().next() {
                    assert!(name == expected_name);
                    assert!(parameters == expected_parameters);
                    assert!(body[0].to_string() == expected_body);
                } else {
                    unreachable!("function literal parse failed")
                }
            }
            Err(error) => {
                unreachable!("{}", error)
            }
        }
    }
}

#[test]
fn test_parse_function_parameter() {
    let tests = vec![
        ("fn zero() {};", vec![]),
        ("fn one(x) {};", vec!["x"]),
        ("fn three(x, y, z) {};", vec!["x", "y", "z"]),
    ];
    for (text, expected) in tests {
        match Parser::new(text).parse() {
            Ok(Source { functions, .. }) => {
                if let Some((_, (parameters, _))) = functions.into_iter().next() {
                    assert!(parameters == expected);
                } else {
                    unreachable!("function literal parse failed")
                }
            }
            Err(error) => {
                unreachable!("{}", error)
            }
        }
    }
}

#[test]
fn test_removed_request_syntax_errors() {
    assert!(
        Parser::new("rq get`GET http://example.com`")
            .parse()
            .is_err()
    );
    assert!(Parser::new("send->;").parse().is_err());
}

#[test]
fn test_parse_test_literal() {
    let tests = vec![
        (
            r#"
            test expectStatusOk {
                let response = user.getIp();
                response.status
            }"#,
            "expectStatusOk",
            2,
        ),
        (
            r#"
            test empty {
            }"#,
            "empty",
            0,
        ),
    ];
    for (text, expected_name, expected_length) in tests {
        match Parser::new(text).parse() {
            Ok(Source { tests, .. }) => {
                if let Some(block) = tests.get(expected_name) {
                    assert!(block.len() == expected_length);
                } else {
                    unreachable!("tests none")
                }
            }
            Err(error) => {
                unreachable!("{}", error)
            }
        }
    }
}
