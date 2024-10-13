#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Opcode {
    Constant(usize),
    Pop,
    Add,
}
