use crate::Kind;
use crate::Token;

pub fn segment(text: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = text.chars().peekable();
    while let Some(char) = chars.next() {
        if !char.is_whitespace() {
            let (kind, literal) = match char {
                '=' => {
                    if let Some(peek @ '=') = chars.peek() {
                        let literal = String::from_iter([char, *peek]);
                        chars.next();
                        (Kind::Eq, literal)
                    } else {
                        (Kind::Assign, String::from(char))
                    }
                }
                '!' => {
                    if let Some(peek @ '=') = chars.peek() {
                        let literal = String::from_iter([char, *peek]);
                        chars.next();
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
                    if let Some(peek @ '|') = chars.peek() {
                        let literal = String::from_iter([char, *peek]);
                        chars.next();
                        (Kind::Lo, literal)
                    } else {
                        (Kind::Bo, String::from(char))
                    }
                }
                '&' => {
                    if let Some(peek @ '&') = chars.peek() {
                        let literal = String::from_iter([char, *peek]);
                        chars.next();
                        (Kind::La, literal)
                    } else {
                        (Kind::Ba, String::from(char))
                    }
                }
                '<' => {
                    if let Some(peek @ '=') = chars.peek() {
                        let literal = String::from_iter([char, *peek]);
                        chars.next();
                        (Kind::Le, literal)
                    } else if let Some(peek @ '<') = chars.peek() {
                        let literal = String::from_iter([char, *peek]);
                        chars.next();
                        (Kind::Sl, literal)
                    } else {
                        (Kind::Lt, String::from(char))
                    }
                }
                '>' => {
                    if let Some(peek @ '=') = chars.peek() {
                        let literal = String::from_iter([char, *peek]);
                        chars.next();
                        (Kind::Ge, literal)
                    } else if let Some(peek @ '>') = chars.peek() {
                        let literal = String::from_iter([char, *peek]);
                        chars.next();
                        (Kind::Sr, literal)
                    } else {
                        (Kind::Gt, String::from(char))
                    }
                }
                ',' => (Kind::Comma, String::from(char)),
                ';' => (Kind::Semi, String::from(char)),
                ':' => (Kind::Colon, String::from(char)),
                '.' => (Kind::Dot, String::from(char)),
                '(' => (Kind::Lp, String::from(char)),
                ')' => (Kind::Rp, String::from(char)),
                '{' => (Kind::Lb, String::from(char)),
                '}' => (Kind::Rb, String::from(char)),
                '[' => (Kind::Ls, String::from(char)),
                ']' => (Kind::Rs, String::from(char)),
                '"' => {
                    let mut string = String::new();
                    while let Some(peek) = chars.peek() {
                        if *peek == '"' {
                            chars.next();
                            break;
                        } else {
                            string.push(*peek);
                            chars.next();
                        }
                    }
                    (Kind::String, string)
                }
                '`' => {
                    let mut string = String::new();
                    while let Some(peek) = chars.peek() {
                        if *peek == '`' {
                            chars.next();
                            break;
                        } else {
                            string.push(*peek);
                            chars.next();
                        }
                    }
                    (Kind::Template, string.to_owned())
                }
                '0'..='9' => {
                    let mut string = String::from(char);
                    let mut has_dot = false;
                    while let Some(peek) = chars.peek() {
                        if peek.is_ascii_digit() || *peek == '.' {
                            if *peek == '.' {
                                has_dot = true
                            }
                            string.push(*peek);
                            chars.next();
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
                    while let Some(peek) = chars.peek() {
                        if peek.is_ascii_alphanumeric() || *peek == '_' {
                            string.push(*peek);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    match string.as_str() {
                        "true" => (Kind::True, string),
                        "false" => (Kind::False, string),
                        "request" => (Kind::Request, string),
                        "let" => (Kind::Let, string),
                        "if" => (Kind::If, string),
                        "else" => (Kind::Else, string),
                        "test" => (Kind::Test, string),
                        _ => (Kind::Ident, string),
                    }
                }
                _ => (Kind::Illegal, String::from(char)),
            };
            tokens.push(Token::new(kind, literal));
        }
    }
    tokens.push(Token::new(Kind::Eof, String::new()));
    tokens
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
            request get()`
              GET http://example.com
              Host: example.com
            `[
            status == 200,
            regex(text, "^\d{4}-\d{2}-\d{2}$") == "2022-02-22"
            ]
            object.field
            test expectStatusOk {
                let response = get();
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
        (Kind::Ident, "fn"),
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
        (Kind::Request, "request"),
        (Kind::Ident, "get"),
        (Kind::Lp, "("),
        (Kind::Rp, ")"),
        (
            Kind::Template,
            "\n              GET http://example.com\n              Host: example.com\n            ",
        ),
        (Kind::Ls, "["),
        (Kind::Ident, "status"),
        (Kind::Eq, "=="),
        (Kind::Integer, "200"),
        (Kind::Comma, ","),
        (Kind::Ident, "regex"),
        (Kind::Lp, "("),
        (Kind::Ident, "text"),
        (Kind::Comma, ","),
        (Kind::String, r"^\d{4}-\d{2}-\d{2}$"),
        (Kind::Rp, ")"),
        (Kind::Eq, "=="),
        (Kind::String, "2022-02-22"),
        (Kind::Rs, "]"),
        (Kind::Ident, "object"),
        (Kind::Dot, "."),
        (Kind::Ident, "field"),
        (Kind::Test, "test"),
        (Kind::Ident, "expectStatusOk"),
        (Kind::Lb, "{"),
        (Kind::Let, "let"),
        (Kind::Ident, "response"),
        (Kind::Assign, "="),
        (Kind::Ident, "get"),
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
        assert!(kind == token.kind);
        assert_eq!(literal, token.literal);
    }
}
