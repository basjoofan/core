use super::token::Kind;
use super::token::Token;

pub fn segment(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();
    while let Some(char) = chars.next() {
        if !char.is_whitespace() {
            let (kind, literal) = match char {
                '#' => (Kind::Well, String::from(char)),
                '=' => {
                    if let Some('=') = chars.peek() {
                        (Kind::Eq, String::from_iter([char, chars.next().unwrap()]))
                    } else {
                        (Kind::Assign, String::from(char))
                    }
                }
                '!' => {
                    if let Some('=') = chars.peek() {
                        (Kind::Ne, String::from_iter([char, chars.next().unwrap()]))
                    } else {
                        (Kind::Bang, String::from(char))
                    }
                }
                '+' => (Kind::Plus, String::from(char)),
                '-' => (Kind::Minus, String::from(char)),
                '*' => (Kind::Star, String::from(char)),
                '/' => (Kind::Slash, String::from(char)),
                '.' => (Kind::Dot, String::from(char)),
                '<' => (Kind::Lt, String::from(char)),
                '>' => (Kind::Gt, String::from(char)),
                ',' => (Kind::Comma, String::from(char)),
                ';' => (Kind::Semi, String::from(char)),
                ':' => (Kind::Colon, String::from(char)),
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
                            string.push(chars.next().unwrap());
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
                            string.push(chars.next().unwrap());
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
                            string.push(chars.next().unwrap());
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
                            string.push(chars.next().unwrap());
                        } else {
                            break;
                        }
                    }
                    match string.as_str() {
                        "true" => (Kind::True, string),
                        "false" => (Kind::False, string),
                        "fn" => (Kind::Fn, string),
                        "rq" => (Kind::Rq, string),
                        "let" => (Kind::Let, string),
                        "if" => (Kind::If, string),
                        "else" => (Kind::Else, string),
                        "return" => (Kind::Return, string),
                        _ => (Kind::Ident, string),
                    }
                }
                _ => (Kind::Illegal, String::from(char)),
            };
            tokens.push(Token::new(kind, literal));
        }
    }
    tokens
}

#[test]
fn test_segment() {
    let input = r#"
            let five = 5;
            let ten = 10;
            let float = 3.14159265358979323846264338327950288;

            let add = fn(x, y) {
            x + y;
            };
            
            let result = add(five, ten);
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
            #[test]
            rq request`
              GET http://example.com
              Host: example.com
            `[
            status == 200,
            regex(text, "^\d{4}-\d{2}-\d{2}$") == "2022-02-22"
            ]
            object.field
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
        (Kind::Fn, "fn"),
        (Kind::Lp, "("),
        (Kind::Ident, "x"),
        (Kind::Comma, ","),
        (Kind::Ident, "y"),
        (Kind::Rp, ")"),
        (Kind::Lb, "{"),
        (Kind::Ident, "x"),
        (Kind::Plus, "+"),
        (Kind::Ident, "y"),
        (Kind::Semi, ";"),
        (Kind::Rb, "}"),
        (Kind::Semi, ";"),
        (Kind::Let, "let"),
        (Kind::Ident, "result"),
        (Kind::Assign, "="),
        (Kind::Ident, "add"),
        (Kind::Lp, "("),
        (Kind::Ident, "five"),
        (Kind::Comma, ","),
        (Kind::Ident, "ten"),
        (Kind::Rp, ")"),
        (Kind::Semi, ";"),
        (Kind::Bang, "!"),
        (Kind::Minus, "-"),
        (Kind::Slash, "/"),
        (Kind::Star, "*"),
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
        (Kind::Return, "return"),
        (Kind::True, "true"),
        (Kind::Semi, ";"),
        (Kind::Rb, "}"),
        (Kind::Else, "else"),
        (Kind::Lb, "{"),
        (Kind::Return, "return"),
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
        (Kind::Well, "#"),
        (Kind::Ls, "["),
        (Kind::Ident, "test"),
        (Kind::Rs, "]"),
        (Kind::Rq, "rq"),
        (Kind::Ident, "request"),
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
    ];
    let tokens = segment(input);
    tokens
        .iter()
        .enumerate()
        .for_each(|(i, t)| println!("token:{} == {}", t, expect[i].1));
    assert_eq!(expect.len(), tokens.len());
    for (i, (kind, literal)) in expect.into_iter().enumerate() {
        let token = tokens.get(i).unwrap();
        assert!(kind == token.kind);
        assert_eq!(literal, token.literal);
    }
}
