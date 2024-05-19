mod evaluator;
mod lexer;
mod native;
mod parser;
mod record;
mod request;
mod syntax;
mod token;
mod value;
pub mod command;

pub struct May<'a, T>(pub &'a Option<T>);

impl<'a, T: std::fmt::Display> std::fmt::Display for May<'a, T> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self.0 {
            Some(ref t) => write!(f, "{}", t),
            None => write!(f, "?"),
        }
    }
}

pub fn add(left: usize, right: usize) -> usize {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
