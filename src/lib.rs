macro_rules! not_wasm {
    ($($item:item)*) => {$(
        #[cfg(not(target_arch = "wasm32"))]
        $item
    )*}
}

mod lexer;
mod parser;
mod syntax;
mod token;
mod value;

pub use parser::Parser;
pub use parser::Source;

use syntax::Expr;
use token::Kind;
use token::Token;
use value::Value;

not_wasm! {
    mod evaluator;
    mod http;
    mod native;
    mod stat;
    mod writer;
    pub mod command;
    use stat::Stats;
    use writer::Assert;
    use writer::Record;
    use writer::Records;
    use writer::Writer;
    use evaluator::Context;
}
