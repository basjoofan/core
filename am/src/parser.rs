use crate::lexer;
use crate::syntax::Expr;
use crate::syntax::Source;
use crate::token::{Kind, Token, LOWEST, STMT, UNARY};
use core::f64;
use std::fmt::Write;

pub struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    pub fn new(text: &str) -> Parser {
        Parser {
            tokens: lexer::segment(text),
            index: usize::default(),
        }
    }

    fn next_token(&mut self) {
        self.index += 1;
    }

    fn current_token(&self) -> Option<&Token> {
        self.tokens.get(self.index)
    }

    fn peek_token(&self) -> Option<&Token> {
        self.tokens.get(self.index + 1)
    }

    fn peek_token_is(&self, kind: Kind) -> bool {
        matches!(self.peek_token(), Some(peek) if kind == peek.kind)
    }

    fn peek_token_expect(&mut self, kind: Kind) -> bool {
        if self.peek_token_is(kind) {
            self.next_token();
            true
        } else {
            // TODO catch peek expect error
            false
        }
    }

    fn current_precedence(&self) -> u8 {
        if let Some(current) = self.current_token() {
            current.precedence()
        } else {
            LOWEST
        }
    }

    fn peek_precedence(&self) -> u8 {
        if let Some(peek) = self.peek_token() {
            peek.precedence()
        } else {
            LOWEST
        }
    }

    pub fn parse(&mut self) -> Source {
        let mut expressions = Vec::new();
        let mut functions = Vec::new();
        let mut requests = Vec::new();
        while self.current_token().is_some() {
            if let Some(expression) = self.parse_expression(LOWEST) {
                match *expression {
                    Expr::Function(..) => functions.push(*expression),
                    Expr::Request(..) => requests.push(*expression),
                    _ => expressions.push(*expression),
                }
            }
            self.next_token();
        }
        Source {
            expressions,
            functions,
            requests,
        }
    }

    fn parse_expression(&mut self, mut precedence: u8) -> Option<Box<Expr>> {
        if let Some(current) = self.current_token() {
            let prefix = match current.kind {
                Kind::Ident => self.parse_ident_expression(),
                Kind::Integer => self.parse_integer_literal(),
                Kind::Float => self.parse_float_literal(),
                Kind::True | Kind::False => self.parse_boolean_literal(),
                Kind::String => self.parse_string_literal(),
                Kind::Let => {
                    precedence = STMT;
                    self.parse_let_expression()
                }
                Kind::Return => {
                    precedence = STMT;
                    self.parse_return_expression()
                }
                Kind::Bang | Kind::Minus => self.parse_unary_expression(),
                Kind::Lp => self.parse_paren_expression(),
                Kind::If => self.parse_if_expression(),
                Kind::Fn => self.parse_function_literal(),
                Kind::Ls => self.parse_array_literal(),
                Kind::Lb => self.parse_map_literal(),
                Kind::Rq => self.parse_request_literal(),
                Kind::Well => self.parse_annotation_literal(),
                _ => None,
            };
            if let Some(prefix) = prefix {
                let mut left = Some(prefix);
                while !self.peek_token_is(Kind::Semi) && precedence < self.peek_precedence() {
                    left = if let Some(peek) = self.peek_token() {
                        match peek.kind {
                            Kind::Plus
                            | Kind::Minus
                            | Kind::Star
                            | Kind::Slash
                            | Kind::Lt
                            | Kind::Gt
                            | Kind::Eq
                            | Kind::Ne => {
                                self.next_token();
                                self.parse_binary_expression(left)
                            }
                            Kind::Lp => {
                                self.next_token();
                                self.parse_call_expression(left)
                            }
                            Kind::Ls => {
                                self.next_token();
                                self.parse_index_expression(left)
                            }
                            Kind::Dot => {
                                self.next_token();
                                self.parse_field_expression(left)
                            }
                            _ => left,
                        }
                    } else {
                        left
                    };
                }
                left
            } else {
                // TODO catch grammar error
                None
            }
        } else {
            None
        }
    }

    fn parse_ident_expression(&self) -> Option<Box<Expr>> {
        if let Some(current) = self.current_token() {
            let token = current.clone();
            let value = current.literal.clone();
            Some(Box::new(Expr::Ident(token, value)))
        } else {
            None
        }
    }

    fn parse_current_string(&self) -> Option<String> {
        self.current_token().map(|token| token.literal.clone())
    }

    fn parse_integer_literal(&self) -> Option<Box<Expr>> {
        if let Some(current) = self.current_token() {
            let token = current.clone();
            let value = match current.literal.clone().parse::<i64>() {
                Ok(integer) => Some(integer),
                Err(_) => {
                    // TODO catch parse int error
                    None
                }
            };
            Some(Box::new(Expr::Integer(token, value)))
        } else {
            None
        }
    }

    fn parse_float_literal(&self) -> Option<Box<Expr>> {
        if let Some(current) = self.current_token() {
            let token = current.clone();
            let value = match current.literal.clone().parse::<f64>() {
                Ok(float) => Some(float),
                Err(_) => {
                    // TODO catch parse float error
                    Some(f64::NAN)
                }
            };
            Some(Box::new(Expr::Float(token, value)))
        } else {
            None
        }
    }

    fn parse_boolean_literal(&self) -> Option<Box<Expr>> {
        if let Some(current) = self.current_token() {
            let token = current.clone();
            let value = match current.literal.clone().parse::<bool>() {
                Ok(boolean) => Some(boolean),
                Err(_) => {
                    // TODO catch parse bool error
                    None
                }
            };
            Some(Box::new(Expr::Boolean(token, value)))
        } else {
            None
        }
    }

    fn parse_string_literal(&self) -> Option<Box<Expr>> {
        if let Some(current) = self.current_token() {
            let token = current.clone();
            let value = current.literal.clone();
            Some(Box::new(Expr::String(token, value)))
        } else {
            None
        }
    }

    fn parse_let_expression(&mut self) -> Option<Box<Expr>> {
        if let Some(current) = self.current_token() {
            let token = current.clone();
            if !self.peek_token_expect(Kind::Ident) {
                return None;
            }
            let name = self.parse_current_string();
            if !self.peek_token_expect(Kind::Assign) {
                return None;
            }
            self.next_token();
            let value = self.parse_expression(LOWEST);
            if self.peek_token_is(Kind::Semi) {
                self.next_token();
            }
            Some(Box::new(Expr::Let(token, name, value)))
        } else {
            None
        }
    }

    fn parse_return_expression(&mut self) -> Option<Box<Expr>> {
        if let Some(current) = self.current_token() {
            let token = current.clone();
            self.next_token();
            let value = self.parse_expression(LOWEST);
            if self.peek_token_is(Kind::Semi) {
                self.next_token();
            }
            Some(Box::new(Expr::Return(token, value)))
        } else {
            None
        }
    }

    fn parse_unary_expression(&mut self) -> Option<Box<Expr>> {
        if let Some(current) = self.current_token() {
            let token = current.clone();
            self.next_token();
            let right = self.parse_expression(UNARY);
            Some(Box::new(Expr::Unary(token, right)))
        } else {
            None
        }
    }

    fn parse_binary_expression(&mut self, left: Option<Box<Expr>>) -> Option<Box<Expr>> {
        if let Some(current) = self.current_token() {
            let token = current.clone();
            let precedence = self.current_precedence();
            self.next_token();
            let right = self.parse_expression(precedence);
            Some(Box::new(Expr::Binary(token, left, right)))
        } else {
            None
        }
    }

    fn parse_paren_expression(&mut self) -> Option<Box<Expr>> {
        if let Some(current) = self.current_token() {
            let token = current.clone();
            self.next_token();
            let expression = self.parse_expression(LOWEST);
            if !self.peek_token_expect(Kind::Rp) {
                None
            } else {
                Some(Box::new(Expr::Paren(token, expression)))
            }
        } else {
            None
        }
    }

    fn parse_if_expression(&mut self) -> Option<Box<Expr>> {
        if let Some(current) = self.current_token() {
            let token = current.clone();
            if !self.peek_token_expect(Kind::Lp) {
                return None;
            }
            self.next_token();
            let condition = self.parse_expression(LOWEST);
            if !self.peek_token_expect(Kind::Rp) {
                return None;
            }
            if !self.peek_token_expect(Kind::Lb) {
                return None;
            }
            let consequence = self.parse_block_expression();
            let mut alternative = Vec::new();
            if self.peek_token_is(Kind::Else) {
                self.next_token();
                if !self.peek_token_expect(Kind::Lb) {
                    return None;
                }
                alternative = self.parse_block_expression();
            }
            Some(Box::new(Expr::If(token, condition, consequence, alternative)))
        } else {
            None
        }
    }

    fn parse_function_literal(&mut self) -> Option<Box<Expr>> {
        if let Some(current) = self.current_token() {
            let token = current.clone();
            let mut name = None;
            if self.peek_token_is(Kind::Ident) {
                self.next_token();
                name = self.parse_current_string();
            }
            if !self.peek_token_expect(Kind::Lp) {
                return None;
            }
            let parameters = self.parse_ident_list(Kind::Rp);
            if !self.peek_token_expect(Kind::Lb) {
                return None;
            }
            let body = self.parse_block_expression();
            Some(Box::new(Expr::Function(token, None, name, parameters, body)))
        } else {
            None
        }
    }

    fn parse_ident_list(&mut self, end: Kind) -> Vec<String> {
        let mut idents = Vec::new();
        if self.peek_token_is(end.clone()) {
            self.next_token();
        } else {
            self.next_token();
            if let Some(ident) = self.parse_current_string() {
                idents.push(ident);
            }
            while self.peek_token_is(Kind::Comma) {
                self.next_token();
                self.next_token();
                if let Some(ident) = self.parse_current_string() {
                    idents.push(ident);
                }
            }
            self.peek_token_expect(end);
        }
        idents
    }

    fn parse_call_expression(&mut self, function: Option<Box<Expr>>) -> Option<Box<Expr>> {
        if let Some(current) = self.current_token() {
            let token = current.clone();
            let arguments = self.parse_expression_list(Kind::Rp);
            Some(Box::new(Expr::Call(token, function, arguments)))
        } else {
            None
        }
    }

    fn parse_expression_list(&mut self, end: Kind) -> Vec<Expr> {
        let mut expressions = Vec::new();
        if self.peek_token_is(end.clone()) {
            self.next_token();
        } else {
            self.next_token();
            if let Some(expression) = self.parse_expression(LOWEST) {
                expressions.push(*expression);
            }
            while self.peek_token_is(Kind::Comma) {
                self.next_token();
                self.next_token();
                if let Some(expression) = self.parse_expression(LOWEST) {
                    expressions.push(*expression);
                }
            }
            self.peek_token_expect(end);
        }
        expressions
    }

    fn parse_array_literal(&mut self) -> Option<Box<Expr>> {
        if let Some(current) = self.current_token() {
            let token = current.clone();
            let elements = self.parse_expression_list(Kind::Rs);
            Some(Box::new(Expr::Array(token, elements)))
        } else {
            None
        }
    }

    fn parse_map_literal(&mut self) -> Option<Box<Expr>> {
        if let Some(current) = self.current_token() {
            let token = current.clone();
            let mut pairs = Vec::new();
            while !self.peek_token_is(Kind::Rb) {
                self.next_token();
                let key = self.parse_expression(LOWEST);
                if !self.peek_token_expect(Kind::Colon) {
                    return None;
                }
                self.next_token();
                let value = self.parse_expression(LOWEST);
                if let (Some(key), Some(value)) = (key, value) {
                    pairs.push((*key, *value));
                }
                if !self.peek_token_is(Kind::Rb) && !self.peek_token_expect(Kind::Comma) {
                    return None;
                }
            }
            if !self.peek_token_expect(Kind::Rb) {
                return None;
            }
            Some(Box::new(Expr::Map(token, pairs)))
        } else {
            None
        }
    }

    fn parse_index_expression(&mut self, left: Option<Box<Expr>>) -> Option<Box<Expr>> {
        if let Some(current) = self.current_token() {
            let token = current.clone();
            self.next_token();
            let index = self.parse_expression(LOWEST);
            if !self.peek_token_expect(Kind::Rs) {
                None
            } else {
                Some(Box::new(Expr::Index(token, left, index)))
            }
        } else {
            None
        }
    }

    fn parse_field_expression(&mut self, object: Option<Box<Expr>>) -> Option<Box<Expr>> {
        if let Some(current) = self.current_token() {
            let token = current.clone();
            self.next_token();
            let field = self.parse_current_string();
            Some(Box::new(Expr::Field(token, object, field)))
        } else {
            None
        }
    }

    fn parse_request_literal(&mut self) -> Option<Box<Expr>> {
        if let Some(current) = self.current_token() {
            let token = current.clone();
            if !self.peek_token_expect(Kind::Ident) {
                return None;
            }
            let name = self.parse_current_string().unwrap();
            if !self.peek_token_expect(Kind::Template) {
                return None;
            }
            let message = self.parse_current_string().unwrap();
            let pieces = divide_template_pieces(message.trim().lines().fold(String::new(), |mut string, str| {
                let _ = writeln!(string, "{}", str.trim());
                string
            }));
            let mut asserts = Vec::new();
            if self.peek_token_is(Kind::Ls) {
                self.next_token();
                asserts = self.parse_expression_list(Kind::Rs);
            }
            Some(Box::new(Expr::Request(token, None, name, pieces, asserts)))
        } else {
            None
        }
    }

    fn parse_annotation_literal(&mut self) -> Option<Box<Expr>> {
        if self.current_token().is_some() {
            if !self.peek_token_expect(Kind::Ls) {
                return None;
            }
            let tags = self.parse_ident_list(Kind::Rs);
            if self.peek_token_is(Kind::Fn) || self.peek_token_is(Kind::Rq) {
                self.next_token();
                if let Some(call) = self.parse_expression(LOWEST) {
                    match *call {
                        Expr::Function(token, _, name, parameters, body) => {
                            Some(Box::new(Expr::Function(token, Some(tags), name, parameters, body)))
                        }
                        Expr::Request(token, _, name, pieces, asserts) => {
                            Some(Box::new(Expr::Request(token, Some(tags), name, pieces, asserts)))
                        }
                        _ => None,
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    fn parse_block_expression(&mut self) -> Vec<Expr> {
        let mut expressions = Vec::new();
        if let Some(Token {
            kind: Kind::Lb,
            literal: _,
        }) = self.current_token()
        {
            self.next_token();
            while let Some(current) = self.current_token() {
                if current.kind == Kind::Rb {
                    break;
                }
                if let Some(expression) = self.parse_expression(LOWEST) {
                    expressions.push(*expression);
                }
                self.next_token();
            }
        }
        expressions
    }
}

fn divide_template_pieces(message: String) -> Vec<Expr> {
    let mut expressions = Vec::new();
    let mut chars = message.chars().peekable();
    while let Some(char) = chars.next() {
        match (char, chars.peek()) {
            ('$', Some('{')) => {
                let mut closed = false;
                let mut string = String::new();
                chars.next();
                for char in chars.by_ref() {
                    if char == '}' {
                        closed = true;
                        break;
                    } else {
                        string.push(char);
                    }
                }
                if closed {
                    if let Some(e) = Parser::new(&string).parse_expression(LOWEST) {
                        expressions.push(*e);
                    }
                } else {
                    string.insert_str(0, "${");
                    expressions.push(Expr::String(
                        Token {
                            kind: Kind::String,
                            literal: string.clone(),
                        },
                        string,
                    ));
                }
            }
            _ => {
                let mut string = String::from(char);
                while let Some(peek) = chars.peek() {
                    if *peek == '$' {
                        break;
                    } else {
                        string.push(chars.next().unwrap());
                    }
                }
                expressions.push(Expr::String(
                    Token {
                        kind: Kind::String,
                        literal: string.clone(),
                    },
                    string,
                ));
            }
        }
    }
    expressions
}

#[test]
fn test_let_expression() {
    let tests = vec![
        ("let x =  5;", 1, "let x 5"),
        ("let y  = true;", 1, "let y true"),
        ("let  foobar = y;", 1, "let foobar y"),
        ("let i = 0; [1][i];", 2, "let i 0"),
    ];
    for (text, len, expected) in tests {
        let source = Parser::new(text).parse();
        assert!(source.expressions.len() == len);
        if let Some(expression) = source.expressions.first() {
            println!("{}", expression);
            if let Expr::Let(token, Some(name), Some(value)) = expression.clone() {
                let parsed = format!("{} {} {}", token, name, value);
                assert!(expected == parsed);
            } else {
                unreachable!("let expression parse failed")
            }
        } else {
            unreachable!("source expression none")
        }
    }
}

#[test]
fn test_return_expression() {
    let tests = vec![
        ("return 5;", 1, "5"),
        ("return true;", 1, "true"),
        ("return foobar;", 1, "foobar"),
        ("return foobar; [1][i];", 2, "foobar"),
    ];
    for (text, len, expected) in tests {
        let source = Parser::new(text).parse();
        println!("source:{}", source);
        assert!(source.expressions.len() == len);
        if let Some(expression) = source.expressions.first() {
            println!("{}", expression);
            if let Expr::Return(_, Some(value)) = expression.clone() {
                assert!(expected == value.to_string());
            } else {
                unreachable!("return expression parse failed")
            }
        } else {
            unreachable!("source expression none")
        }
    }
}

#[test]
fn test_ident_expression() {
    let text = "foobar;";
    let source = Parser::new(text).parse();
    assert!(source.expressions.len() == 1);
    if let Some(expression) = source.expressions.first() {
        println!("{}", expression);
        if let Expr::Ident(_, value) = expression {
            assert!(value == "foobar");
        } else {
            unreachable!("ident expression parse failed")
        }
    } else {
        unreachable!("source expression none")
    }
}

#[test]
fn test_integer_literal() {
    let text = "5;";
    let source = Parser::new(text).parse();
    assert!(source.expressions.len() == 1);
    if let Some(expression) = source.expressions.first() {
        println!("{}", expression);
        if let Expr::Integer(_, Some(value)) = *expression {
            assert!(value == 5);
        } else {
            unreachable!("integer literal parse failed")
        }
    } else {
        unreachable!("source expression none")
    }
}

#[test]
fn test_float_literal() {
    let text = "3.14159265358979323846264338327950288;";
    let source = Parser::new(text).parse();
    assert!(source.expressions.len() == 1);
    if let Some(expression) = source.expressions.first() {
        println!("expression:{}", expression);
        if let Expr::Float(_, Some(value)) = *expression {
            println!("value:{}", value);
            assert!(value == f64::consts::PI);
        } else {
            unreachable!("float literal parse failed")
        }
    } else {
        unreachable!("source expression none")
    }
}

#[test]
fn test_boolean_literal() {
    let tests = vec![("true;", true), ("false;", false)];
    for (text, expected) in tests {
        let source = Parser::new(text).parse();
        assert!(source.expressions.len() == 1);
        if let Some(expression) = source.expressions.first() {
            println!("{}", expression);
            if let Expr::Boolean(_, Some(value)) = *expression {
                assert!(value == expected);
            } else {
                unreachable!("boolean literal parse failed")
            }
        } else {
            unreachable!("source expression none")
        }
    }
}

#[test]
fn test_string_literal() {
    let text = r#""hello world";"#;
    let source = Parser::new(text).parse();
    assert!(source.expressions.len() == 1);
    if let Some(expression) = source.expressions.first() {
        println!("{}", expression);
        if let Expr::String(_, value) = expression {
            assert!(value == "hello world");
        } else {
            unreachable!("string literal parse failed")
        }
    } else {
        unreachable!("source expression none")
    }
}

#[test]
fn test_unary_expression() {
    let tests = vec![
        ("!5;", "!", "5"),
        ("-15;", "-", "15"),
        ("!foobar;", "!", "foobar"),
        ("-foobar;", "-", "foobar"),
        ("!true;", "!", "true"),
        ("!false;", "!", "false"),
    ];
    for (text, expected_operator, expected_right) in tests {
        let source = Parser::new(text).parse();
        assert!(source.expressions.len() == 1);
        if let Some(expression) = source.expressions.first() {
            println!("{}", expression);
            if let Expr::Unary(token, Some(right)) = expression {
                assert!(expected_operator == token.to_string());
                assert!(expected_right == right.to_string());
            } else {
                unreachable!("unary expression parse failed")
            }
        } else {
            unreachable!("source expression none")
        }
    }
}

#[test]
fn test_binary_expression() {
    let tests = vec![
        ("5 + 5;", "5", "+", "5"),
        ("5 - 5;", "5", "-", "5"),
        ("5 * 5;", "5", "*", "5"),
        ("5 / 5;", "5", "/", "5"),
        ("5 > 5;", "5", ">", "5"),
        ("5 < 5;", "5", "<", "5"),
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
    ];
    for (text, expected_left, expected_operator, expected_right) in tests {
        let source = Parser::new(text).parse();
        assert!(source.expressions.len() == 1);
        if let Some(expression) = source.expressions.first() {
            println!("{}", expression);
            if let Expr::Binary(token, Some(left), Some(right)) = expression {
                assert!(expected_left == left.to_string());
                assert!(expected_operator == token.to_string());
                assert!(expected_right == right.to_string());
            } else {
                unreachable!("binary expression parse failed")
            }
        } else {
            unreachable!("source expression none")
        }
    }
}

#[test]
fn test_operator_precedence() {
    let tests = vec![
        ("-a * b", "((-a) * b)"),
        ("!-a", "(!(-a))"),
        ("a + b + c", "((a + b) + c)"),
        ("a + b - c", "((a + b) - c)"),
        ("a * b * c", "((a * b) * c)"),
        ("a * b / c", "((a * b) / c)"),
        ("a + b / c", "(a + (b / c))"),
        ("a + b * c + d / e - f", "(((a + (b * c)) + (d / e)) - f)"),
        ("3 + 4; -5 * 5", "(3 + 4)((-5) * 5)"),
        ("5 > 4 == 3 < 4", "((5 > 4) == (3 < 4))"),
        ("5 < 4 != 3 > 4", "((5 < 4) != (3 > 4))"),
        ("3 + 4 * 5 == 3 * 1 + 4 * 5", "((3 + (4 * 5)) == ((3 * 1) + (4 * 5)))"),
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
        ("add(a + b + c * d / f + g)", "add((((a + b) + ((c * d) / f)) + g))"),
        ("a * [1, 2, 3, 4][b * c] * d", "((a * ([1, 2, 3, 4][(b * c)])) * d)"),
        (
            "add(a * b[2], b[1], 2 * [1, 2][1])",
            "add((a * (b[2])), (b[1]), (2 * ([1, 2][1])))",
        ),
    ];
    for (text, expected) in tests {
        let source = Parser::new(text).parse();
        println!("{}=={}", source, expected);
        assert!(source.to_string() == expected);
    }
}

#[test]
fn test_if_expression() {
    let text = "if (x < y) { x }";
    let source = Parser::new(text).parse();
    assert!(source.expressions.len() == 1);
    if let Some(expression) = source.expressions.first() {
        println!("{}", expression);
        if let Expr::If(_, Some(condition), consequence, alternative) = expression {
            assert!(condition.to_string() == "(x < y)");
            assert!(consequence[0].to_string() == "x");
            assert!(alternative.is_empty())
        } else {
            unreachable!("if expression parse failed")
        }
    } else {
        unreachable!("source expression none")
    }
}

#[test]
fn test_if_else_expression() {
    let text = "if (x < y) { x } else { y }";
    let source = Parser::new(text).parse();
    assert!(source.expressions.len() == 1);
    if let Some(expression) = source.expressions.first() {
        println!("{}", expression);
        if let Expr::If(_, Some(condition), consequence, alternative) = expression {
            assert!(condition.to_string() == "(x < y)");
            assert!(consequence[0].to_string() == "x");
            assert!(alternative[0].to_string() == "y")
        } else {
            unreachable!("if expression parse failed")
        }
    } else {
        unreachable!("source expression none")
    }
}

#[test]
fn test_function_literal() {
    let tests = vec![
        ("fn(x, y) { x + y }", None, vec!["x", "y"], "(x + y)"),
        (
            "fn add(x, y) { x + y }",
            Some(String::from("add")),
            vec!["x", "y"],
            "(x + y)",
        ),
    ];
    for (text, expected_name, expected_parameters, expected_body) in tests {
        let source = Parser::new(text).parse();
        println!("source({}):{}", source.functions.len(), source);
        assert!(source.functions.len() == 1);
        if let Some(function) = source.functions.first() {
            println!("{}", function);
            if let Expr::Function(_, _, name, parameters, body) = function.clone() {
                assert!(name == expected_name);
                assert!(parameters == expected_parameters);
                assert!(body[0].to_string() == expected_body);
            } else {
                unreachable!("function literal parse failed")
            }
        } else {
            unreachable!("source expression none")
        }
    }
}

#[test]
fn test_function_parameter() {
    let tests = vec![
        ("fn() {};", vec![]),
        ("fn(x) {};", vec!["x"]),
        ("fn(x, y, z) {};", vec!["x", "y", "z"]),
        ("fn no() {};", vec![]),
        ("fn one(x) {};", vec!["x"]),
        ("fn multiple(x, y, z) {};", vec!["x", "y", "z"]),
    ];
    for (text, expected) in tests {
        let source = Parser::new(text).parse();
        assert!(source.functions.len() == 1);
        if let Some(function) = source.functions.first() {
            println!("{}", function);
            if let Expr::Function(_, _, _, parameters, _) = function.clone() {
                assert!(parameters == expected);
            } else {
                unreachable!("function literal parse failed")
            }
        } else {
            unreachable!("source expression none")
        }
    }
}

#[test]
fn test_call_expression() {
    let text = "add(1, 2 * 3, 4 + 5);";
    let source = Parser::new(text).parse();
    assert!(source.expressions.len() == 1);
    if let Some(expression) = source.expressions.first() {
        println!("{}", expression);
        if let Expr::Call(_, Some(function), arguments) = expression {
            assert!(function.to_string() == "add");
            assert!(arguments[0].to_string() == "1");
            assert!(arguments[1].to_string() == "(2 * 3)");
            assert!(arguments[2].to_string() == "(4 + 5)");
        } else {
            unreachable!("call expression parse failed")
        }
    } else {
        unreachable!("source expression none")
    }
}

#[test]
fn test_call_expression_argument() {
    let tests = vec![
        ("add();", "add", vec![]),
        ("add(1);", "add", vec!["1"]),
        ("add(1, 2 * 3, 4 + 5);", "add", vec!["1", "(2 * 3)", "(4 + 5)"]),
    ];
    for (text, function_name, expected) in tests {
        let source = Parser::new(text).parse();
        assert!(source.expressions.len() == 1);
        if let Some(expression) = source.expressions.first() {
            println!("{}", expression);
            if let Expr::Call(_, Some(function), arguments) = expression {
                assert!(function.to_string() == function_name);
                assert!(arguments.iter().map(|a| a.to_string()).collect::<Vec<String>>() == expected);
            } else {
                unreachable!("call expression parse failed")
            }
        } else {
            unreachable!("source expression none")
        }
    }
}

#[test]
fn test_array_literal_empty() {
    let text = "[]";
    let source = Parser::new(text).parse();
    assert!(source.expressions.len() == 1);
    if let Some(expression) = source.expressions.first() {
        println!("{}", expression);
        if let Expr::Array(_, elements) = expression {
            assert!(elements.is_empty());
        } else {
            unreachable!("array literal parse failed")
        }
    } else {
        unreachable!("source expression none")
    }
}

#[test]
fn test_array_literal() {
    let text = "[1, 2 * 2, 3 + 3]";
    let source = Parser::new(text).parse();
    assert!(source.expressions.len() == 1);
    if let Some(expression) = source.expressions.first() {
        println!("{}", expression);
        if let Expr::Array(_, elements) = expression {
            assert!(elements.len() == 3);
            assert!(elements[0].to_string() == "1");
            assert!(elements[1].to_string() == "(2 * 2)");
            assert!(elements[2].to_string() == "(3 + 3)");
        } else {
            unreachable!("array literal parse failed")
        }
    } else {
        unreachable!("source expression none")
    }
}

#[test]
fn test_index_expression() {
    let text = "myArray[1 + 1]";
    let source = Parser::new(text).parse();
    assert!(source.expressions.len() == 1);
    if let Some(expression) = source.expressions.first() {
        println!("{}", expression);
        if let Expr::Index(_, Some(left), Some(index)) = expression {
            assert!(left.to_string() == "myArray");
            assert!(index.to_string() == "(1 + 1)");
        } else {
            unreachable!("index expression parse failed")
        }
    } else {
        unreachable!("source expression none")
    }
}

#[test]
fn test_field_expression() {
    let text = "object.field";
    let source = Parser::new(text).parse();
    assert!(source.expressions.len() == 1);
    if let Some(expression) = source.expressions.first() {
        println!("{}", expression);
        if let Expr::Field(_, Some(object), Some(field)) = expression {
            assert!(object.to_string() == "object");
            assert!(*field == "field");
        } else {
            unreachable!("field expression parse failed")
        }
    } else {
        unreachable!("source expression none")
    }
}

#[test]
fn test_map_literal_empty() {
    let text = "{}";
    let source = Parser::new(text).parse();
    assert!(source.expressions.len() == 1);
    if let Some(expression) = source.expressions.first() {
        println!("{}", expression);
        if let Expr::Map(_, pairs) = expression {
            assert!(pairs.is_empty());
        } else {
            unreachable!("map literal parse failed")
        }
    } else {
        unreachable!("source expression none")
    }
}

#[test]
fn test_map_literal_string_key() {
    let text = r#"{"one": 1, "two": 2, "three": 3}"#;
    let expected = vec![
        (String::from("\"one\""), String::from("1")),
        (String::from("\"two\""), String::from("2")),
        (String::from("\"three\""), String::from("3")),
    ];
    let source = Parser::new(text).parse();
    assert!(source.expressions.len() == 1);
    if let Some(expression) = source.expressions.first() {
        println!("{}", expression);
        if let Expr::Map(_, pairs) = expression {
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
        unreachable!("source expression none")
    }
}

#[test]
fn test_map_literal_boolean_key() {
    let text = r#"{true: 1, false: 2}"#;
    let expected = vec![
        (String::from("true"), String::from("1")),
        (String::from("false"), String::from("2")),
    ];
    let source = Parser::new(text).parse();
    assert!(source.expressions.len() == 1);
    if let Some(expression) = source.expressions.first() {
        println!("{}", expression);
        if let Expr::Map(_, pairs) = expression {
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
        unreachable!("source expression none")
    }
}

#[test]
fn test_map_literal_integer_key() {
    let text = r#"{1: 1, 2: 2, 3: 3}"#;
    let expected = vec![
        (String::from("1"), String::from("1")),
        (String::from("2"), String::from("2")),
        (String::from("3"), String::from("3")),
    ];
    let source = Parser::new(text).parse();
    assert!(source.expressions.len() == 1);
    if let Some(expression) = source.expressions.first() {
        println!("{}", expression);
        if let Expr::Map(_, pairs) = expression {
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
        unreachable!("source expression none")
    }
}

#[test]
fn test_map_literal_with_expression() {
    let text = r#"{"one": 0 + 1, "two": 10 - 8, "three": 15 / 5}"#;
    let expected = vec![
        (String::from("\"one\""), String::from("(0 + 1)")),
        (String::from("\"two\""), String::from("(10 - 8)")),
        (String::from("\"three\""), String::from("(15 / 5)")),
    ];
    let source = Parser::new(text).parse();
    assert!(source.expressions.len() == 1);
    if let Some(expression) = source.expressions.first() {
        println!("{}", expression);
        if let Expr::Map(_, pairs) = expression {
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
        unreachable!("source expression none")
    }
}

#[test]
fn test_template_pieces() {
    let tests = vec![
        (
            "s${a}xdefe${b}efe${c}x$xx",
            vec![r#""s""#, "a", r#""xdefe""#, "b", r#""efe""#, "c", r#""x""#, r#""$xx""#],
        ),
        (
            "s${a}xdefe${b}efe${cxxx",
            vec![r#""s""#, "a", r#""xdefe""#, "b", r#""efe""#, r#""${cxxx""#],
        ),
        (
            "s${a + b}xdefe${b*c+d}efe${c()}",
            vec![r#""s""#, "(a + b)", r#""xdefe""#, "((b * c) + d)", r#""efe""#, "c()"],
        ),
    ];
    for (message, expected_pieces) in tests {
        let pieces = divide_template_pieces(message.to_string());
        pieces.iter().for_each(|piece| println!("{}", piece));
        assert!(pieces.iter().map(|piece| piece.to_string()).collect::<Vec<String>>() == expected_pieces);
    }
}

#[test]
fn test_request_literal() {
    let tests = vec![
        (
            "rq request`\nGET http://${host}/api\nHost: example.com\n`",
            1,
            "request",
            vec!["\"GET http://\"", "host", "\"/api\nHost: example.com\n\""],
        ),
        ("rq request`POST`", 1, "request", vec!["\"POST\n\""]),
    ];
    for (text, expected_len, expected_name, expected_pieces) in tests {
        let source = Parser::new(text).parse();
        assert!(source.requests.len() == expected_len);
        if let Some(request) = source.requests.first() {
            println!("request:{}", request);
            if let Expr::Request(_, _, name, pieces, asserts) = request.clone() {
                assert!(name == expected_name);
                assert!(pieces.iter().map(|piece| piece.to_string()).collect::<Vec<String>>() == expected_pieces);
                assert!(asserts.is_empty());
            } else {
                unreachable!("request literal parse failed")
            }
        } else {
            unreachable!("source expression none")
        }
    }
}

#[test]
fn test_request_asserts() {
    let tests = vec![
        (
            r#"rq request`\nGET http://${host}/api\nHost: example.com\n`[
               status == 200,
               regex(text, "^\d{4}-\d{2}-\d{2}$") == "2022-02-22"
               ]"#,
            2,
            vec![
                "(status == 200)",
                r#"(regex(text, "^\d{4}-\d{2}-\d{2}$") == "2022-02-22")"#,
            ],
        ),
        (r#"rq request`POST`[]"#, 0, vec![]),
    ];
    for (text, expected_len, expected_asserts) in tests {
        let source = Parser::new(text).parse();
        if let Some(request) = source.requests.first() {
            println!("request:{}", request);
            if let Expr::Request(_, _, _, _, asserts) = request.clone() {
                assert!(asserts.len() == expected_len);
                assert!(asserts.iter().map(|assert| assert.to_string()).collect::<Vec<String>>() == expected_asserts);
            } else {
                unreachable!("request literal parse failed")
            }
        } else {
            unreachable!("source expression none")
        }
    }
}

#[test]
fn test_annotation_literal() {
    let tests = vec![
        (
            r#"
            #[test]
            rq request`
                GET http://${host}/api\nHost: example.com\n`[
                status == 200]"#,
            1,
            Some(vec!["test".to_string()]),
        ),
        (r#"#[]rq request`POST`[]"#, 0, Some(vec![])),
        (
            r#"
            #[test]
            fn add(x, y) { 
                x + y 
            }
            "#,
            1,
            Some(vec!["test".to_string()]),
        ),
        (r#"fn add(x, y) { x + y }"#, 0, None),
        (r#"#[test]fn add(x, y) { x + y }"#, 1, Some(vec!["test".to_string()])),
        (
            r#"#[test, tag]fn add(x, y) { x + y }"#,
            2,
            Some(vec!["test".to_string(), "tag".to_string()]),
        ),
    ];
    for (text, expected_len, expected_tags) in tests {
        let source = Parser::new(text).parse();
        let mut calls = Vec::new();
        calls.extend_from_slice(&source.requests);
        calls.extend_from_slice(&source.functions);
        if let Some(call) = calls.first() {
            println!("call:\n{}", call);
            if let Expr::Request(_, tags, _, _, _) = call.clone() {
                assert!(tags.clone().unwrap_or_default().len() == expected_len);
                assert!(tags == expected_tags);
            } else if let Expr::Function(_, tags, _, _, _) = call.clone() {
                assert!(tags.clone().unwrap_or_default().len() == expected_len);
                assert!(tags == expected_tags);
            } else {
                unreachable!("annotation literal parse failed")
            }
        } else {
            unreachable!("source expression none")
        }
    }
}
