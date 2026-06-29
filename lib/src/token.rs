use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Illegal(String), // illegal token
    Eof,             // end of file
    // literal
    Ident(String),   // add, x, y ...
    Integer(String), // 123456789
    Float(String),   // 3.14159265358979323846264338327950288
    True,            // true
    False,           // false
    String(String),  // "Hello world!"
    // keyword
    Function, // fn
    Let,      // let
    If,       // if
    Else,     // else
    Test,     // test
    Break,    // break
    Continue, // continue
    Loop,     // loop
    While,    // while
    For,      // for
    In,       // in
    Client,   // client
    // delimiter
    Assign, // =
    Comma,  // ,
    Semi,   // ;
    Colon,  // :
    Dot,    // .
    Open,   // ..
    Close,  // ..=
    // operator
    Add, // +
    Sub, // -
    Mul, // *
    Div, // /
    Rem, // %
    Not, // !
    Bx,  // ^
    Bo,  // |
    Ba,  // &
    Sl,  // <<
    Sr,  // >>
    Lo,  // ||
    La,  // &&
    Lt,  // <
    Gt,  // >
    Le,  // <=
    Ge,  // >=
    Eq,  // ==
    Ne,  // !=
    // couple
    Lp, // (
    Rp, // )
    Lb, // {
    Rb, // }
    Ls, // [
    Rs, // ]
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: Kind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: Kind, span: Span) -> Token {
        Token { kind, span }
    }

    pub fn precedence(&self) -> u8 {
        match self.kind {
            Kind::Lo => 1,    // a || b
            Kind::Open => 1,  // a..b
            Kind::Close => 1, // a..=b
            Kind::La => 2,    // a && b
            Kind::Bo => 3,    // a | b
            Kind::Bx => 4,    // a ^ b
            Kind::Ba => 5,    // a & b
            Kind::Eq => 6,    // a == b
            Kind::Ne => 6,    // a != b
            Kind::Lt => 7,    // a < b
            Kind::Gt => 7,    // a > b
            Kind::Le => 7,    // a <= b
            Kind::Ge => 7,    // a >= b
            Kind::Sl => 8,    // a << b
            Kind::Sr => 8,    // a >> b
            Kind::Add => 9,   // a + b
            Kind::Sub => 9,   // a - b
            Kind::Mul => 10,  // a * b
            Kind::Div => 10,  // a / b
            Kind::Rem => 10,  // a / b
            // Kind::Sub => 11,  -x unary minus + 2
            Kind::Not => 11, // !x
            Kind::Lp => 12,  // function()
            Kind::Ls => 13,  // array[index]
            Kind::Dot => 13, // left.field
            _ => 0,
        }
    }
}

impl Kind {
    pub fn same(&self, other: &Kind) -> bool {
        std::mem::discriminant(self) == std::mem::discriminant(other)
    }

    pub fn literal(&self) -> &str {
        match self {
            Kind::Illegal(literal)
            | Kind::Ident(literal)
            | Kind::Integer(literal)
            | Kind::Float(literal)
            | Kind::String(literal) => literal,
            Kind::Eof => "💥",
            Kind::True => "true",
            Kind::False => "false",
            Kind::Add => "+",
            Kind::Sub => "-",
            Kind::Mul => "*",
            Kind::Div => "/",
            Kind::Rem => "%",
            Kind::Not => "!",
            Kind::Bx => "^",
            Kind::Bo => "|",
            Kind::Ba => "&",
            Kind::Sl => "<<",
            Kind::Sr => ">>",
            Kind::Lo => "||",
            Kind::La => "&&",
            Kind::Lt => "<",
            Kind::Gt => ">",
            Kind::Le => "<=",
            Kind::Ge => ">=",
            Kind::Eq => "==",
            Kind::Ne => "!=",
            Kind::Assign => "=",
            Kind::Comma => ",",
            Kind::Semi => ";",
            Kind::Colon => ":",
            Kind::Dot => ".",
            Kind::Open => "..",
            Kind::Close => "..=",
            Kind::Lp => "(",
            Kind::Rp => ")",
            Kind::Lb => "{",
            Kind::Rb => "}",
            Kind::Ls => "[",
            Kind::Rs => "]",
            Kind::Function => "fn",
            Kind::Let => "let",
            Kind::If => "if",
            Kind::Else => "else",
            Kind::Test => "test",
            Kind::Break => "break",
            Kind::Continue => "continue",
            Kind::Loop => "loop",
            Kind::While => "while",
            Kind::For => "for",
            Kind::In => "in",
            Kind::Client => "client",
        }
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.kind.literal())
    }
}
