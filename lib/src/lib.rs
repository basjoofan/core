macro_rules! string {
    ($text:expr, $range:expr) => {
        $text[$range].to_owned()
    };
}

pub mod api;
mod lexer;
pub mod mech;
mod native;
mod parser;
mod stat;
mod syntax;
mod token;
mod trans;
mod value;

pub use syntax::Expr;
use token::Kind;
use token::Span;
use token::Token;
pub use value::Value;

pub use mech::{Mech, Report, Trans};
pub use native::{Function, Output, Registry};
pub use parser::Parser;
pub use stat::Stats;
pub use syntax::Source;
pub use trans::Content;
pub use trans::Header;
pub use trans::Pending;
pub use trans::Request;
pub use trans::Response;
pub use trans::Result;
pub use trans::Timing;
