use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Kind {
    Illegal, // illegal token
    // literal
    Ident,   // add, x, y ...
    Integer, // 123456789
    Float,   // 3.14159265358979323846264338327950288
    True,    // true
    False,   // false
    String,  // "Hello world!"
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
    pub lite: String,
}

impl Token {
    pub fn new(kind: Kind, span: Span, lite: String) -> Token {
        Token { kind, span, lite }
    }

    pub fn lite(&self) -> &str {
        self.lite.as_str()
    }

    pub fn rule(&self) -> u8 {
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

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.lite())
    }
}
