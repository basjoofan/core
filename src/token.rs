use std::fmt::Display;
use std::fmt::Formatter;
use std::fmt::Result;

#[derive(Debug, Clone, PartialEq, Eq)]
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

#[derive(Clone, PartialEq, Eq)]
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
            Kind::Eq => EQUALS,
            Kind::Ne => EQUALS,
            Kind::Lt => COMPARE,
            Kind::Gt => COMPARE,
            Kind::Plus => ADD_SUB,
            Kind::Minus => ADD_SUB,
            Kind::Star => MUL_DIV,
            Kind::Slash => MUL_DIV,
            // Kind::Minus | Kind::Bang  => UNARY
            Kind::Lp => CALL,
            Kind::Ls => SELECT,
            Kind::Dot => SELECT,
            // Kind::Let | Kind::Return  => STMT
            _ => LOWEST,
        }
    }
}

impl Display for Token {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}", self.literal)
    }
}

pub const LOWEST: u8 = 1;
const EQUALS: u8 = 2; // ==
const COMPARE: u8 = 3; // > or <
const ADD_SUB: u8 = 4; // + or -
const MUL_DIV: u8 = 5; // * or /
pub const UNARY: u8 = 6; // -X or !X
const CALL: u8 = 7; // myFunction(X)
const SELECT: u8 = 8; // array[index] or object.field
pub const STMT: u8 = 9; // let or return
