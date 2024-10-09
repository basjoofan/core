use crate::lexer;
use crate::syntax::Expr;
use crate::token::{Kind, Token, LOWEST, STMT, UNARY};

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

    fn current_token(&self) -> &Token {
        &self.tokens[self.index]
    }

    fn peek_token(&self) -> Option<&Token> {
        self.tokens.get(self.index + 1)
    }

    fn peek_token_is(&self, kind: Kind) -> bool {
        matches!(self.peek_token(), Some(peek) if kind == peek.kind)
    }

    fn peek_token_expect(&mut self, kind: Kind) -> Result<(), String> {
        if let Some(peek) = self.peek_token() {
            if kind == peek.kind {
                self.next_token();
                Ok(())
            } else {
                Err(format!("token expect {:?} but found {:?}", kind, peek.kind))
            }
        } else {
            Err(format!("token expect {:?} but found none", kind))
        }
    }

    fn current_precedence(&self) -> u8 {
        self.current_token().precedence()
    }

    fn peek_precedence(&self) -> u8 {
        if let Some(peek) = self.peek_token() {
            peek.precedence()
        } else {
            LOWEST
        }
    }

    pub fn parse(&mut self) -> Result<Vec<Expr>, String> {
        let mut exprs = Vec::new();
        while self.index < self.tokens.len() {
            exprs.push(self.parse_expr(LOWEST)?);
            if self.peek_token_is(Kind::Semi) {
                self.next_token();
            }
            self.next_token();
        }
        Ok(exprs)
    }

    fn parse_expr(&mut self, mut precedence: u8) -> Result<Expr, String> {
        let prefix = match self.current_token().kind {
            Kind::Ident => self.parse_ident_expr(),
            Kind::Integer => self.parse_integer_literal()?,
            Kind::Float => self.parse_float_literal()?,
            Kind::True | Kind::False => self.parse_boolean_literal()?,
            Kind::String => self.parse_string_literal(),
            Kind::Let => {
                precedence = STMT;
                self.parse_let_expr()?
            }
            Kind::Return => {
                precedence = STMT;
                self.parse_return_expr()?
            }
            Kind::Bang | Kind::Minus => self.parse_unary_expr()?,
            Kind::Lp => self.parse_paren_expr()?,
            Kind::If => self.parse_if_expr()?,
            Kind::Fn => self.parse_function_literal()?,
            Kind::Ls => self.parse_array_literal()?,
            Kind::Lb => self.parse_map_literal()?,
            Kind::Rq => self.parse_request_literal()?,
            Kind::Test => self.parse_test_literal()?,
            _ => Err(format!("parse expr error: {}", self.current_token()))?,
        };
        let mut left = prefix;
        while !self.peek_token_is(Kind::Semi) && precedence < self.peek_precedence() {
            left = if let Some(peek) = self.peek_token() {
                match peek.kind {
                    Kind::Plus | Kind::Minus | Kind::Star | Kind::Slash | Kind::Lt | Kind::Gt | Kind::Eq | Kind::Ne => {
                        self.next_token();
                        self.parse_binary_expr(left)?
                    }
                    Kind::Lp => {
                        self.next_token();
                        self.parse_call_expr(left)?
                    }
                    Kind::Ls => {
                        self.next_token();
                        self.parse_index_expr(left)?
                    }
                    Kind::Dot => {
                        self.next_token();
                        self.parse_field_expr(left)?
                    }
                    _ => left,
                }
            } else {
                left
            };
        }
        Ok(left)
    }

    fn parse_ident_expr(&self) -> Expr {
        let token = self.current_token().clone();
        let value = token.literal.clone();
        Expr::Ident(token, value)
    }

    fn parse_current_string(&self) -> String {
        self.current_token().literal.clone()
    }

    fn parse_integer_literal(&self) -> Result<Expr, String> {
        let token = self.current_token().clone();
        match token.literal.parse::<i64>() {
            Ok(integer) => Ok(Expr::Integer(token, integer)),
            Err(_) => Err(format!("parse integer error: {}", token)),
        }
    }

    fn parse_float_literal(&self) -> Result<Expr, String> {
        let token = self.current_token().clone();
        match token.literal.parse::<f64>() {
            Ok(float) => Ok(Expr::Float(token, float)),
            Err(_) => Err(format!("parse float error: {}", token)),
        }
    }

    fn parse_boolean_literal(&self) -> Result<Expr, String> {
        let token = self.current_token().clone();
        match token.literal.parse::<bool>() {
            Ok(boolean) => Ok(Expr::Boolean(token, boolean)),
            Err(_) => Err(format!("parse boolean error: {}", token)),
        }
    }

    fn parse_string_literal(&self) -> Expr {
        let token = self.current_token().clone();
        let string = token.literal.clone();
        Expr::String(token, string)
    }

    fn parse_let_expr(&mut self) -> Result<Expr, String> {
        let token = self.current_token().clone();
        self.peek_token_expect(Kind::Ident)?;
        let name = self.parse_current_string();
        self.peek_token_expect(Kind::Assign)?;
        self.next_token();
        let value = self.parse_expr(LOWEST)?;
        if self.peek_token_is(Kind::Semi) {
            self.next_token();
        }
        Ok(Expr::Let(token, name, Box::new(value)))
    }

    fn parse_return_expr(&mut self) -> Result<Expr, String> {
        let token = self.current_token().clone();
        self.next_token();
        let value = self.parse_expr(LOWEST)?;
        if self.peek_token_is(Kind::Semi) {
            self.next_token();
        }
        Ok(Expr::Return(token, Box::new(value)))
    }

    fn parse_unary_expr(&mut self) -> Result<Expr, String> {
        let token = self.current_token().clone();
        self.next_token();
        let right = self.parse_expr(UNARY)?;
        Ok(Expr::Unary(token, Box::new(right)))
    }

    fn parse_binary_expr(&mut self, left: Expr) -> Result<Expr, String> {
        let token = self.current_token().clone();
        let precedence = self.current_precedence();
        self.next_token();
        let right = self.parse_expr(precedence)?;
        Ok(Expr::Binary(token, Box::new(left), Box::new(right)))
    }

    fn parse_paren_expr(&mut self) -> Result<Expr, String> {
        let token = self.current_token().clone();
        self.next_token();
        let expr = self.parse_expr(LOWEST)?;
        self.peek_token_expect(Kind::Rp)?;
        Ok(Expr::Paren(token, Box::new(expr)))
    }

    fn parse_if_expr(&mut self) -> Result<Expr, String> {
        let token = self.current_token().clone();
        self.peek_token_expect(Kind::Lp)?;
        self.next_token();
        let condition = self.parse_expr(LOWEST)?;
        self.peek_token_expect(Kind::Rp)?;
        let consequence = self.parse_block_expr()?;
        let mut alternative = Vec::new();
        if self.peek_token_is(Kind::Else) {
            self.next_token();
            alternative = self.parse_block_expr()?;
        }
        Ok(Expr::If(token, Box::new(condition), consequence, alternative))
    }

    fn parse_function_literal(&mut self) -> Result<Expr, String> {
        let token = self.current_token().clone();
        self.peek_token_expect(Kind::Lp)?;
        let parameters = self.parse_ident_list(Kind::Rp)?;
        let body = self.parse_block_expr()?;
        Ok(Expr::Function(token, parameters, body))
    }

    fn parse_ident_list(&mut self, end: Kind) -> Result<Vec<String>, String> {
        let mut idents = Vec::new();
        if self.peek_token_is(end.clone()) {
            self.next_token();
        } else {
            self.peek_token_expect(Kind::Ident)?;
            idents.push(self.parse_current_string());
            while self.peek_token_is(Kind::Comma) {
                self.next_token();
                self.peek_token_expect(Kind::Ident)?;
                idents.push(self.parse_current_string());
            }
            self.peek_token_expect(end)?;
        }
        Ok(idents)
    }

    fn parse_call_expr(&mut self, function: Expr) -> Result<Expr, String> {
        let token = self.current_token().clone();
        let arguments = self.parse_expr_list(Kind::Rp)?;
        Ok(Expr::Call(token, Box::new(function), arguments))
    }

    fn parse_expr_list(&mut self, end: Kind) -> Result<Vec<Expr>, String> {
        let mut exprs = Vec::new();
        if self.peek_token_is(end.clone()) {
            self.next_token();
        } else {
            self.next_token();
            exprs.push(self.parse_expr(LOWEST)?);
            while self.peek_token_is(Kind::Comma) {
                self.next_token();
                self.next_token();
                exprs.push(self.parse_expr(LOWEST)?);
            }
            self.peek_token_expect(end)?;
        }
        Ok(exprs)
    }

    fn parse_array_literal(&mut self) -> Result<Expr, String> {
        let token = self.current_token().clone();
        let elements = self.parse_expr_list(Kind::Rs)?;
        Ok(Expr::Array(token, elements))
    }

    fn parse_map_literal(&mut self) -> Result<Expr, String> {
        let token = self.current_token().clone();
        let mut pairs = Vec::new();
        while !self.peek_token_is(Kind::Rb) {
            self.next_token();
            let key = self.parse_expr(LOWEST)?;
            self.peek_token_expect(Kind::Colon)?;
            self.next_token();
            let value = self.parse_expr(LOWEST)?;
            pairs.push((key, value));
            if !self.peek_token_is(Kind::Rb) {
                self.peek_token_expect(Kind::Comma)?;
            }
        }
        self.peek_token_expect(Kind::Rb)?;
        Ok(Expr::Map(token, pairs))
    }

    fn parse_index_expr(&mut self, left: Expr) -> Result<Expr, String> {
        let token = self.current_token().clone();
        self.next_token();
        let index = self.parse_expr(LOWEST)?;
        self.peek_token_expect(Kind::Rs)?;
        Ok(Expr::Index(token, Box::new(left), Box::new(index)))
    }

    fn parse_field_expr(&mut self, object: Expr) -> Result<Expr, String> {
        let token = self.current_token().clone();
        self.peek_token_expect(Kind::Ident)?;
        let field = self.parse_current_string();
        Ok(Expr::Field(token, Box::new(object), field))
    }

    fn parse_request_literal(&mut self) -> Result<Expr, String> {
        let token = self.current_token().clone();
        self.peek_token_expect(Kind::Ident)?;
        let name = self.parse_current_string();
        self.peek_token_expect(Kind::Template)?;
        let message = self.parse_current_string();
        let pieces = divide_template_pieces(message.trim().lines().fold(String::new(), |mut string, str| {
            string.push_str(str.trim());
            string.push_str("\n");
            string
        }))?;
        let mut asserts = Vec::new();
        if self.peek_token_is(Kind::Ls) {
            self.next_token();
            asserts = self.parse_expr_list(Kind::Rs)?;
        }
        Ok(Expr::Request(token, name, pieces, asserts))
    }

    fn parse_test_literal(&mut self) -> Result<Expr, String> {
        let token = self.current_token().clone();
        self.peek_token_expect(Kind::Ident)?;
        let name = self.parse_current_string();
        let block = self.parse_block_expr()?;
        Ok(Expr::Test(token, name, block))
    }

    fn parse_block_expr(&mut self) -> Result<Vec<Expr>, String> {
        let mut exprs = Vec::new();
        self.peek_token_expect(Kind::Lb)?;
        while !self.peek_token_is(Kind::Rb) {
            self.next_token();
            exprs.push(self.parse_expr(LOWEST)?);
            if self.peek_token_is(Kind::Semi) {
                self.next_token();
            }
        }
        self.peek_token_expect(Kind::Rb)?;
        Ok(exprs)
    }
}

