use super::Kind;
use super::Token;

pub fn segment(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut scanner = Scanner::new(text);
    while let Some(char) = scanner.next() {
        if !char.is_whitespace() {
            let (line, column) = scanner.previous_location();
            let start = scanner.previous_index();
            let (kind, content_start, content_end) = match char {
                '=' => {
                    if scanner.peek() == Some('=') {
                        scanner.next();
                        (Kind::Eq, start, scanner.byte_index())
                    } else {
                        (Kind::Assign, start, scanner.byte_index())
                    }
                }
                '!' => {
                    if scanner.peek() == Some('=') {
                        scanner.next();
                        (Kind::Ne, start, scanner.byte_index())
                    } else {
                        (Kind::Not, start, scanner.byte_index())
                    }
                }
                '+' => (Kind::Add, start, scanner.byte_index()),
                '-' => (Kind::Sub, start, scanner.byte_index()),
                '*' => (Kind::Mul, start, scanner.byte_index()),
                '/' => (Kind::Div, start, scanner.byte_index()),
                '%' => (Kind::Rem, start, scanner.byte_index()),
                '^' => (Kind::Bx, start, scanner.byte_index()),
                '|' => {
                    if scanner.peek() == Some('|') {
                        scanner.next();
                        (Kind::Lo, start, scanner.byte_index())
                    } else {
                        (Kind::Bo, start, scanner.byte_index())
                    }
                }
                '&' => {
                    if scanner.peek() == Some('&') {
                        scanner.next();
                        (Kind::La, start, scanner.byte_index())
                    } else {
                        (Kind::Ba, start, scanner.byte_index())
                    }
                }
                '<' => {
                    if scanner.peek() == Some('=') {
                        scanner.next();
                        (Kind::Le, start, scanner.byte_index())
                    } else if scanner.peek() == Some('<') {
                        scanner.next();
                        (Kind::Sl, start, scanner.byte_index())
                    } else {
                        (Kind::Lt, start, scanner.byte_index())
                    }
                }
                '>' => {
                    if scanner.peek() == Some('=') {
                        scanner.next();
                        (Kind::Ge, start, scanner.byte_index())
                    } else if scanner.peek() == Some('>') {
                        scanner.next();
                        (Kind::Sr, start, scanner.byte_index())
                    } else {
                        (Kind::Gt, start, scanner.byte_index())
                    }
                }
                ',' => (Kind::Comma, start, scanner.byte_index()),
                ';' => (Kind::Semi, start, scanner.byte_index()),
                ':' => (Kind::Colon, start, scanner.byte_index()),
                '.' => {
                    if scanner.peek() == Some('.') {
                        scanner.next();
                        if scanner.peek() == Some('=') {
                            scanner.next();
                            (Kind::Close, start, scanner.byte_index())
                        } else {
                            (Kind::Open, start, scanner.byte_index())
                        }
                    } else {
                        (Kind::Dot, start, scanner.byte_index())
                    }
                }
                '(' => (Kind::Lp, start, scanner.byte_index()),
                ')' => (Kind::Rp, start, scanner.byte_index()),
                '{' => (Kind::Lb, start, scanner.byte_index()),
                '}' => (Kind::Rb, start, scanner.byte_index()),
                '[' => (Kind::Ls, start, scanner.byte_index()),
                ']' => (Kind::Rs, start, scanner.byte_index()),
                '"' => {
                    let (valid, content_start, content_end) = scanner.quoted('"', start);
                    let literal = &text[content_start..content_end];
                    let kind = if valid {
                        Kind::String(literal.to_owned())
                    } else {
                        Kind::Illegal(literal.to_owned())
                    };
                    tokens.push(Token::new(kind, line, column));
                    continue;
                }
                '0'..='9' => {
                    let mut has_dot = false;
                    while let Some(peek) = scanner.peek() {
                        if peek.is_ascii_digit() {
                            scanner.next();
                        } else if peek == '.' {
                            if scanner.peek_n(1) == Some('.') {
                                break;
                            }
                            has_dot = true;
                            scanner.next();
                        } else {
                            break;
                        }
                    }
                    if has_dot {
                        (Kind::Float("".to_owned()), start, scanner.byte_index())
                    } else {
                        (Kind::Integer("".to_owned()), start, scanner.byte_index())
                    }
                }
                'A'..='Z' | 'a'..='z' | '_' => {
                    while let Some(peek) = scanner.peek() {
                        if peek.is_ascii_alphanumeric() || peek == '_' {
                            scanner.next();
                        } else {
                            break;
                        }
                    }
                    let end = scanner.byte_index();
                    let kind = match &text[start..end] {
                        "true" => Kind::True,
                        "false" => Kind::False,
                        "fn" => Kind::Function,
                        "let" => Kind::Let,
                        "if" => Kind::If,
                        "else" => Kind::Else,
                        "test" => Kind::Test,
                        "break" => Kind::Break,
                        "continue" => Kind::Continue,
                        "loop" => Kind::Loop,
                        "while" => Kind::While,
                        "for" => Kind::For,
                        "in" => Kind::In,
                        "client" => Kind::Client,
                        _ => Kind::Ident("".to_owned()),
                    };
                    (kind, start, end)
                }
                _ => (Kind::Illegal("".to_owned()), start, scanner.byte_index()),
            };
            let literal = &text[content_start..content_end];
            let kind = match kind {
                Kind::Illegal(_) => Kind::Illegal(literal.to_owned()),
                Kind::Ident(_) => Kind::Ident(literal.to_owned()),
                Kind::Integer(_) => Kind::Integer(literal.to_owned()),
                Kind::Float(_) => Kind::Float(literal.to_owned()),
                kind => kind,
            };
            tokens.push(Token::new(kind, line, column));
        }
    }
    tokens.push(Token::new(Kind::Eof, scanner.line, scanner.column));
    tokens
}

