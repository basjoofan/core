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
        let mut chars = text.char_indices().peekable();
        while let Some((index, char)) = chars.next() {
            if !char.is_whitespace() {
                let start = index;
                let kind = match char {
                    '=' => {
                        if let Some((_, '=')) = chars.peek() {
                            chars.next();
                            Kind::Eq
                        } else {
                            Kind::Assign
                        }
                    }
                    '!' => {
                        if let Some((_, '=')) = chars.peek() {
                            chars.next();
                            Kind::Ne
                        } else {
                            Kind::Not
                        }
                    }
                    '+' => Kind::Add,
                    '-' => Kind::Sub,
                    '*' => Kind::Mul,
                    '/' => Kind::Div,
                    '%' => Kind::Rem,
                    '^' => Kind::Bx,
                    '|' => {
                        if let Some((_, '|')) = chars.peek() {
                            chars.next();
                            Kind::Lo
                        } else {
                            Kind::Bo
                        }
                    }
                    '&' => {
                        if let Some((_, '&')) = chars.peek() {
                            chars.next();
                            Kind::La
                        } else {
                            Kind::Ba
                        }
                    }
                    '<' => {
                        if let Some((_, '=')) = chars.peek() {
                            chars.next();
                            Kind::Le
                        } else if let Some((_, '<')) = chars.peek() {
                            chars.next();
                            Kind::Sl
                        } else {
                            Kind::Lt
                        }
                    }
                    '>' => {
                        if let Some((_, '=')) = chars.peek() {
                            chars.next();
                            Kind::Ge
                        } else if let Some((_, '>')) = chars.peek() {
                            chars.next();
                            Kind::Sr
                        } else {
                            Kind::Gt
                        }
                    }
                    ',' => Kind::Comma,
                    ';' => Kind::Semi,
                    ':' => Kind::Colon,
                    '.' => {
                        if let Some((_, '.')) = chars.peek() {
                            chars.next();
                            if let Some((_, '=')) = chars.peek() {
                                chars.next();
                                Kind::Close
                            } else {
                                Kind::Open
                            }
                        } else {
                            Kind::Dot
                        }
                    }
                    '(' => Kind::Lp,
                    ')' => Kind::Rp,
                    '{' => Kind::Lb,
                    '}' => Kind::Rb,
                    '[' => Kind::Ls,
                    ']' => Kind::Rs,
                    '"' => {
                        let mut string = String::new();
                        while let Some((_, peek)) = chars.peek() {
                            if peek == &'"' {
                                chars.next();
                                break;
                            } else {
                                string.push(*peek);
                                chars.next();
                            }
                        }
                        Kind::String(string)
                    }
                    '0'..='9' => {
                        //let mut number = String::from(char);
                        let mut point = false;
                        let mut end = start;
                        while let Some((index, peek)) = chars.peek() {
                            if peek.is_ascii_digit() {
                                //number.push(*peek);
                                chars.next();
                            } else if peek == &'.' {
                                if text.chars().nth(*index + 1) == Some('.') {
                                    break;
                                }
                                point = true;
                                //number.push(*peek);
                                chars.next();
                            } else {
                                end = *index;
                                break;
                            }
                        }
                        let number = text[start..end].to_owned();
                        if point {
                            Kind::Float(number)
                        } else {
                            Kind::Integer(number)
                        }
                    }
                    'A'..='Z' | 'a'..='z' | '_' => {
                        let mut end = start;
                        while let Some((index, peek)) = chars.peek() {
                            if peek.is_ascii_alphanumeric() || peek == &'_' {
                                chars.next();
                            } else {
                                end = *index;
                                break;
                            }
                        }
                        match &text[start..end] {
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
                        }
                    }
                    _ => Kind::Illegal(text[start..start + 1].to_owned()),
                };
                let end = start + kind.literal().len();
                let span = Span { start, end };
                tokens.push(Token { kind, span });
            }
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
    //assert_eq!(expect.len(), tokens.len());
    for (i, (kind, literal)) in expect.into_iter().enumerate() {
        let token = tokens.get(i).unwrap();
        println!("{}, {}, {}", i, kind, literal);
        assert_eq!(kind, token.kind);
        assert_eq!(literal, token.kind.literal());
    }
}

#[test]
fn test_segment_string_escapes() {
    let tokens = Lexer::new().segment(r#""say \"hi\"\\path\n\t\(name)""#);
    assert_eq!(
        tokens[0].kind,
        Kind::String(r#"say \"hi\"\\path\n\t\(name)"#.to_owned())
    );
    assert_eq!(tokens[0].kind.literal(), r#"say \"hi\"\\path\n\t\(name)"#);
}

#[test]
fn test_segment_token_locations() {
    let tokens = Lexer::new().segment("let x = 1;\n  test call {}");
    assert_eq!((tokens[0].span.start, tokens[0].span.end), (0, 3));
    assert_eq!((tokens[5].span.start, tokens[5].span.end), (13, 17));
}

#[test]
fn test_segment_string_invalid() {
    let tokens = Lexer::new().segment(r#""invalid\q""#);
    assert_eq!(tokens[0].kind, Kind::Illegal(r#"invalid\q"#.to_owned()));
    assert_eq!(tokens[0].kind.literal(), r#"invalid\q"#);
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
