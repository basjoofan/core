use super::Kind;
use super::Span;
use super::Token;

pub struct Lexer {}

impl Lexer {
    pub fn new() -> Self {
        Self {}
    }

    pub fn segment(&mut self, text: &str) -> Vec<Token> {
        let mut tokens = Vec::new();
        let bytes = text.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            let mut peek = index + 1;
            let byte = bytes[index];
            if !byte.is_ascii_whitespace() {
                let start = index;
                let (kind, end) = match byte {
                    b'=' => {
                        if let Some(b'=') = bytes.get(peek) {
                            (Kind::Eq, peek)
                        } else {
                            (Kind::Assign, index)
                        }
                    }
                    b'!' => {
                        if let Some(b'=') = bytes.get(peek) {
                            (Kind::Ne, peek)
                        } else {
                            (Kind::Not, index)
                        }
                    }
                    b'+' => (Kind::Add, index),
                    b'-' => (Kind::Sub, index),
                    b'*' => (Kind::Mul, index),
                    b'/' => (Kind::Div, index),
                    b'%' => (Kind::Rem, index),
                    b'^' => (Kind::Bx, index),
                    b'|' => {
                        if let Some(b'|') = bytes.get(peek) {
                            (Kind::Lo, peek)
                        } else {
                            (Kind::Bo, index)
                        }
                    }
                    b'&' => {
                        if let Some(b'&') = bytes.get(peek) {
                            (Kind::La, peek)
                        } else {
                            (Kind::Ba, index)
                        }
                    }
                    b'<' => {
                        if let Some(b'=') = bytes.get(peek) {
                            (Kind::Le, peek)
                        } else if let Some(b'<') = bytes.get(peek) {
                            (Kind::Sl, peek)
                        } else {
                            (Kind::Lt, index)
                        }
                    }
                    b'>' => {
                        if let Some(b'=') = bytes.get(peek) {
                            (Kind::Ge, peek)
                        } else if let Some(b'>') = bytes.get(peek) {
                            (Kind::Sr, peek)
                        } else {
                            (Kind::Gt, index)
                        }
                    }
                    b',' => (Kind::Comma, index),
                    b';' => (Kind::Semi, index),
                    b':' => (Kind::Colon, index),
                    b'.' => {
                        if let Some(b'.') = bytes.get(peek) {
                            if let Some(b'=') = bytes.get(peek + 1) {
                                (Kind::Close, peek + 1)
                            } else {
                                (Kind::Open, peek)
                            }
                        } else {
                            (Kind::Dot, index)
                        }
                    }
                    b'(' => (Kind::Lp, index),
                    b')' => (Kind::Rp, index),
                    b'{' => (Kind::Lb, index),
                    b'}' => (Kind::Rb, index),
                    b'[' => (Kind::Ls, index),
                    b']' => (Kind::Rs, index),
                    b'"' => {
                        let mut string = Vec::new();
                        let mut valid = false;
                        while let Some(byte) = bytes.get(peek) {
                            if byte == &b'\\' {
                                peek += 1;
                                if let Some(byte) = bytes.get(peek) {
                                    peek += 1;
                                    match byte {
                                        b'"' => string.push(b'"'),
                                        b'\\' => string.push(b'\\'),
                                        b'n' => string.push(b'\n'),
                                        b'r' => string.push(b'\r'),
                                        b't' => string.push(b'\t'),
                                        b'0' => string.push(b'\0'),
                                        byte => {
                                            string.push(b'\\');
                                            string.push(*byte);
                                        }
                                    }
                                } else {
                                    break;
                                }
                            } else if byte == &b'"' {
                                valid = true;
                                break;
                            } else {
                                string.push(*byte);
                                peek += 1;
                            }
                        }
                        if valid {
                            match String::from_utf8(string) {
                                Ok(string) => (Kind::String(string), peek),
                                Err(error) => (Kind::Illegal(error.to_string()), peek),
                            }
                        } else {
                            (Kind::Illegal(text[index + 1..peek].to_owned()), peek)
                        }
                    }
                    b'0'..=b'9' => {
                        let mut point = false;
                        while let Some(byte) = bytes.get(peek) {
                            if byte.is_ascii_digit() {
                                peek += 1;
                            } else if byte == &b'.' {
                                if bytes.get(peek + 1) == Some(&b'.') {
                                    break;
                                }
                                point = true;
                                peek += 1;
                            } else {
                                break;
                            }
                        }
                        let number = text[index..peek].to_owned();
                        if point {
                            (Kind::Float(number), peek - 1)
                        } else {
                            (Kind::Integer(number), peek - 1)
                        }
                    }
                    b'A'..=b'Z' | b'a'..=b'z' | b'_' => {
                        while let Some(byte) = bytes.get(peek) {
                            if byte.is_ascii_alphanumeric() || byte == &b'_' {
                                peek += 1;
                            } else {
                                break;
                            }
                        }
                        (
                            match &text[index..peek] {
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
                                literal => Kind::Ident(literal.to_owned()),
                            },
                            peek - 1,
                        )
                    }
                    _ => {
                        let rear = &text[index..];
                        if let Some(char) = rear.chars().next() {
                            let length = char.len_utf8();
                            let end = index + length - 1;
                            (Kind::Illegal(char.to_string()), end)
                        } else {
                            let end = index + rear.len() - 1;
                            (Kind::Illegal(rear.to_string()), end)
                        }
                    }
                };
                peek = end + 1;
                let span = Span { start, end };
                tokens.push(Token { kind, span });
            }
            index = peek;
        }
        tokens.push(Token::new(
            Kind::Eof,
            Span {
                start: text.len(),
                end: text.len(),
            },
        ));
        tokens
    }
}

