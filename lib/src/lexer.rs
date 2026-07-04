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
                let (kind, lite, end) = match byte {
                    b'=' => {
                        if let Some(b'=') = bytes.get(peek) {
                            (Kind::Eq, string!(text, start..=peek), peek)
                        } else {
                            (Kind::Assign, string!(text, start..=index), index)
                        }
                    }
                    b'!' => {
                        if let Some(b'=') = bytes.get(peek) {
                            (Kind::Ne, string!(text, start..=peek), peek)
                        } else {
                            (Kind::Not, string!(text, start..=index), index)
                        }
                    }
                    b'+' => (Kind::Add, string!(text, start..=index), index),
                    b'-' => (Kind::Sub, string!(text, start..=index), index),
                    b'*' => (Kind::Mul, string!(text, start..=index), index),
                    b'/' => (Kind::Div, string!(text, start..=index), index),
                    b'%' => (Kind::Rem, string!(text, start..=index), index),
                    b'^' => (Kind::Bx, string!(text, start..=index), index),
                    b'|' => {
                        if let Some(b'|') = bytes.get(peek) {
                            (Kind::Lo, string!(text, start..=peek), peek)
                        } else {
                            (Kind::Bo, string!(text, start..=index), index)
                        }
                    }
                    b'&' => {
                        if let Some(b'&') = bytes.get(peek) {
                            (Kind::La, string!(text, start..=peek), peek)
                        } else {
                            (Kind::Ba, string!(text, start..=index), index)
                        }
                    }
                    b'<' => {
                        if let Some(b'=') = bytes.get(peek) {
                            (Kind::Le, string!(text, start..=peek), peek)
                        } else if let Some(b'<') = bytes.get(peek) {
                            (Kind::Sl, string!(text, start..=peek), peek)
                        } else {
                            (Kind::Lt, string!(text, start..=index), index)
                        }
                    }
                    b'>' => {
                        if let Some(b'=') = bytes.get(peek) {
                            (Kind::Ge, string!(text, start..=peek), peek)
                        } else if let Some(b'>') = bytes.get(peek) {
                            (Kind::Sr, string!(text, start..=peek), peek)
                        } else {
                            (Kind::Gt, string!(text, start..=index), index)
                        }
                    }
                    b',' => (Kind::Comma, string!(text, start..=index), index),
                    b';' => (Kind::Semi, string!(text, start..=index), index),
                    b':' => (Kind::Colon, string!(text, start..=index), index),
                    b'.' => {
                        if let Some(b'.') = bytes.get(peek) {
                            if let Some(b'=') = bytes.get(peek + 1) {
                                (Kind::Close, string!(text, start..=peek + 1), peek + 1)
                            } else {
                                (Kind::Open, string!(text, start..=peek), peek)
                            }
                        } else {
                            (Kind::Dot, string!(text, start..=index), index)
                        }
                    }
                    b'(' => (Kind::Lp, string!(text, start..=index), index),
                    b')' => (Kind::Rp, string!(text, start..=index), index),
                    b'{' => (Kind::Lb, string!(text, start..=index), index),
                    b'}' => (Kind::Rb, string!(text, start..=index), index),
                    b'[' => (Kind::Ls, string!(text, start..=index), index),
                    b']' => (Kind::Rs, string!(text, start..=index), index),
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
                                Ok(string) => (Kind::String, string, peek),
                                Err(error) => (Kind::Illegal, error.to_string(), peek),
                            }
                        } else {
                            (Kind::Illegal, string!(text, index + 1..peek), peek)
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
                        let number = string!(text, index..peek);
                        if point {
                            (Kind::Float, number, peek - 1)
                        } else {
                            (Kind::Integer, number, peek - 1)
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
                        let word = string!(text, index..peek);
                        let kind = match word.as_str() {
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
                            _ => Kind::Ident,
                        };
                        (kind, word, peek - 1)
                    }
                    _ => {
                        let rear = &text[index..];
                        if let Some(char) = rear.chars().next() {
                            let length = char.len_utf8();
                            let end = index + length - 1;
                            (Kind::Illegal, char.to_string(), end)
                        } else {
                            let end = index + rear.len() - 1;
                            (Kind::Illegal, rear.to_string(), end)
                        }
                    }
                };
                peek = end + 1;
                let span = Span { start, end };
                tokens.push(Token::new(kind, span, lite));
            }
            index = peek;
        }
        tokens.push(Token::new(
            Kind::Eof,
            Span {
                start: text.len(),
                end: text.len(),
            },
            "💥".to_owned(),
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
        (Kind::String, "hello"),
        (Kind::String, "hello world"),
        (Kind::Ls, "["),
        (Kind::Integer, "1"),
        (Kind::Comma, ","),
        (Kind::Integer, "2"),
        (Kind::Rs, "]"),
        (Kind::Semi, ";"),
        (Kind::Lb, "{"),
        (Kind::String, "key"),
        (Kind::Colon, ":"),
        (Kind::String, "value"),
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
        (Kind::Eof, "💥"),
    ];
    let tokens = Lexer::new().segment(text);
    assert_eq!(expect.len(), tokens.len());
    for (i, (kind, lite)) in expect.into_iter().enumerate() {
        let token = tokens.get(i).unwrap();
        assert_eq!(kind, token.kind);
        assert_eq!(lite, token.lite());
        assert_eq!(lite, token.lite);
    }
}

