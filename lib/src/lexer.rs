use std::char;

use super::Kind;
use super::Span;
use super::Token;
use std::collections::HashMap;

pub struct Lexer;

impl Lexer {
    pub fn new() -> Self {
        Self
    }

    #[cfg(test)]
    pub fn segment(&mut self, text: &str) -> Vec<Token> {
        self.segment_with_string_ends(text, &HashMap::new())
    }

    pub(crate) fn segment_with_string_ends(
        &mut self,
        text: &str,
        string_ends: &HashMap<usize, usize>,
    ) -> Vec<Token> {
        let bytes = text.as_bytes();
        let mut tokens = Vec::new();
        let mut index = 0;

        while index < bytes.len() {
            // skip blanks
            if bytes[index].is_ascii_whitespace() {
                index += 1;
                continue;
            }
            // skip comments
            if bytes[index] == b'/' && bytes.get(index + 1) == Some(&b'/') {
                index += 2;
                while index < bytes.len() && bytes[index] != b'\n' {
                    index += 1;
                }
                continue;
            }
            // segment token
            let start = index;
            let (kind, lite, end) = match bytes[index] {
                b'@' => (Kind::Tag, "@".to_owned(), index),
                b'=' if bytes.get(index + 1) == Some(&b'=') => {
                    (Kind::Eq, "==".to_owned(), index + 1)
                }
                b'=' => (Kind::Assign, "=".to_owned(), index),
                b'!' if bytes.get(index + 1) == Some(&b'=') => {
                    (Kind::Ne, "!=".to_owned(), index + 1)
                }
                b'!' => (Kind::Not, "!".to_owned(), index),
                b'<' if bytes.get(index + 1) == Some(&b'=') => {
                    (Kind::Le, "<=".to_owned(), index + 1)
                }
                b'>' if bytes.get(index + 1) == Some(&b'=') => {
                    (Kind::Ge, ">=".to_owned(), index + 1)
                }
                b'<' if bytes.get(index + 1) == Some(&b'<') => {
                    (Kind::Sl, "<<".to_owned(), index + 1)
                }
                b'>' if bytes.get(index + 1) == Some(&b'>') => {
                    (Kind::Sr, ">>".to_owned(), index + 1)
                }
                b'<' => (Kind::Lt, "<".to_owned(), index),
                b'>' => (Kind::Gt, ">".to_owned(), index),
                b'&' if bytes.get(index + 1) == Some(&b'&') => {
                    (Kind::La, "&&".to_owned(), index + 1)
                }
                b'|' if bytes.get(index + 1) == Some(&b'|') => {
                    (Kind::Lo, "||".to_owned(), index + 1)
                }
                b'+' => (Kind::Add, "+".to_owned(), index),
                b'-' => (Kind::Sub, "-".to_owned(), index),
                b'*' => (Kind::Mul, "*".to_owned(), index),
                b'/' => (Kind::Div, "/".to_owned(), index),
                b'%' => (Kind::Rem, "%".to_owned(), index),
                b'^' => (Kind::Bx, "^".to_owned(), index),
                b'|' => (Kind::Bo, "|".to_owned(), index),
                b'&' => (Kind::Ba, "&".to_owned(), index),
                b',' => (Kind::Comma, ",".to_owned(), index),
                b';' => (Kind::Semi, ";".to_owned(), index),
                b':' => (Kind::Colon, ":".to_owned(), index),
                b'.' => (Kind::Dot, ".".to_owned(), index),
                b'(' => (Kind::Lp, "(".to_owned(), index),
                b')' => (Kind::Rp, ")".to_owned(), index),
                b'{' => (Kind::Lb, "{".to_owned(), index),
                b'}' => (Kind::Rb, "}".to_owned(), index),
                b'[' => (Kind::Ls, "[".to_owned(), index),
                b']' => (Kind::Rs, "]".to_owned(), index),
                b'0'..=b'9' => {
                    index += 1;
                    while bytes.get(index).is_some_and(u8::is_ascii_digit) {
                        index += 1;
                    }
                    if bytes.get(index) == Some(&b'.')
                        && bytes.get(index + 1).is_some_and(u8::is_ascii_digit)
                    {
                        index += 1;
                        while bytes.get(index).is_some_and(u8::is_ascii_digit) {
                            index += 1;
                        }
                        (Kind::Float, text[start..index].to_owned(), index - 1)
                    } else {
                        (Kind::Integer, text[start..index].to_owned(), index - 1)
                    }
                }
                b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                    index += 1;
                    while bytes
                        .get(index)
                        .is_some_and(|&byte| byte.is_ascii_alphanumeric() || byte == b'_')
                    {
                        index += 1;
                    }
                    let word = &text[start..index];
                    let kind = match word {
                        "env" => Kind::Env,
                        "api" => Kind::Api,
                        "let" => Kind::Let,
                        "test" => Kind::Test,
                        "expect" => Kind::Expect,
                        "true" => Kind::True,
                        "false" => Kind::False,
                        "null" => Kind::Null,
                        _ => Kind::Ident,
                    };
                    (kind, word.to_owned(), index - 1)
                }
                b'"' => {
                    let string_end = string_ends.get(&start).copied();
                    index += 1;
                    let mut value = String::new();
                    while let Some(&byte) = bytes.get(index) {
                        if byte == b'"' && string_end.is_none_or(|end| index == end) {
                            break;
                        }
                        if byte != b'\\' {
                            let Some(character) =
                                text.get(index..).and_then(|rest| rest.chars().next())
                            else {
                                break;
                            };
                            value.push(character);
                            index += character.len_utf8();
                            continue;
                        }
                        index += 1;
                        let Some(&escaped) = bytes.get(index) else {
                            break;
                        };
                        match escaped {
                            b'"' => value.push('"'),
                            b'\\' => value.push('\\'),
                            b'n' => value.push('\n'),
                            b'r' => value.push('\r'),
                            b't' => value.push('\t'),
                            b'0' => value.push('\0'),
                            _ => {
                                value.push('\\');
                                value.push(escaped as char);
                            }
                        }
                        index += 1;
                    }
                    if bytes.get(index) == Some(&b'"') {
                        (Kind::String, value, index)
                    } else {
                        (Kind::Illegal, string!(text, start..), bytes.len() - 1)
                    }
                }
                b'`' => {
                    index += 1;
                    let mut value = String::new();
                    while let Some(&byte) = bytes.get(index) {
                        if byte == b'`' {
                            break;
                        }
                        if byte == b'\\' && bytes.get(index + 1) == Some(&b'`') {
                            value.push('`');
                            index += 2;
                            continue;
                        }
                        let Some(character) =
                            text.get(index..).and_then(|rest| rest.chars().next())
                        else {
                            break;
                        };
                        value.push(character);
                        index += character.len_utf8();
                    }
                    if bytes.get(index) == Some(&b'`') {
                        (Kind::Raw, dedent_raw(&value), index)
                    } else {
                        (Kind::Illegal, string!(text, start..), bytes.len() - 1)
                    }
                }
                _ => match text.get(index..).and_then(|rest| rest.chars().next()) {
                    Some(char) => {
                        let end = index + char.len_utf8() - 1;
                        (Kind::Illegal, char.to_string(), end)
                    }
                    None => (Kind::Illegal, String::new(), index),
                },
            };
            tokens.push(Token::new(kind, Span { start, end }, lite));
            index = end + 1;
        }
        tokens
    }
}