fn divide_template_pieces(message: String) -> Result<Vec<Expr>, String> {
    let mut exprs = Vec::new();
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
                    exprs.push(Parser::new(&string).parse_expr(LOWEST)?);
                } else {
                    string.insert_str(0, "${");
                    exprs.push(Expr::String(
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
                exprs.push(Expr::String(
                    Token {
                        kind: Kind::String,
                        literal: string.clone(),
                    },
                    string,
                ));
            }
        }
    }
    Ok(exprs)
}

#[test]
fn test_parse_let_expr() {
    let tests = vec![
        ("let x =  5;", 1, "let x 5"),
        ("let y  = true;", 1, "let y true"),
        ("let  foobar = y;", 1, "let foobar y"),
        ("let i = 0; [1][i];", 2, "let i 0"),
    ];
    for (text, len, expected) in tests {
        match Parser::new(text).parse() {
            Ok(source) => {
                println!("{}", source.len());
                assert!(source.len() == len);
                if let Some(expr) = source.first() {
                    println!("{}", expr);
                    if let Expr::Let(token, name, value) = expr {
                        let parsed = format!("{} {} {}", token, name, value);
                        assert!(expected == parsed);
                    } else {
                        unreachable!("let expr parse failed")
                    }
                } else {
                    unreachable!("source expr none")
                }
            }
            Err(message) => {
                unreachable!("{}", message)
            }
        }
    }
}

