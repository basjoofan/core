#[derive(Debug, PartialEq, Eq)]
pub enum Opcode {
    Constant(usize),
    Add,
}
