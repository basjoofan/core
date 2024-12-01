use crate::lexer;
use crate::Expr;
use crate::Kind;
use crate::Token;

pub struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    pub fn new(text: &str) -> Parser {
        Parser {
            tokens: lexer::segment(text),
            index: usize::MIN,
        }
    }

    fn next_token(&mut self) {
        let index = self.index + 1;
        (index < self.tokens.len()).then(|| self.index = index);
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
            u8::MIN
        }
    }

    pub fn parse(&mut self) -> Result<Vec<Expr>, String> {
        let mut exprs = Vec::new();
        while self.current_token().kind != Kind::Eof {
            exprs.push(self.parse_expr(u8::MIN)?);
            if self.peek_token_is(Kind::Semi) {
                self.next_token();
            }
            self.next_token();
        }
        Ok(exprs)
    }

    fn parse_expr(&mut self, mut precedence: u8) -> Result<Expr, String> {
        let mut left = match self.current_token().kind {
            Kind::Ident => self.parse_ident_expr(),
            Kind::Integer => self.parse_integer_literal()?,
            Kind::Float => self.parse_float_literal()?,
            Kind::True | Kind::False => self.parse_boolean_literal()?,
            Kind::String => self.parse_string_literal(),
            Kind::Let => {
                precedence = u8::MAX;
                self.parse_let_expr()?
            }
            Kind::Return => {
                precedence = u8::MAX;
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
        while !self.peek_token_is(Kind::Semi) && precedence < self.peek_precedence() {
            left = match self.peek_token() {
                Some(Token { kind: Kind::Plus, .. })
                | Some(Token { kind: Kind::Minus, .. })
                | Some(Token { kind: Kind::Star, .. })
                | Some(Token { kind: Kind::Slash, .. })
                | Some(Token { kind: Kind::Percent, .. })
                | Some(Token { kind: Kind::Lt, .. })
                | Some(Token { kind: Kind::Gt, .. })
                | Some(Token { kind: Kind::Le, .. })
                | Some(Token { kind: Kind::Ge, .. })
                | Some(Token { kind: Kind::Eq, .. })
                | Some(Token { kind: Kind::Ne, .. })
                | Some(Token { kind: Kind::Bx, .. })
                | Some(Token { kind: Kind::Ba, .. })
                | Some(Token { kind: Kind::Bo, .. })
                | Some(Token { kind: Kind::La, .. })
                | Some(Token { kind: Kind::Lo, .. }) => {
                    self.next_token();
                    self.parse_binary_expr(left)?
                }
                Some(Token { kind: Kind::Lp, .. }) => {
                    self.next_token();
                    self.parse_call_expr(left)?
                }
                Some(Token { kind: Kind::Ls, .. }) => {
                    self.next_token();
                    self.parse_index_expr(left)?
                }
                Some(Token { kind: Kind::Dot, .. }) => {
                    self.next_token();
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
        self.current_token().literal.clone()
    }

    fn parse_integer_literal(&self) -> Result<Expr, String> {
        let token = self.current_token();
        match token.literal.parse::<i64>() {
            Ok(integer) => Ok(Expr::Integer(integer)),
            Err(_) => Err(format!("parse integer error: {}", token)),
        }
    }

    fn parse_float_literal(&self) -> Result<Expr, String> {
        let token = self.current_token();
        match token.literal.parse::<f64>() {
            Ok(float) => Ok(Expr::Float(float)),
            Err(_) => Err(format!("parse float error: {}", token)),
        }
    }

    fn parse_boolean_literal(&self) -> Result<Expr, String> {
        let token = self.current_token();
        match token.literal.parse::<bool>() {
            Ok(boolean) => Ok(Expr::Boolean(boolean)),
            Err(_) => Err(format!("parse boolean error: {}", token)),
        }
    }

    fn parse_string_literal(&self) -> Expr {
        Expr::String(self.parse_current_string())
    }

    fn parse_let_expr(&mut self) -> Result<Expr, String> {
        self.peek_token_expect(Kind::Ident)?;
        let name = self.parse_current_string();
        self.peek_token_expect(Kind::Assign)?;
        self.next_token();
        let value = match self.parse_expr(u8::MIN)? {
            Expr::Function(_, parameters, body) => Expr::Function(Some(name.clone()), parameters, body),
            expr => expr,
        };
        if self.peek_token_is(Kind::Semi) {
            self.next_token();
        }
        Ok(Expr::Let(name, Box::new(value)))
    }

    fn parse_return_expr(&mut self) -> Result<Expr, String> {
        self.next_token();
        let value = self.parse_expr(u8::MIN)?;
        if self.peek_token_is(Kind::Semi) {
            self.next_token();
        }
        Ok(Expr::Return(Box::new(value)))
    }

    fn parse_unary_expr(&mut self) -> Result<Expr, String> {
        let token = self.current_token().clone();
        let mut precedence = self.current_precedence();
        (token.kind == Kind::Minus).then(|| {
            precedence += 2;
        });
        self.next_token();
        let right = self.parse_expr(precedence)?;
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
        self.next_token();
        let expr = self.parse_expr(u8::MIN)?;
        self.peek_token_expect(Kind::Rp)?;
        Ok(Expr::Paren(Box::new(expr)))
    }

    fn parse_if_expr(&mut self) -> Result<Expr, String> {
        self.peek_token_expect(Kind::Lp)?;
        self.next_token();
        let condition = self.parse_expr(u8::MIN)?;
        self.peek_token_expect(Kind::Rp)?;
        let consequence = self.parse_block_expr()?;
        let mut alternative = Vec::new();
        if self.peek_token_is(Kind::Else) {
            self.next_token();
            alternative = self.parse_block_expr()?;
        }
        Ok(Expr::If(Box::new(condition), consequence, alternative))
    }

    fn parse_function_literal(&mut self) -> Result<Expr, String> {
        self.peek_token_expect(Kind::Lp)?;
        let parameters = self.parse_ident_list(Kind::Rp)?;
        let body = self.parse_block_expr()?;
        Ok(Expr::Function(None, parameters, body))
    }

    fn parse_ident_list(&mut self, end: Kind) -> Result<Vec<String>, String> {
        let mut idents = Vec::new();
        while !self.peek_token_is(end) {
            self.peek_token_expect(Kind::Ident)?;
            idents.push(self.parse_current_string());
            if !self.peek_token_is(end) {
                self.peek_token_expect(Kind::Comma)?;
            }
        }
        self.peek_token_expect(end)?;
        Ok(idents)
    }

    fn parse_call_expr(&mut self, function: Expr) -> Result<Expr, String> {
        let arguments = self.parse_expr_list(Kind::Rp)?;
        Ok(Expr::Call(Box::new(function), arguments))
    }

    fn parse_expr_list(&mut self, end: Kind) -> Result<Vec<Expr>, String> {
        let mut exprs = Vec::new();
        while !self.peek_token_is(end) {
            self.next_token();
            exprs.push(self.parse_expr(u8::MIN)?);
            if !self.peek_token_is(end) {
                self.peek_token_expect(Kind::Comma)?;
            }
        }
        self.peek_token_expect(end)?;
        Ok(exprs)
    }

    fn parse_array_literal(&mut self) -> Result<Expr, String> {
        let elements = self.parse_expr_list(Kind::Rs)?;
        Ok(Expr::Array(elements))
    }

    fn parse_map_literal(&mut self) -> Result<Expr, String> {
        let mut pairs = Vec::new();
        while !self.peek_token_is(Kind::Rb) {
            self.next_token();
            let key = self.parse_expr(u8::MIN)?;
            self.peek_token_expect(Kind::Colon)?;
            self.next_token();
            let value = self.parse_expr(u8::MIN)?;
            pairs.push((key, value));
            if !self.peek_token_is(Kind::Rb) {
                self.peek_token_expect(Kind::Comma)?;
            }
        }
        self.peek_token_expect(Kind::Rb)?;
        Ok(Expr::Map(pairs))
    }

    fn parse_index_expr(&mut self, left: Expr) -> Result<Expr, String> {
        self.next_token();
        let index = self.parse_expr(u8::MIN)?;
        self.peek_token_expect(Kind::Rs)?;
        Ok(Expr::Index(Box::new(left), Box::new(index)))
    }

    fn parse_field_expr(&mut self, object: Expr) -> Result<Expr, String> {
        self.peek_token_expect(Kind::Ident)?;
        let field = self.parse_current_string();
        Ok(Expr::Field(Box::new(object), field))
    }

    fn parse_request_literal(&mut self) -> Result<Expr, String> {
        self.peek_token_expect(Kind::Ident)?;
        let name = self.parse_current_string();
        self.peek_token_expect(Kind::Lp)?;
        let parameters = self.parse_ident_list(Kind::Rp)?;
        self.peek_token_expect(Kind::Template)?;
        let message = self.parse_current_string();
        let mut asserts = Vec::new();
        if self.peek_token_is(Kind::Ls) {
            self.next_token();
            asserts = self.parse_expr_list(Kind::Rs)?;
        }
        Ok(Expr::Request(name, parameters, message, asserts))
    }

    fn parse_test_literal(&mut self) -> Result<Expr, String> {
        self.peek_token_expect(Kind::Ident)?;
        let name = self.parse_current_string();
        let block = self.parse_block_expr()?;
        Ok(Expr::Test(name, block))
    }

    fn parse_block_expr(&mut self) -> Result<Vec<Expr>, String> {
        let mut exprs = Vec::new();
        self.peek_token_expect(Kind::Lb)?;
        while !self.peek_token_is(Kind::Rb) {
            self.next_token();
            exprs.push(self.parse_expr(u8::MIN)?);
            if self.peek_token_is(Kind::Semi) {
                self.next_token();
            }
        }
        self.peek_token_expect(Kind::Rb)?;
        Ok(exprs)
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
            Ok(source) => {
                println!("{}", source.len());
                println!("{:?}", source);
                assert!(source.len() == len);
                if let Some(expr) = source.first() {
                    println!("{}", expr);
                    if let Expr::Let(name, value) = expr {
                        let parsed = format!("let {} {}", name, value);
                        println!("{}={}", expected, parsed);
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
                if let Expr::Return(value) = expr {
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
            if let Expr::Ident(value) = expr {
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
            if let Expr::Integer(value) = *expr {
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
            if let Expr::Float(value) = *expr {
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
                if let Expr::Boolean(value) = *expr {
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
            if let Expr::String(value) = expr {
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
        ("~5;", "~", "5"),
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
            Ok(source) => {
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
            Err(error) => {
                unreachable!("source expr error:{}", error);
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
        ("!add()", "(!add())"),
        ("-add()", "(-add())"),
        ("!array[1]", "(!(array[1]))"),
        ("-object.field", "(-object.field)"),
        ("3 > 2 && 2 > 1", "((3 > 2) && (2 > 1))"),
        ("a || b * c", "(a || (b * c))"),
        ("a && b < c", "(a && (b < c))"),
        ("b + c || a", "((b + c) || a)"),
        ("b < c & a", "((b < c) & a)"),
    ];
    for (text, expected) in tests {
        match Parser::new(text).parse() {
            Ok(source) => {
                let actual: String = source.iter().map(|e| e.to_string()).collect::<String>();
                println!("{}=={}", actual, expected);
                assert_eq!(actual, expected);
            }
            Err(error) => {
                unreachable!("source expr error:{}", error);
            }
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
            if let Expr::If(condition, consequence, alternative) = expr {
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
            if let Expr::If(condition, consequence, alternative) = expr {
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
            if let Expr::Call(function, arguments) = expr {
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
                if let Expr::Call(function, arguments) = expr {
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
            if let Expr::Array(elements) = expr {
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
            if let Expr::Array(elements) = expr {
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
            if let Expr::Index(left, index) = expr {
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
            if let Expr::Field(object, field) = expr {
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
            if let Expr::Map(pairs) = expr {
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
fn test_parse_map_literal_one_element() {
    let text = "{1: true}";
    if let Ok(source) = Parser::new(text).parse() {
        assert!(source.len() == 1);
        if let Some(expr) = source.first() {
            println!("{}", expr);
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
            unreachable!("source expr none")
        }
    }
}

#[test]
fn test_parse_request_literal() {
    let tests = vec![
        (
            "rq request(host)`\nGET http://{host}/api\nHost: example.com\n`",
            1,
            "request",
            vec!["host"],
            "\nGET http://{host}/api\nHost: example.com\n",
        ),
        ("rq request()`POST`", 1, "request", vec![], "POST"),
    ];
    for (text, expected_len, expected_name, expected_parameters, expected_message) in tests {
        if let Ok(source) = Parser::new(text).parse() {
            assert!(source.len() == expected_len);
            if let Some(request) = source.first() {
                println!("request:{}", request);
                if let Expr::Request(name, parameters, message, asserts) = request.clone() {
                    assert!(name == expected_name);
                    assert!(message == expected_message);
                    assert!(parameters == expected_parameters);
                    assert!(asserts.is_empty());
                } else {
                    unreachable!("request literal parse failed")
                }
            } else {
                unreachable!("source expr none")
            }
        } else {
            unreachable!("source expr error")
        }
    }
}

#[test]
fn test_parse_request_asserts() {
    let tests = vec![
        (
            r#"rq request()`\nGET http://${host}/api\nHost: example.com\n`[
               status == 200,
               regex(text, "^\d{4}-\d{2}-\d{2}$") == "2022-02-22"
               ]"#,
            2,
            vec![
                "(status == 200)",
                r#"(regex(text, "^\d{4}-\d{2}-\d{2}$") == "2022-02-22")"#,
            ],
        ),
        (r#"rq request()`POST`[]"#, 0, vec![]),
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
        } else {
            unreachable!("source expr error")
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
            if let Some(Expr::Test(name, block)) = source.first() {
                assert!(name == expected_name);
                assert!(block.len() == expected_length);
            } else {
                unreachable!("source expr none")
            }
        }
    }
}
