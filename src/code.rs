#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Opcode {
    None,
    Const(usize),
    Pop,
    True,
    False,

    Neg,
    Not,

    Add,
    Sub,
    Mul,
    Div,
    Rem,

    Lt,
    Gt,
    Le,
    Ge,
    Eq,
    Ne,
    Bx,
    Ba,
    Bo,

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

    Native(isize),

    Closure(usize, usize),
    GetFree(usize),
    Current,

    Field,
}