#[test]
fn test_parse_return_expr() {
    let tests = vec![
        ("return 5;", 1, "5"),
        ("return true;", 1, "true"),
        ("return foobar;", 1, "foobar"),
        ("return foobar; [1][i];", 2, "foobar"),
    ];
    for (text, len, expected) in tests {
        if let Ok(source) = Parser::new(text).parse() {
            assert!(source.len() == len);
            if let Some(expr) = source.first() {
                println!("{}", expr);
                if let Expr::Return(_, value) = expr {
                    assert!(expected == value.to_string());
                } else {
                    unreachable!("return expr parse failed")
                }
            } else {
                unreachable!("source expr none")
            }
        }
    }
}

#[test]
fn test_parse_ident_expr() {
    let text = "foobar;";
    if let Ok(source) = Parser::new(text).parse() {
        assert!(source.len() == 1);
        if let Some(expr) = source.first() {
            println!("{}", expr);
            if let Expr::Ident(_, value) = expr {
                assert!(value == "foobar");
            } else {
                unreachable!("ident expr parse failed")
            }
        } else {
            unreachable!("source expr none")
        }
    }
}

#[test]
fn test_parse_integer_literal() {
    let text = "5;";
    if let Ok(source) = Parser::new(text).parse() {
        assert!(source.len() == 1);
        if let Some(expr) = source.first() {
            println!("{}", expr);
            if let Expr::Integer(_, value) = *expr {
                assert!(value == 5);
            } else {
                unreachable!("integer literal parse failed")
            }
        } else {
            unreachable!("source expr none")
        }
    }
}

