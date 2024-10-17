use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Kind {
    Illegal, // illegal token
    Eof,     // end of file

    // ident + literal
    Ident,    // add, foobar, x, y, ...
    Integer,  // 56789
    Float,    // 3.14159265358979323846264338327950288
    True,     // true
    False,    // false
    String,   // "foobar"
    Template, // `GET http://example.com`

    // operator
    Assign, // =
    Bang,   // !
    Plus,   // +
    Minus,  // -
    Star,   // *
    Slash,  // /
    Dot,    // .

    Lt, // <
    Gt, // >
    Eq, // ==
    Ne, // !=

    // delimiter
    Comma, // ,
    Semi,  // ;
    Colon, // :

    // couple
    Lp, // (
    Rp, // )
    Lb, // {
    Rb, // }
    Ls, // [
    Rs, // ]

    // keyword
    Fn,     // fn
    Rq,     // rq
    Let,    // let
    If,     // if
    Else,   // else
    Return, // return
    Test,   // test
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: Kind,
    pub literal: String,
}

impl Token {
    pub fn new(kind: Kind, literal: String) -> Token {
        Token { kind, literal }
    }

    pub fn precedence(&self) -> u8 {
        match self.kind {
            Kind::Eq => 2,    // 8==6
            Kind::Ne => 2,    // 8!=6
            Kind::Lt => 3,    // 8<6
            Kind::Gt => 3,    // 8>6
            Kind::Plus => 4,  // 8+6
            Kind::Minus => 4, // 8-6
            Kind::Star => 5,  // 8*6
            Kind::Slash => 5, // 8/6
            // Kind::Minus => 6,  -X unary minus +2
            Kind::Bang => 6,   // !X
            Kind::Lp => 7,     // function()
            Kind::Ls => 8,     // array[index]
            Kind::Dot => 8,    // object.field
            _ => 0,
        }
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.literal)
    }
}