#[test]
fn test_segment_control_flow() {
    let text = "loop { break 1 } continue while for in 1..2 1.. ..2 1..=2 ..=2";
    let expect = vec![
        (Kind::Loop, "loop"),
        (Kind::Lb, "{"),
        (Kind::Break, "break"),
        (Kind::Integer, "1"),
        (Kind::Rb, "}"),
        (Kind::Continue, "continue"),
        (Kind::While, "while"),
        (Kind::For, "for"),
        (Kind::In, "in"),
        (Kind::Integer, "1"),
        (Kind::Open, ".."),
        (Kind::Integer, "2"),
        (Kind::Integer, "1"),
        (Kind::Open, ".."),
        (Kind::Open, ".."),
        (Kind::Integer, "2"),
        (Kind::Integer, "1"),
        (Kind::Close, "..="),
        (Kind::Integer, "2"),
        (Kind::Close, "..="),
        (Kind::Integer, "2"),
        (Kind::Eof, "💥"),
    ];
    let tokens = Lexer::new().segment(text);
    assert_eq!(expect.len(), tokens.len());
    for (i, (kind, lite)) in expect.into_iter().enumerate() {
        let token = tokens.get(i).unwrap();
        assert_eq!(kind, token.kind);
        assert_eq!(lite, token.lite());
        assert_eq!(lite, token.lite);
    }
}

#[test]
fn test_segment_token_locations() {
    let text = "let x = 1;\n  test call {}";
    let tokens = Lexer::new().segment(text);
    assert_eq!((tokens[0].span.start, tokens[0].span.end), (0, 2));
    assert_eq!(tokens[0].lite(), &text[0..=2]);
    assert_eq!(tokens[0].lite, "let");
    assert_eq!((tokens[5].span.start, tokens[5].span.end), (13, 16));
    assert_eq!(tokens[5].lite(), &text[13..=16]);
    assert_eq!(tokens[5].lite, "test");
    assert_eq!(tokens[6].lite, "call");
}

#[test]
fn test_segment_string_escapes() {
    let tokens = Lexer::new().segment(r#""say \"hi\"\\path\n\t\(name)""#);
    let string = "say \"hi\"\\path\n\t\\(name)";
    assert_eq!(tokens[0].lite, string);
    assert_eq!(tokens[0].kind, Kind::String);
}

#[test]
fn test_segment_string_half() {
    let tokens = Lexer::new().segment(r#"let string = "This is half; let x = 2;"#);
    assert_eq!(tokens[3].kind, Kind::Illegal);
}

#[test]
fn test_segment_string_invalid() {
    let string = r#"invalid\qline"#;
    let text = format!("\"{}\"", string);
    let tokens = Lexer::new().segment(text.as_str());
    assert_eq!(tokens[0].kind, Kind::String);
    assert_eq!(tokens[0].lite, string);
}

#[test]
fn test_segment_unicode_lite() {
    let tokens = Lexer::new().segment("你好 + world");
    assert_eq!(tokens[0].kind, Kind::Illegal);
    assert_eq!(tokens[0].lite, "你");
    assert_eq!(tokens[1].kind, Kind::Illegal);
    assert_eq!(tokens[1].lite, "好");
    assert_eq!(tokens[2].lite, "+");
    assert_eq!(tokens[2].lite(), "+");
}
