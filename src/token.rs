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
    Lt,     // <
    Gt,     // >
    Le,     // <=
    Ge,     // >=
    Eq,     // ==
    Ne,     // !=
    Bo,     // |
    Ba,     // &
    Lo,     // ||
    La,     // &&

    Dot, // .

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
            Kind::Lo=> 1,     // ||
            Kind::La => 2,     // &&
            Kind::Bo=> 3,     // |
            Kind::Ba=> 4,     // &
            Kind::Eq => 5,    // 8==6
            Kind::Ne => 5,    // 8!=6
            Kind::Lt => 6,    // 8<6
            Kind::Gt => 6,    // 8>6
            Kind::Plus => 7,  // 8+6
            Kind::Minus => 7, // 8-6
            Kind::Star => 8,  // 8*6
            Kind::Slash => 8, // 8/6
            // Kind::Minus => 9,  -X unary minus +2
            Kind::Bang => 9, // !X
            Kind::Lp => 10,   // function()
            Kind::Ls => 11,   // array[index]
            Kind::Dot => 11,  // object.field
            _ => 0,
        }
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.literal)
    }
}
