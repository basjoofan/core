use crate::Expr;
use crate::Kind;
use crate::Opcode;
use crate::Value;

pub struct Compiler {
    pub instructions: Vec<Opcode>,
    pub consts: Vec<Value>,
}

impl Compiler {
    pub fn new() -> Self {
        Self {
            instructions: Vec::new(),
            consts: Vec::new(),
        }
    }

    fn emit(&mut self, opcode: Opcode) -> usize {
        self.instructions.push(opcode);
        self.instructions.len() - 1
    }

    fn save(&mut self, value: Value) -> usize {
        self.consts.push(value);
        self.consts.len() - 1
    }

    pub fn compile(&mut self, source: &Vec<Expr>) -> Result<(), String> {
        for expr in source.iter() {
            self.compile_expr(expr)?;
            self.emit(Opcode::Pop);
        }
        Ok(())
    }

    fn compile_expr(&mut self, expr: &Expr) -> Result<(), String> {
        match expr {
            Expr::Ident(_, _) => todo!(),
            Expr::Integer(_, value) => {
                let integer = Value::Integer(*value);
                let index = self.save(integer);
                self.emit(Opcode::Const(index));
            }
            Expr::Float(_, _) => todo!(),
            Expr::Boolean(_, _) => todo!(),
            Expr::String(_, _) => todo!(),
            Expr::Let(_, _, _) => todo!(),
            Expr::Return(_, _) => todo!(),
            Expr::Unary(_, _) => todo!(),
            Expr::Binary(token, left, right) => {
                self.compile_expr(left)?;
                self.compile_expr(right)?;
                match token.kind {
                    Kind::Plus => self.emit(Opcode::Add),
                    Kind::Minus => self.emit(Opcode::Sub),
                    Kind::Star => self.emit(Opcode::Mul),
                    Kind::Slash => self.emit(Opcode::Div),
                    _ => Err(format!("Unknown operator: {}", token))?,
                };
            }
            Expr::Paren(_, value) => self.compile_expr(value)?,
            Expr::If(_, _, _, _) => todo!(),
            Expr::Function(_, _, _) => todo!(),
            Expr::Call(_, _, _) => todo!(),
            Expr::Array(_, _) => todo!(),
            Expr::Map(_, _) => todo!(),
            Expr::Index(_, _, _) => todo!(),
            Expr::Field(_, _, _) => todo!(),
            Expr::Request(_, _, _, _) => todo!(),
            Expr::Test(_, _, _) => todo!(),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::Compiler;
    use crate::Opcode;
    use crate::Value;

    fn run_compiler_tests(tests: Vec<(&str, Vec<Value>, Vec<Opcode>)>) {
        for (text, consts, instructions) in tests {
            let source = crate::parser::Parser::new(text).parse().unwrap();
            let mut compiler = Compiler::new();
            let result = compiler.compile(&source);
            assert!(result.is_ok(), "compile error: {}", result.unwrap_err());
            assert_eq!(compiler.instructions, instructions);
            assert_eq!(compiler.consts, consts);
        }
    }

    #[test]
    fn test_integer_arithmetic() {
        let tests = vec![
            (
                "1 + 2",
                vec![Value::Integer(1), Value::Integer(2)],
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Add, Opcode::Pop],
            ),
            (
                "1; 2",
                vec![Value::Integer(1), Value::Integer(2)],
                vec![Opcode::Const(0), Opcode::Pop, Opcode::Const(1), Opcode::Pop],
            ),
            (
                "1 - 2",
                vec![Value::Integer(1), Value::Integer(2)],
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Sub, Opcode::Pop],
            ),
            (
                "1 * 2",
                vec![Value::Integer(1), Value::Integer(2)],
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Mul, Opcode::Pop],
            ),
            (
                "2 / 1",
                vec![Value::Integer(2), Value::Integer(1)],
                vec![Opcode::Const(0), Opcode::Const(1), Opcode::Div, Opcode::Pop],
            ),
        ];
        run_compiler_tests(tests);
    }
}