#[test]
fn test_parse_float_literal() {
    let text = "3.14159265358979323846264338327950288;";
    if let Ok(source) = Parser::new(text).parse() {
        assert!(source.len() == 1);
        if let Some(expr) = source.first() {
            println!("expr:{}", expr);
            if let Expr::Float(_, value) = *expr {
                println!("value:{}", value);
                assert!(value == core::f64::consts::PI);
            } else {
                unreachable!("float literal parse failed")
            }
        } else {
            unreachable!("source expr none")
        }
    }
}

#[test]
fn test_parse_boolean_literal() {
    let tests = vec![("true;", true), ("false;", false)];
    for (text, expected) in tests {
        if let Ok(source) = Parser::new(text).parse() {
            assert!(source.len() == 1);
            if let Some(expr) = source.first() {
                println!("{}", expr);
                if let Expr::Boolean(_, value) = *expr {
                    assert!(value == expected);
                } else {
                    unreachable!("boolean literal parse failed")
                }
            } else {
                unreachable!("source expr none")
            }
        }
    }
}

#[test]
fn test_parse_string_literal() {
    let text = r#""hello world";"#;
    if let Ok(source) = Parser::new(text).parse() {
        assert!(source.len() == 1);
        if let Some(expr) = source.first() {
            println!("{}", expr);
            if let Expr::String(_, value) = expr {
                assert!(value == "hello world");
            } else {
                unreachable!("string literal parse failed")
            }
        } else {
            unreachable!("source expr none")
        }
    }
}