fn dedent_raw(value: &str) -> String {
    if !value.contains('\n') {
        return value.to_owned();
    }
    let mut lines = value.lines().collect::<Vec<_>>();
    if lines.first().is_some_and(|line| line.trim().is_empty()) {
        lines.remove(0);
    }
    if lines.last().is_some_and(|line| line.trim().is_empty()) {
        lines.pop();
    }
    let indent = lines
        .iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.len() - line.trim_start_matches([' ', '\t']).len())
        .min()
        .unwrap_or(0);
    lines
        .into_iter()
        .map(|line| line.get(indent..).unwrap_or(""))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair(token: &Token) -> (Kind, String) {
        (token.kind.clone(), token.lite.clone())
    }

    fn lex(text: &str) -> Vec<(Kind, String)> {
        Lexer::new().segment(text).iter().map(pair).collect()
    }

    fn owned(values: Vec<(Kind, &str)>) -> Vec<(Kind, String)> {
        values
            .into_iter()
            .map(|(kind, text)| (kind, text.to_owned()))
            .collect()
    }

    #[test]
    fn lexer_segments_literals_and_keywords() {
        assert_eq!(
            lex("env api let test expect true false null name 42 3.14 \"text\" `raw`"),
            owned(vec![
                (Kind::Env, "env"),
                (Kind::Api, "api"),
                (Kind::Let, "let"),
                (Kind::Test, "test"),
                (Kind::Expect, "expect"),
                (Kind::True, "true"),
                (Kind::False, "false"),
                (Kind::Null, "null"),
                (Kind::Ident, "name"),
                (Kind::Integer, "42"),
                (Kind::Float, "3.14"),
                (Kind::String, "text"),
                (Kind::Raw, "raw"),
            ])
        );
    }

    #[test]
    fn lexer_segments_delimiters_and_operators() {
        assert_eq!(
            lex("= , ; : . @ ( ) { } [ ] + - * / % ! ^ | & << >> || && < > <= >= == !="),
            owned(vec![
                (Kind::Assign, "="),
                (Kind::Comma, ","),
                (Kind::Semi, ";"),
                (Kind::Colon, ":"),
                (Kind::Dot, "."),
                (Kind::Tag, "@"),
                (Kind::Lp, "("),
                (Kind::Rp, ")"),
                (Kind::Lb, "{"),
                (Kind::Rb, "}"),
                (Kind::Ls, "["),
                (Kind::Rs, "]"),
                (Kind::Add, "+"),
                (Kind::Sub, "-"),
                (Kind::Mul, "*"),
                (Kind::Div, "/"),
                (Kind::Rem, "%"),
                (Kind::Not, "!"),
                (Kind::Bx, "^"),
                (Kind::Bo, "|"),
                (Kind::Ba, "&"),
                (Kind::Sl, "<<"),
                (Kind::Sr, ">>"),
                (Kind::Lo, "||"),
                (Kind::La, "&&"),
                (Kind::Lt, "<"),
                (Kind::Gt, ">"),
                (Kind::Le, "<="),
                (Kind::Ge, ">="),
                (Kind::Eq, "=="),
                (Kind::Ne, "!="),
            ])
        );
    }

    #[test]
    fn lexer_handles_v1_representative_tokens() {
        let text = r#""Content-Type": "application/json", created.json.id != null; `\n  <id>\(id)</id>\n`"#;
        let actual = lex(text);
        assert_eq!(
            actual,
            owned(vec![
                (Kind::String, "Content-Type"),
                (Kind::Colon, ":"),
                (Kind::String, "application/json"),
                (Kind::Comma, ","),
                (Kind::Ident, "created"),
                (Kind::Dot, "."),
                (Kind::Ident, "json"),
                (Kind::Dot, "."),
                (Kind::Ident, "id"),
                (Kind::Ne, "!="),
                (Kind::Null, "null"),
                (Kind::Semi, ";"),
                (Kind::Raw, "\\n  <id>\\(id)</id>\\n"),
            ])
        );
    }

    #[test]
    fn lexer_skips_comments_and_decodes_string_escapes() {
        let actual = lex("// ignored\n\"a\\\"b\"");
        assert_eq!(actual, owned(vec![(Kind::String, "a\"b")]));
    }

    #[test]
    fn lexer_decodes_string_escapes_but_keeps_raw_source() {
        let tokens = Lexer::new().segment(r#""line\n\(token)" `line\n\(token)`"#);
        assert_eq!(tokens[0].kind, Kind::String);
        assert_eq!(tokens[0].lite, "line\n\\(token)");
        assert_eq!(tokens[1].kind, Kind::Raw);
        assert_eq!(tokens[1].lite, r#"line\n\(token)"#);
    }

    #[test]
    fn lexer_allows_escaped_backtick_in_raw_string() {
        assert_eq!(lex(r"`a\` b`"), owned(vec![(Kind::Raw, "a` b")]));
    }

    #[test]
    fn lexer_removes_common_raw_string_indentation() {
        assert_eq!(
            lex("`\n    first\n      second\n    third\n`"),
            owned(vec![(Kind::Raw, "first\n  second\nthird")])
        );
    }

    #[test]
    fn lexer_marks_unterminated_literals_illegal() {
        for text in ["\"half", "`half"] {
            let tokens = Lexer::new().segment(text);
            assert_eq!(tokens.len(), 1);
            assert_eq!(tokens[0].kind, Kind::Illegal);
            assert_eq!(
                tokens[0].span,
                Span {
                    start: 0,
                    end: text.len() - 1
                }
            );
        }
    }

    #[test]
    fn lexer_tracks_utf8_byte_spans_and_illegal_characters() {
        let tokens = Lexer::new().segment("name 🍀 next");
        assert_eq!(tokens[0].span, Span { start: 0, end: 3 });
        assert_eq!(
            tokens[1],
            Token::new(Kind::Illegal, Span { start: 5, end: 8 }, "🍀".into())
        );
        assert_eq!(tokens[2].span, Span { start: 10, end: 13 });
    }

    #[test]
    fn lexer_treats_hyphen_as_subtraction_without_whitespace() {
        assert_eq!(
            lex("left-right"),
            owned(vec![
                (Kind::Ident, "left"),
                (Kind::Sub, "-"),
                (Kind::Ident, "right"),
            ])
        );
    }
}
