mod code;
mod compiler;
mod http;
mod lexer;
mod machine;
mod native;
mod parser;
mod record;
mod stat;
mod symbol;
mod syntax;
mod token;
mod value;
mod writer;

use code::Opcode;
use compiler::Compiler;
use machine::Machine;
use parser::Parser;
use symbol::Symbol;
use symbol::Symbols;
use syntax::Expr;
use token::Kind;
use token::Token;
use value::Value;

pub mod command;

#[macro_export]
macro_rules! join {
    ($ident: ident, $format: literal, $separator:literal) => {
        $ident
            .iter()
            .map(|e| format!($format, e))
            .collect::<Vec<String>>()
            .join($separator)
    };
    ($ident: ident, $format: literal, $middle:literal, $separator:literal) => {
        $ident
            .iter()
            .map(|(k, v)| format!(concat!($format, $middle, $format), k, v))
            .collect::<Vec<String>>()
            .join($separator)
    };
}
