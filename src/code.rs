#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Opcode {
    Const(usize),
    Pop,

    Add,
    Sub,
    Mul,
    Div,

    True,
    False,

    Lt,
    Gt,
    Eq,
    Ne,

    Minus,
    Bang,

}
