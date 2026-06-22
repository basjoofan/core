use super::Kind;
use super::Token;

pub fn segment(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut scanner = Scanner::new(text);
    while let Some(char) = scanner.next() {
        if !char.is_whitespace() {
            let (line, column) = scanner.previous_location();
            let (kind, literal) = match char {
                '=' => {
                    if let Some(peek @ '=') = scanner.peek() {
                        let literal = String::from_iter([char, peek]);
                        scanner.next();
                        (Kind::Eq, literal)
                    } else {
                        (Kind::Assign, String::from(char))
                    }
                }
                '!' => {
                    if let Some(peek @ '=') = scanner.peek() {
                        let literal = String::from_iter([char, peek]);
                        scanner.next();
                        (Kind::Ne, literal)
                    } else {
                        (Kind::Not, String::from(char))
                    }
                }
                '+' => (Kind::Add, String::from(char)),
                '-' => (Kind::Sub, String::from(char)),
                '*' => (Kind::Mul, String::from(char)),
                '/' => (Kind::Div, String::from(char)),
                '%' => (Kind::Rem, String::from(char)),
                '^' => (Kind::Bx, String::from(char)),
                '|' => {
                    if let Some(peek @ '|') = scanner.peek() {
                        let literal = String::from_iter([char, peek]);
                        scanner.next();
                        (Kind::Lo, literal)
                    } else {
                        (Kind::Bo, String::from(char))
                    }
                }
                '&' => {
                    if let Some(peek @ '&') = scanner.peek() {
                        let literal = String::from_iter([char, peek]);
                        scanner.next();
                        (Kind::La, literal)
                    } else {
                        (Kind::Ba, String::from(char))
                    }
                }
                '<' => {
                    if let Some(peek @ '=') = scanner.peek() {
                        let literal = String::from_iter([char, peek]);
                        scanner.next();
                        (Kind::Le, literal)
                    } else if let Some(peek @ '<') = scanner.peek() {
                        let literal = String::from_iter([char, peek]);
                        scanner.next();
                        (Kind::Sl, literal)
                    } else {
                        (Kind::Lt, String::from(char))
                    }
                }
                '>' => {
                    if let Some(peek @ '=') = scanner.peek() {
                        let literal = String::from_iter([char, peek]);
                        scanner.next();
                        (Kind::Ge, literal)
                    } else if let Some(peek @ '>') = scanner.peek() {
                        let literal = String::from_iter([char, peek]);
                        scanner.next();
                        (Kind::Sr, literal)
                    } else {
                        (Kind::Gt, String::from(char))
                    }
                }
                ',' => (Kind::Comma, String::from(char)),
                ';' => (Kind::Semi, String::from(char)),
                ':' => (Kind::Colon, String::from(char)),
                '.' => {
                    if let Some(peek @ '.') = scanner.peek() {
                        let mut literal = String::from_iter([char, peek]);
                        scanner.next();
                        if let Some(peek @ '=') = scanner.peek() {
                            literal.push(peek);
                            scanner.next();
                            (Kind::DotDotEq, literal)
                        } else {
                            (Kind::DotDot, literal)
                        }
                    } else {
                        (Kind::Dot, String::from(char))
                    }
                }
                '(' => (Kind::Lp, String::from(char)),
                ')' => (Kind::Rp, String::from(char)),
                '{' => (Kind::Lb, String::from(char)),
                '}' => (Kind::Rb, String::from(char)),
                '[' => (Kind::Ls, String::from(char)),
                ']' => (Kind::Rs, String::from(char)),
                '"' => scanner.quoted('"', Kind::String, "string"),
                '`' => scanner.quoted('`', Kind::Template, "template"),
                '\'' => {
                    let mut string = String::new();
                    while let Some(peek) = scanner.peek() {
                        if peek.is_ascii_alphanumeric() || peek == '_' {
                            string.push(peek);
                            scanner.next();
                        } else {
                            break;
                        }
                    }
                    if string.is_empty() {
                        (Kind::Illegal, String::from(char))
                    } else {
                        (Kind::Label, string)
                    }
                }
                '0'..='9' => {
                    let mut string = String::from(char);
                    let mut has_dot = false;
                    while let Some(peek) = scanner.peek() {
                        if peek.is_ascii_digit() {
                            string.push(peek);
                            scanner.next();
                        } else if peek == '.' {
                            if scanner.peek_n(1) == Some('.') {
                                break;
                            }
                            has_dot = true;
                            string.push(peek);
                            scanner.next();
                        } else {
                            break;
                        }
                    }
                    if has_dot {
                        (Kind::Float, string)
                    } else {
                        (Kind::Integer, string)
                    }
                }
                'A'..='Z' | 'a'..='z' | '_' => {
                    let mut string = String::from(char);
                    while let Some(peek) = scanner.peek() {
                        if peek.is_ascii_alphanumeric() || peek == '_' {
                            string.push(peek);
                            scanner.next();
                        } else {
                            break;
                        }
                    }
                    match string.as_str() {
                        "true" => (Kind::True, string),
                        "false" => (Kind::False, string),
                        "fn" => (Kind::Function, string),
                        "let" => (Kind::Let, string),
                        "if" => (Kind::If, string),
                        "else" => (Kind::Else, string),
                        "test" => (Kind::Test, string),
                        "break" => (Kind::Break, string),
                        "continue" => (Kind::Continue, string),
                        "loop" => (Kind::Loop, string),
                        "while" => (Kind::While, string),
                        "for" => (Kind::For, string),
                        "in" => (Kind::In, string),
                        "client" => (Kind::Client, string),
                        _ => (Kind::Ident, string),
                    }
                }
                _ => (Kind::Illegal, String::from(char)),
            };
            tokens.push(Token::at(kind, literal, line, column));
        }
    }
    tokens.push(Token::at(
        Kind::Eof,
        String::new(),
        scanner.line,
        scanner.column,
    ));
    tokens
}