#[test]
fn test_parse_unary_expr() {
    let tests = vec![
        ("!5;", "!", "5"),
        ("-15;", "-", "15"),
        ("!foobar;", "!", "foobar"),
        ("-foobar;", "-", "foobar"),
        ("!true;", "!", "true"),
        ("!false;", "!", "false"),
    ];
    for (text, expected_operator, expected_right) in tests {
        if let Ok(source) = Parser::new(text).parse() {
            assert!(source.len() == 1);
            if let Some(expr) = source.first() {
                println!("{}", expr);
                if let Expr::Unary(token, right) = expr {
                    assert!(expected_operator == token.to_string());
                    assert!(expected_right == right.to_string());
                } else {
                    unreachable!("unary expr parse failed")
                }
            } else {
                unreachable!("source expr none")
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
        if let Ok(source) = Parser::new(text).parse() {
            assert!(source.len() == 1);
            if let Some(expr) = source.first() {
                println!("{}", expr);
                if let Expr::Binary(token, left, right) = expr {
                    assert!(expected_left == left.to_string());
                    assert!(expected_operator == token.to_string());
                    assert!(expected_right == right.to_string());
                } else {
                    unreachable!("binary expr parse failed")
                }
            } else {
                unreachable!("source expr none")
            }
        }
    }
}

#[test]
fn test_parse_operator_precedence() {
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
        if let Ok(source) = Parser::new(text).parse() {
            println!(
                "{}:{:?}",
                source[0],
                source.iter().map(|e| e.to_string() + "|").collect::<String>()
            );
            let actual: String = source.iter().map(|e| e.to_string()).collect::<String>();
            println!("{}=={}", actual, expected);
            assert!(actual == expected);
        }
    }
}

#[test]
fn test_parse_if_expr() {
    let text = "if (x < y) { x }";
    if let Ok(source) = Parser::new(text).parse() {
        assert!(source.len() == 1);
        if let Some(expr) = source.first() {
            println!("{}", expr);
            if let Expr::If(_, condition, consequence, alternative) = expr {
                assert!(condition.to_string() == "(x < y)");
                assert!(consequence[0].to_string() == "x");
                assert!(alternative.is_empty())
            } else {
                unreachable!("if expr parse failed")
            }
        } else {
            unreachable!("source expr none")
        }
    }
}

#[test]
fn test_parse_if_else_expr() {
    let text = "if (x < y) { z;x } else { y }";
    if let Ok(source) = Parser::new(text).parse() {
        assert!(source.len() == 1);
        if let Some(expr) = source.first() {
            println!("{}", expr);
            if let Expr::If(_, condition, consequence, alternative) = expr {
                assert!(condition.to_string() == "(x < y)");
                assert!(consequence[0].to_string() == "z");
                assert!(consequence[1].to_string() == "x");
                assert!(alternative[0].to_string() == "y")
            } else {
                unreachable!("if expr parse failed")
            }
        } else {
            unreachable!("source expr none")
        }
    }
}

#[test]
fn test_parse_function_literal() {
    let tests = vec![
        ("fn(x, y) { x + y }", vec!["x", "y"], "(x + y)"),
        ("fn(x) { x + 1;x }", vec!["x"], "(x + 1)"),
        ("fn() { 1 + 2;3 }", vec![], "(1 + 2)"),
    ];
    for (text, expected_parameters, expected_body) in tests {
        if let Ok(source) = Parser::new(text).parse() {
            if let Some(function) = source.first() {
                println!("{}", function);
                if let Expr::Function(_, parameters, body) = function {
                    assert!(parameters == &expected_parameters);
                    assert!(body[0].to_string() == expected_body);
                } else {
                    unreachable!("function literal parse failed")
                }
            } else {
                unreachable!("source expr none")
            }
        }
    }
}

#[test]
fn test_parse_function_parameter() {
    let tests = vec![
        ("fn() {};", vec![]),
        ("fn(x) {};", vec!["x"]),
        ("fn(x, y, z) {};", vec!["x", "y", "z"]),
    ];
    for (text, expected) in tests {
        if let Ok(source) = Parser::new(text).parse() {
            if let Some(function) = source.first() {
                println!("{}", function);
                if let Expr::Function(_, parameters, _) = function {
                    assert!(parameters == &expected);
                } else {
                    unreachable!("function literal parse failed")
                }
            } else {
                unreachable!("source expr none")
            }
        }
    }
}

#[test]
fn test_parse_call_expr() {
    let text = "add(1, 2 * 3, 4 + 5);";
    if let Ok(source) = Parser::new(text).parse() {
        assert!(source.len() == 1);
        if let Some(expr) = source.first() {
            println!("{}", expr);
            if let Expr::Call(_, function, arguments) = expr {
                assert!(function.to_string() == "add");
                assert!(arguments[0].to_string() == "1");
                assert!(arguments[1].to_string() == "(2 * 3)");
                assert!(arguments[2].to_string() == "(4 + 5)");
            } else {
                unreachable!("call expr parse failed")
            }
        } else {
            unreachable!("source expr none")
        }
    }
}

#[test]
fn test_parse_call_expr_argument() {
    let tests = vec![
        ("add();", "add", vec![]),
        ("add(1);", "add", vec!["1"]),
        ("add(1, 2 * 3, 4 + 5);", "add", vec!["1", "(2 * 3)", "(4 + 5)"]),
    ];
    for (text, function_name, expected) in tests {
        if let Ok(source) = Parser::new(text).parse() {
            assert!(source.len() == 1);
            if let Some(expr) = source.first() {
                println!("{}", expr);
                if let Expr::Call(_, function, arguments) = expr {
                    assert!(function.to_string() == function_name);
                    assert!(arguments.iter().map(|a| a.to_string()).collect::<Vec<String>>() == expected);
                } else {
                    unreachable!("call expr parse failed")
                }
            } else {
                unreachable!("source expr none")
            }
        }
    }
}

#[test]
fn test_parse_array_literal_empty() {
    let text = "[]";
    if let Ok(source) = Parser::new(text).parse() {
        assert!(source.len() == 1);
        if let Some(expr) = source.first() {
            println!("{}", expr);
            if let Expr::Array(_, elements) = expr {
                assert!(elements.is_empty());
            } else {
                unreachable!("array literal parse failed")
            }
        } else {
            unreachable!("source expr none")
        }
    }
}

#[test]
fn test_parse_array_literal() {
    let text = "[1, 2 * 2, 3 + 3]";
    if let Ok(source) = Parser::new(text).parse() {
        assert!(source.len() == 1);
        if let Some(expr) = source.first() {
            println!("{}", expr);
            if let Expr::Array(_, elements) = expr {
                assert!(elements.len() == 3);
                assert!(elements[0].to_string() == "1");
                assert!(elements[1].to_string() == "(2 * 2)");
                assert!(elements[2].to_string() == "(3 + 3)");
            } else {
                unreachable!("array literal parse failed")
            }
        } else {
            unreachable!("source expr none")
        }
    }
}

#[test]
fn test_parse_index_expr() {
    let text = "myArray[1 + 1]";
    if let Ok(source) = Parser::new(text).parse() {
        assert!(source.len() == 1);
        if let Some(expr) = source.first() {
            println!("{}", expr);
            if let Expr::Index(_, left, index) = expr {
                assert!(left.to_string() == "myArray");
                assert!(index.to_string() == "(1 + 1)");
            } else {
                unreachable!("index expr parse failed")
            }
        } else {
            unreachable!("source expr none")
        }
    }
}

#[test]
fn test_parse_field_expr() {
    let text = "object.field";
    if let Ok(source) = Parser::new(text).parse() {
        assert!(source.len() == 1);
        if let Some(expr) = source.first() {
            println!("{}", expr);
            if let Expr::Field(_, object, field) = expr {
                assert!(object.to_string() == "object");
                assert!(*field == "field");
            } else {
                unreachable!("field expr parse failed")
            }
        } else {
            unreachable!("source expr none")
        }
    }
}

#[test]
fn test_parse_map_literal_empty() {
    let text = "{}";
    if let Ok(source) = Parser::new(text).parse() {
        assert!(source.len() == 1);
        if let Some(expr) = source.first() {
            println!("{}", expr);
            if let Expr::Map(_, pairs) = expr {
                assert!(pairs.is_empty());
            } else {
                unreachable!("map literal parse failed")
            }
        } else {
            unreachable!("source expr none")
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
    if let Ok(source) = Parser::new(text).parse() {
        assert!(source.len() == 1);
        if let Some(expr) = source.first() {
            println!("{}", expr);
            if let Expr::Map(_, pairs) = expr {
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
            unreachable!("source expr none")
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
    if let Ok(source) = Parser::new(text).parse() {
        assert!(source.len() == 1);
        if let Some(expr) = source.first() {
            println!("{}", expr);
            if let Expr::Map(_, pairs) = expr {
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
            unreachable!("source expr none")
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
    if let Ok(source) = Parser::new(text).parse() {
        assert!(source.len() == 1);
        if let Some(expr) = source.first() {
            println!("{}", expr);
            if let Expr::Map(_, pairs) = expr {
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
            unreachable!("source expr none")
        }
    }
}

#[test]
fn test_parse_map_literal_with_expr() {
    let text = r#"{"one": 0 + 1, "two": 10 - 8, "three": 15 / 5}"#;
    let expected = vec![
        (String::from("\"one\""), String::from("(0 + 1)")),
        (String::from("\"two\""), String::from("(10 - 8)")),
        (String::from("\"three\""), String::from("(15 / 5)")),
    ];
    if let Ok(source) = Parser::new(text).parse() {
        assert!(source.len() == 1);
        if let Some(expr) = source.first() {
            println!("{}", expr);
            if let Expr::Map(_, pairs) = expr {
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
            unreachable!("source expr none")
        }
    }
}

#[test]
fn test_parse_template_pieces() {
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
        if let Ok(pieces) = divide_template_pieces(message.to_string()) {
            pieces.iter().for_each(|piece| println!("{}", piece));
            assert!(pieces.iter().map(|piece| piece.to_string()).collect::<Vec<String>>() == expected_pieces);
        }
    }
}

#[test]
fn test_parse_request_literal() {
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
        if let Ok(source) = Parser::new(text).parse() {
            assert!(source.len() == expected_len);
            if let Some(request) = source.first() {
                println!("request:{}", request);
                if let Expr::Request(_, name, pieces, asserts) = request.clone() {
                    assert!(name == expected_name);
                    pieces.iter().for_each(|piece| println!("{}", piece));
                    assert!(pieces.iter().map(|piece| piece.to_string()).collect::<Vec<String>>() == expected_pieces);
                    assert!(asserts.is_empty());
                } else {
                    unreachable!("request literal parse failed")
                }
            } else {
                unreachable!("source expr none")
            }
        }
    }
}

#[test]
fn test_parse_request_asserts() {
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
        if let Ok(source) = Parser::new(text).parse() {
            if let Some(request) = source.first() {
                println!("request:{}", request);
                if let Expr::Request(_, _, _, asserts) = request.clone() {
                    assert!(asserts.len() == expected_len);
                    assert!(
                        asserts.iter().map(|assert| assert.to_string()).collect::<Vec<String>>() == expected_asserts
                    );
                } else {
                    unreachable!("request literal parse failed")
                }
            } else {
                unreachable!("source expr none")
            }
        }
    }
}

#[test]
fn test_parse_test_literal() {
    let tests = vec![
        (
            r#"
            test expectStatusOk {
                let response = request();
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
        if let Ok(source) = Parser::new(text).parse() {
            if let Some(Expr::Test(_, name, block)) = source.first() {
                assert!(name == expected_name);
                assert!(block.len() == expected_length);
            } else {
                unreachable!("source expr none")
            }
        }
    }
}
