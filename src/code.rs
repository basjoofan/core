#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Opcode {
    None,
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

    Judge(usize),
    Jump(usize),

    GetGlobal(usize),
    SetGlobal(usize),

    Array(usize),
    Map(usize),
    Index,

    Return,
    Call(usize),

    GetLocal(usize),
    SetLocal(usize),
}