struct Scanner {
    chars: Vec<char>,
    index: usize,
    line: usize,
    column: usize,
    previous_line: usize,
    previous_column: usize,
}

impl Scanner {
    fn new(text: &str) -> Self {
        Self {
            chars: text.chars().collect(),
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
        self.chars.get(self.index + offset).copied()
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

    fn quoted(&mut self, end: char, kind: Kind, name: &str) -> (Kind, String) {
        let mut string = String::new();
        let mut error = None;
        while let Some(character) = self.next() {
            if character == end {
                return match error {
                    Some(error) => (Kind::Illegal, error),
                    None => (kind, string),
                };
            }
            if character != '\\' {
                string.push(character);
                continue;
            }
            let Some(escaped) = self.next() else {
                break;
            };
            match escaped {
                '"' => string.push('"'),
                '`' => string.push('`'),
                '\\' => string.push('\\'),
                'b' => string.push('\u{08}'),
                'f' => string.push('\u{0c}'),
                'n' => string.push('\n'),
                'r' => string.push('\r'),
                't' => string.push('\t'),
                '(' => {
                    string.push('\\');
                    string.push('(');
                }
                escaped => {
                    error.get_or_insert_with(|| format!("invalid escape sequence '\\{escaped}'"));
                }
            }
        }
        (Kind::Illegal, format!("unterminated {name} literal"))
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
        (Kind::Ident, "five"),
        (Kind::Assign, "="),
        (Kind::Integer, "5"),
        (Kind::Semi, ";"),
        (Kind::Let, "let"),
        (Kind::Ident, "ten"),
        (Kind::Assign, "="),
        (Kind::Integer, "10"),
        (Kind::Semi, ";"),
        (Kind::Let, "let"),
        (Kind::Ident, "float"),
        (Kind::Assign, "="),
        (Kind::Float, "3.14159265358979323846264338327950288"),
        (Kind::Semi, ";"),
        (Kind::Let, "let"),
        (Kind::Ident, "add"),
        (Kind::Assign, "="),
        (Kind::Function, "fn"),
        (Kind::Lp, "("),
        (Kind::Ident, "x"),
        (Kind::Comma, ","),
        (Kind::Ident, "y"),
        (Kind::Rp, ")"),
        (Kind::Lb, "{"),
        (Kind::Ident, "x"),
        (Kind::Add, "+"),
        (Kind::Ident, "y"),
        (Kind::Semi, ";"),
        (Kind::Rb, "}"),
        (Kind::Semi, ";"),
        (Kind::Let, "let"),
        (Kind::Ident, "number"),
        (Kind::Assign, "="),
        (Kind::Ident, "add"),
        (Kind::Lp, "("),
        (Kind::Ident, "five"),
        (Kind::Comma, ","),
        (Kind::Ident, "ten"),
        (Kind::Rp, ")"),
        (Kind::Semi, ";"),
        (Kind::Not, "!"),
        (Kind::Sub, "-"),
        (Kind::Div, "/"),
        (Kind::Mul, "*"),
        (Kind::Integer, "5"),
        (Kind::Semi, ";"),
        (Kind::Integer, "5"),
        (Kind::Lt, "<"),
        (Kind::Integer, "10"),
        (Kind::Gt, ">"),
        (Kind::Integer, "5"),
        (Kind::Semi, ";"),
        (Kind::If, "if"),
        (Kind::Lp, "("),
        (Kind::Integer, "5"),
        (Kind::Lt, "<"),
        (Kind::Integer, "10"),
        (Kind::Rp, ")"),
        (Kind::Lb, "{"),
        (Kind::Ident, "return"),
        (Kind::True, "true"),
        (Kind::Semi, ";"),
        (Kind::Rb, "}"),
        (Kind::Else, "else"),
        (Kind::Lb, "{"),
        (Kind::Ident, "return"),
        (Kind::False, "false"),
        (Kind::Semi, ";"),
        (Kind::Rb, "}"),
        (Kind::Integer, "10"),
        (Kind::Eq, "=="),
        (Kind::Integer, "10"),
        (Kind::Semi, ";"),
        (Kind::Integer, "10"),
        (Kind::Ne, "!="),
        (Kind::Integer, "9"),
        (Kind::Semi, ";"),
        (Kind::String, "foobar"),
        (Kind::String, "foo bar"),
        (Kind::Ls, "["),
        (Kind::Integer, "1"),
        (Kind::Comma, ","),
        (Kind::Integer, "2"),
        (Kind::Rs, "]"),
        (Kind::Semi, ";"),
        (Kind::Lb, "{"),
        (Kind::String, "foo"),
        (Kind::Colon, ":"),
        (Kind::String, "bar"),
        (Kind::Rb, "}"),
        (Kind::Semi, ";"),
        (Kind::Ident, "_a2"),
        (Kind::Ident, "left"),
        (Kind::Dot, "."),
        (Kind::Ident, "field"),
        (Kind::Test, "test"),
        (Kind::Ident, "expectStatusOk"),
        (Kind::Lb, "{"),
        (Kind::Let, "let"),
        (Kind::Ident, "response"),
        (Kind::Assign, "="),
        (Kind::Ident, "user"),
        (Kind::Dot, "."),
        (Kind::Ident, "getIp"),
        (Kind::Lp, "("),
        (Kind::Rp, ")"),
        (Kind::Semi, ";"),
        (Kind::Ident, "response"),
        (Kind::Dot, "."),
        (Kind::Ident, "status"),
        (Kind::Rb, "}"),
        (Kind::Integer, "1"),
        (Kind::Ba, "&"),
        (Kind::Integer, "0"),
        (Kind::Integer, "1"),
        (Kind::Bo, "|"),
        (Kind::Integer, "0"),
        (Kind::True, "true"),
        (Kind::La, "&&"),
        (Kind::False, "false"),
        (Kind::False, "false"),
        (Kind::Lo, "||"),
        (Kind::True, "true"),
        (Kind::Eof, ""),
    ];
    let tokens = segment(text);
    assert_eq!(expect.len(), tokens.len());
    for (i, (kind, literal)) in expect.into_iter().enumerate() {
        let token = tokens.get(i).unwrap();
        assert_eq!(kind, token.kind);
        assert_eq!(literal, token.literal);
    }
}

#[test]
fn test_string_escapes_and_interpolation_marker() {
    let tokens = segment(r#""say \"hi\"\\path\n\t\(name)""#);
    assert_eq!(tokens[0].kind, Kind::String);
    assert_eq!(tokens[0].literal, "say \"hi\"\\path\n\t\\(name)");
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
    assert_eq!(tokens[0].kind, Kind::Illegal);
    assert_eq!(tokens[0].literal, "invalid escape sequence '\\q'");
}

#[test]
fn test_segment_loop_tokens() {
    let text = "'outer: loop { break 'outer 1 } continue while for in 1..2 1.. ..2 1..=2 ..=2";
    let expect = vec![
        (Kind::Label, "outer"),
        (Kind::Colon, ":"),
        (Kind::Loop, "loop"),
        (Kind::Lb, "{"),
        (Kind::Break, "break"),
        (Kind::Label, "outer"),
        (Kind::Integer, "1"),
        (Kind::Rb, "}"),
        (Kind::Continue, "continue"),
        (Kind::While, "while"),
        (Kind::For, "for"),
        (Kind::In, "in"),
        (Kind::Integer, "1"),
        (Kind::DotDot, ".."),
        (Kind::Integer, "2"),
        (Kind::Integer, "1"),
        (Kind::DotDot, ".."),
        (Kind::DotDot, ".."),
        (Kind::Integer, "2"),
        (Kind::Integer, "1"),
        (Kind::DotDotEq, "..="),
        (Kind::Integer, "2"),
        (Kind::DotDotEq, "..="),
        (Kind::Integer, "2"),
        (Kind::Eof, ""),
    ];
    let tokens = segment(text);
    assert_eq!(expect.len(), tokens.len());
    for (i, (kind, literal)) in expect.into_iter().enumerate() {
        let token = tokens.get(i).unwrap();
        assert_eq!(kind, token.kind);
        assert_eq!(literal, token.literal);
    }
}