#[test]
fn test_segment_base_tokens() {
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
            "hello"
            "hello world"
            [1, 2];
            {"key": "value"};
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
        (Kind::String("hello".to_owned()), "hello"),
        (Kind::String("hello world".to_owned()), "hello world"),
        (Kind::Ls, "["),
        (Kind::Integer("1".to_owned()), "1"),
        (Kind::Comma, ","),
        (Kind::Integer("2".to_owned()), "2"),
        (Kind::Rs, "]"),
        (Kind::Semi, ";"),
        (Kind::Lb, "{"),
        (Kind::String("key".to_owned()), "key"),
        (Kind::Colon, ":"),
        (Kind::String("value".to_owned()), "value"),
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
    let tokens = Lexer::new().segment(text);
    assert_eq!(expect.len(), tokens.len());
    for (i, (kind, literal)) in expect.into_iter().enumerate() {
        let token = tokens.get(i).unwrap();
        assert_eq!(kind, token.kind);
        assert_eq!(literal, token.kind.literal());
    }
}

#[test]
fn test_segment_control_flow() {
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
    let tokens = Lexer::new().segment(text);
    assert_eq!(expect.len(), tokens.len());
    for (i, (kind, literal)) in expect.into_iter().enumerate() {
        let token = tokens.get(i).unwrap();
        println!("{}, {:?}, {} | {}", i, kind, literal, token.kind.literal());
        assert_eq!(kind, token.kind);
        assert_eq!(literal, token.kind.literal());
    }
}

#[test]
fn test_segment_token_locations() {
    let text = "let x = 1;\n  test call {}";
    let tokens = Lexer::new().segment(text);
    assert_eq!((tokens[0].span.start, tokens[0].span.end), (0, 2));
    assert_eq!(tokens[0].kind.literal(), &text[0..=2]);
    assert_eq!((tokens[5].span.start, tokens[5].span.end), (13, 16));
    assert_eq!(tokens[5].kind.literal(), &text[13..=16]);
}

#[test]
fn test_segment_string_escapes() {
    let tokens = Lexer::new().segment(r#""say \"hi\"\\path\n\t\(name)""#);
    let string = "say \"hi\"\\path\n\t\\(name)";
    assert_eq!(tokens[0].kind.literal(), string);
    assert_eq!(tokens[0].kind, Kind::String(string.to_string()));
}

#[test]
fn test_segment_string_half() {
    let tokens = Lexer::new().segment(r#"let string = "This is half; let x = 2;"#);
    assert_eq!(
        tokens[3].kind,
        Kind::Illegal(r#"This is half; let x = 2;"#.to_owned())
    );
}

#[test]
fn test_segment_string_invalid() {
    let string = r#"invalid\qline"#;
    let text = format!("\"{}\"", string);
    let tokens = Lexer::new().segment(text.as_str());
    assert_eq!(tokens[0].kind, Kind::String(string.to_owned()));
    assert_eq!(tokens[0].kind.literal(), string);
}

#[test]
fn test_segment_unicode_literal() {
    let tokens = Lexer::new().segment("你好 + world");
    assert_eq!(tokens[0].kind, Kind::Illegal("你".to_owned()));
    assert_eq!(tokens[0].kind.literal(), "你");
    assert_eq!(tokens[1].kind, Kind::Illegal("好".to_owned()));
    assert_eq!(tokens[1].kind.literal(), "好");
    assert_eq!(tokens[2].kind.literal(), "+");
}