struct Scanner {
    chars: Vec<(usize, char)>,
    text_len: usize,
    index: usize,
    line: usize,
    column: usize,
    previous_line: usize,
    previous_column: usize,
}

impl Scanner {
    fn new(text: &str) -> Self {
        Self {
            chars: text.char_indices().collect(),
            text_len: text.len(),
            index: 0,
            line: 1,
            column: 1,
            previous_line: 1,
            previous_column: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.peek_n(0)
    }

    fn peek_n(&self, offset: usize) -> Option<char> {
        self.chars
            .get(self.index + offset)
            .map(|(_, character)| *character)
    }

    fn next(&mut self) -> Option<char> {
        let character = self.peek()?;
        self.previous_line = self.line;
        self.previous_column = self.column;
        self.index += 1;
        if character == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(character)
    }

    fn previous_location(&self) -> (usize, usize) {
        (self.previous_line, self.previous_column)
    }

    fn previous_index(&self) -> usize {
        self.chars[self.index - 1].0
    }

    fn byte_index(&self) -> usize {
        self.chars
            .get(self.index)
            .map_or(self.text_len, |(index, _)| *index)
    }

    fn quoted(&mut self, end: char, start: usize) -> (bool, usize, usize) {
        let content_start = self.byte_index();
        let mut valid = true;
        while let Some(character) = self.next() {
            if character == end {
                return (valid, content_start, self.previous_index());
            }
            if character == '\\' {
                match self.next() {
                    Some('"' | '\\' | 'b' | 'f' | 'n' | 'r' | 't' | '(') => {}
                    Some(_) => valid = false,
                    None => break,
                }
            }
        }
        (false, start, self.byte_index())
    }
}

#[test]
fn test_segment() {
    let text = r#"
            let five = 5;
            let ten = 10;
            let float = 3.14159265358979323846264338327950288;

            let add = fn(x, y) {
            x + y;
            };
            
            let number = add(five, ten);
            !-/*5;
            5 < 10 > 5;
            
            if (5 < 10) {
                return true;
            } else {
                return false;
            }
            
            10 == 10;
            10 != 9;
            "foobar"
            "foo bar"
            [1, 2];
            {"foo": "bar"};
            _a2
            left.field
            test expectStatusOk {
                let response = user.getIp();
                response.status
            }
            1&0
            1|0
            true&&false
            false||true
            "#;
    let expect = vec![
        (Kind::Let, "let"),
        (Kind::Ident("five".to_owned()), "five"),
        (Kind::Assign, "="),
        (Kind::Integer("5".to_owned()), "5"),
        (Kind::Semi, ";"),
        (Kind::Let, "let"),
        (Kind::Ident("ten".to_owned()), "ten"),
        (Kind::Assign, "="),
        (Kind::Integer("10".to_owned()), "10"),
        (Kind::Semi, ";"),
        (Kind::Let, "let"),
        (Kind::Ident("float".to_owned()), "float"),
        (Kind::Assign, "="),
        (
            Kind::Float("3.14159265358979323846264338327950288".to_owned()),
            "3.14159265358979323846264338327950288",
        ),
        (Kind::Semi, ";"),
        (Kind::Let, "let"),
        (Kind::Ident("add".to_owned()), "add"),
        (Kind::Assign, "="),
        (Kind::Function, "fn"),
        (Kind::Lp, "("),
        (Kind::Ident("x".to_owned()), "x"),
        (Kind::Comma, ","),
        (Kind::Ident("y".to_owned()), "y"),
        (Kind::Rp, ")"),
        (Kind::Lb, "{"),
        (Kind::Ident("x".to_owned()), "x"),
        (Kind::Add, "+"),
        (Kind::Ident("y".to_owned()), "y"),
        (Kind::Semi, ";"),
        (Kind::Rb, "}"),
        (Kind::Semi, ";"),
        (Kind::Let, "let"),
        (Kind::Ident("number".to_owned()), "number"),
        (Kind::Assign, "="),
        (Kind::Ident("add".to_owned()), "add"),
        (Kind::Lp, "("),
        (Kind::Ident("five".to_owned()), "five"),
        (Kind::Comma, ","),
        (Kind::Ident("ten".to_owned()), "ten"),
        (Kind::Rp, ")"),
        (Kind::Semi, ";"),
        (Kind::Not, "!"),
        (Kind::Sub, "-"),
        (Kind::Div, "/"),
        (Kind::Mul, "*"),
        (Kind::Integer("5".to_owned()), "5"),
        (Kind::Semi, ";"),
        (Kind::Integer("5".to_owned()), "5"),
        (Kind::Lt, "<"),
        (Kind::Integer("10".to_owned()), "10"),
        (Kind::Gt, ">"),
        (Kind::Integer("5".to_owned()), "5"),
        (Kind::Semi, ";"),
        (Kind::If, "if"),
        (Kind::Lp, "("),
        (Kind::Integer("5".to_owned()), "5"),
        (Kind::Lt, "<"),
        (Kind::Integer("10".to_owned()), "10"),
        (Kind::Rp, ")"),
        (Kind::Lb, "{"),
        (Kind::Ident("return".to_owned()), "return"),
        (Kind::True, "true"),
        (Kind::Semi, ";"),
        (Kind::Rb, "}"),
        (Kind::Else, "else"),
        (Kind::Lb, "{"),
        (Kind::Ident("return".to_owned()), "return"),
        (Kind::False, "false"),
        (Kind::Semi, ";"),
        (Kind::Rb, "}"),
        (Kind::Integer("10".to_owned()), "10"),
        (Kind::Eq, "=="),
        (Kind::Integer("10".to_owned()), "10"),
        (Kind::Semi, ";"),
        (Kind::Integer("10".to_owned()), "10"),
        (Kind::Ne, "!="),
        (Kind::Integer("9".to_owned()), "9"),
        (Kind::Semi, ";"),
        (Kind::String("foobar".to_owned()), "foobar"),
        (Kind::String("foo bar".to_owned()), "foo bar"),
        (Kind::Ls, "["),
        (Kind::Integer("1".to_owned()), "1"),
        (Kind::Comma, ","),
        (Kind::Integer("2".to_owned()), "2"),
        (Kind::Rs, "]"),
        (Kind::Semi, ";"),
        (Kind::Lb, "{"),
        (Kind::String("foo".to_owned()), "foo"),
        (Kind::Colon, ":"),
        (Kind::String("bar".to_owned()), "bar"),
        (Kind::Rb, "}"),
        (Kind::Semi, ";"),
        (Kind::Ident("_a2".to_owned()), "_a2"),
        (Kind::Ident("left".to_owned()), "left"),
        (Kind::Dot, "."),
        (Kind::Ident("field".to_owned()), "field"),
        (Kind::Test, "test"),
        (Kind::Ident("expectStatusOk".to_owned()), "expectStatusOk"),
        (Kind::Lb, "{"),
        (Kind::Let, "let"),
        (Kind::Ident("response".to_owned()), "response"),
        (Kind::Assign, "="),
        (Kind::Ident("user".to_owned()), "user"),
        (Kind::Dot, "."),
        (Kind::Ident("getIp".to_owned()), "getIp"),
        (Kind::Lp, "("),
        (Kind::Rp, ")"),
        (Kind::Semi, ";"),
        (Kind::Ident("response".to_owned()), "response"),
        (Kind::Dot, "."),
        (Kind::Ident("status".to_owned()), "status"),
        (Kind::Rb, "}"),
        (Kind::Integer("1".to_owned()), "1"),
        (Kind::Ba, "&"),
        (Kind::Integer("0".to_owned()), "0"),
        (Kind::Integer("1".to_owned()), "1"),
        (Kind::Bo, "|"),
        (Kind::Integer("0".to_owned()), "0"),
        (Kind::True, "true"),
        (Kind::La, "&&"),
        (Kind::False, "false"),
        (Kind::False, "false"),
        (Kind::Lo, "||"),
        (Kind::True, "true"),
        (Kind::Eof, "💥"),
    ];
    let tokens = segment(text);
    assert_eq!(expect.len(), tokens.len());
    for (i, (kind, literal)) in expect.into_iter().enumerate() {
        let token = tokens.get(i).unwrap();
        assert_eq!(kind, token.kind);
        assert_eq!(literal, token.kind.literal());
    }
}

#[test]
fn test_string_escapes_and_interpolation_marker() {
    let tokens = segment(r#""say \"hi\"\\path\n\t\(name)""#);
    assert_eq!(
        tokens[0].kind,
        Kind::String(r#"say \"hi\"\\path\n\t\(name)"#.to_owned())
    );
    assert_eq!(tokens[0].kind.literal(), r#"say \"hi\"\\path\n\t\(name)"#);
}

#[test]
fn test_token_locations() {
    let tokens = segment("let x = 1;\n  test call {}");
    assert_eq!((tokens[0].line, tokens[0].column), (1, 1));
    let test = tokens
        .iter()
        .find(|token| token.kind == Kind::Test)
        .unwrap();
    assert_eq!((test.line, test.column), (2, 3));
}

#[test]
fn test_invalid_string_escape() {
    let tokens = segment(r#""invalid\q""#);
    assert_eq!(tokens[0].kind, Kind::Illegal(r#"invalid\q"#.to_owned()));
    assert_eq!(tokens[0].kind.literal(), r#"invalid\q"#);
}

#[test]
fn test_unicode_literal_uses_valid_utf8_boundaries() {
    let tokens = segment("你好 + world");
    assert_eq!(tokens[0].kind, Kind::Illegal("你".to_owned()));
    assert_eq!(tokens[0].kind.literal(), "你");
    assert_eq!(tokens[1].kind, Kind::Illegal("好".to_owned()));
    assert_eq!(tokens[1].kind.literal(), "好");
    assert_eq!(tokens[2].kind.literal(), "+");
}

#[test]
fn test_segment_loop_tokens() {
    let text = "loop { break 1 } continue while for in 1..2 1.. ..2 1..=2 ..=2";
    let expect = vec![
        (Kind::Loop, "loop"),
        (Kind::Lb, "{"),
        (Kind::Break, "break"),
        (Kind::Integer("1".to_owned()), "1"),
        (Kind::Rb, "}"),
        (Kind::Continue, "continue"),
        (Kind::While, "while"),
        (Kind::For, "for"),
        (Kind::In, "in"),
        (Kind::Integer("1".to_owned()), "1"),
        (Kind::Open, ".."),
        (Kind::Integer("2".to_owned()), "2"),
        (Kind::Integer("1".to_owned()), "1"),
        (Kind::Open, ".."),
        (Kind::Open, ".."),
        (Kind::Integer("2".to_owned()), "2"),
        (Kind::Integer("1".to_owned()), "1"),
        (Kind::Close, "..="),
        (Kind::Integer("2".to_owned()), "2"),
        (Kind::Close, "..="),
        (Kind::Integer("2".to_owned()), "2"),
        (Kind::Eof, "💥"),
    ];
    let tokens = segment(text);
    assert_eq!(expect.len(), tokens.len());
    for (i, (kind, literal)) in expect.into_iter().enumerate() {
        let token = tokens.get(i).unwrap();
        assert_eq!(kind, token.kind);
        assert_eq!(literal, token.kind.literal());
    }
}
