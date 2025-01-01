use crate::native;
use crate::Context;
use crate::Expr;
use crate::Kind;
use crate::Token;
use crate::Value;
use std::collections::HashMap;

pub fn eval_expr(expr: &Expr, context: &mut Context) -> Result<Value, String> {
    match expr {
        Expr::Ident(ident) => eval_ident_expr(ident, context),
        Expr::Let(name, expr) => eval_let_expr(name, expr, context),
        Expr::Integer(integer) => eval_integer_literal(integer),
        Expr::Float(float) => eval_float_literal(float),
        Expr::Boolean(boolean) => eval_boolean_literal(boolean),
        Expr::String(string) => eval_string_literal(string),
        Expr::Return(..) => todo!(),
        Expr::Unary(token, right) => eval_unary_expr(token, right, context),
        Expr::Binary(token, left, right) => eval_binary_expr(token, left, right, context),
        Expr::Paren(expr) => eval_expr(expr, context),
        Expr::If(condition, consequence, alternative) => eval_if_expr(condition, consequence, alternative, context),
        Expr::Function(..) => todo!(),
        Expr::Call(function, arguments) => eval_call_expr(function, arguments, context),
        Expr::Array(elements) => eval_array_literal(elements, context),
        Expr::Map(pairs) => eval_map_literal(pairs, context),
        Expr::Index(value, index) => eval_index_expr(value, index, context),
        Expr::Field(map, field) => eval_field_expr(map, field, context),
        Expr::Request(..) => todo!(),
        Expr::Test(..) => todo!(),
    }
}

fn eval_ident_expr(ident: &String, context: &mut Context) -> Result<Value, String> {
    match context.get(ident) {
        Some(value) => Ok(value),
        None => Err(format!("ident:{} not found", ident)),
    }
}

fn eval_let_expr(name: &String, expr: &Box<Expr>, context: &mut Context) -> Result<Value, String> {
    let value = eval_expr(expr, context)?;
    context.set(name, value.to_owned());
    Ok(value)
}

fn eval_integer_literal(integer: &i64) -> Result<Value, String> {
    Ok(Value::Integer(*integer))
}

fn eval_float_literal(float: &f64) -> Result<Value, String> {
    Ok(Value::Float(*float))
}

fn eval_boolean_literal(boolean: &bool) -> Result<Value, String> {
    Ok(Value::Boolean(*boolean))
}

fn eval_string_literal(string: &String) -> Result<Value, String> {
    Ok(Value::String(string.to_owned()))
}

fn eval_unary_expr(token: &Token, right: &Box<Expr>, context: &mut Context) -> Result<Value, String> {
    let right = eval_expr(right, context)?;
    match (token.kind, right) {
        (Kind::Not, Value::Boolean(false)) | (Kind::Not, Value::None) => Ok(Value::Boolean(true)),
        (Kind::Not, Value::Integer(integer)) => Ok(Value::Integer(!integer)),
        (Kind::Not, _) => Ok(Value::Boolean(false)),
        (Kind::Sub, Value::Integer(integer)) => Ok(Value::Integer(-integer)),
        (Kind::Sub, Value::Float(float)) => Ok(Value::Float(-float)),
        (_, right) => Err(format!("unknown operator: {}{:?}", token, right)),
    }
}

fn eval_binary_expr(token: &Token, left: &Box<Expr>, right: &Box<Expr>, context: &mut Context) -> Result<Value, String> {
    match token.kind {
        Kind::Add => Ok(eval_expr(left, context)? + eval_expr(right, context)?),
        Kind::Sub => Ok(eval_expr(left, context)? - eval_expr(right, context)?),
        Kind::Mul => Ok(eval_expr(left, context)? * eval_expr(right, context)?),
        Kind::Div => Ok(eval_expr(left, context)? / eval_expr(right, context)?),
        Kind::Rem => Ok(eval_expr(left, context)? % eval_expr(right, context)?),
        Kind::Bx => Ok(eval_expr(left, context)? ^ eval_expr(right, context)?),
        Kind::Bo => Ok(eval_expr(left, context)? | eval_expr(right, context)?),
        Kind::Ba => Ok(eval_expr(left, context)? & eval_expr(right, context)?),
        Kind::Sl => Ok(eval_expr(left, context)? << eval_expr(right, context)?),
        Kind::Sr => Ok(eval_expr(left, context)? >> eval_expr(right, context)?),
        Kind::Lo => match eval_expr(left, context)? {
            Value::Boolean(false) | Value::None => eval_expr(right, context),
            left => Ok(left),
        },
        Kind::La => match eval_expr(left, context)? {
            left @ (Value::Boolean(false) | Value::None) => Ok(left),
            _ => eval_expr(right, context),
        },
        Kind::Lt => Ok(Value::Boolean(eval_expr(left, context)? < eval_expr(right, context)?)),
        Kind::Gt => Ok(Value::Boolean(eval_expr(left, context)? > eval_expr(right, context)?)),
        Kind::Le => Ok(Value::Boolean(eval_expr(left, context)? <= eval_expr(right, context)?)),
        Kind::Ge => Ok(Value::Boolean(eval_expr(left, context)? >= eval_expr(right, context)?)),
        Kind::Eq => Ok(Value::Boolean(eval_expr(left, context)? == eval_expr(right, context)?)),
        Kind::Ne => Ok(Value::Boolean(eval_expr(left, context)? != eval_expr(right, context)?)),
        _ => Err(format!("not support operator: {} {} {}", left, token, right)),
    }
}

fn eval_if_expr(condition: &Box<Expr>, consequence: &[Expr], alternative: &[Expr], context: &mut Context) -> Result<Value, String> {
    let condition = eval_expr(condition, context)?;
    match condition {
        Value::Boolean(false) | Value::None => eval_block_expr(alternative, context),
        _ => eval_block_expr(consequence, context),
    }
}

fn eval_call_expr(function: &str, arguments: &[Expr], context: &mut Context) -> Result<Value, String> {
    let arguments = eval_exprs(arguments, context)?;
    match function {
        "println" => Ok(native::println(arguments)),
        "print" => Ok(native::print(arguments)),
        "format" => Ok(native::format(arguments)),
        "length" => Ok(native::length(arguments)),
        "append" => Ok(native::append(arguments)),
        _ => Err(format!("function {} not found", function)),
    }
}

fn eval_array_literal(elements: &[Expr], context: &mut Context) -> Result<Value, String> {
    Ok(Value::Array(eval_exprs(elements, context)?))
}

fn eval_map_literal(pairs: &Vec<(Expr, Expr)>, context: &mut Context) -> Result<Value, String> {
    let mut map = HashMap::new();
    for (key, value) in pairs {
        let key = eval_expr(key, context)?;
        let value = eval_expr(value, context)?;
        map.insert(key.to_string(), value);
    }
    Ok(Value::Map(map))
}

fn eval_index_expr(value: &Box<Expr>, index: &Box<Expr>, context: &mut Context) -> Result<Value, String> {
    // TODO enhance indent expr get variable use reference
    let value = eval_expr(value, context)?;
    let index = eval_expr(index, context)?;
    match (value, index) {
        (Value::Array(mut elements), Value::Integer(index)) => {
            let index = index as usize;
            if index < elements.len() {
                let element = elements.remove(index);
                Ok(element)
            } else {
                Ok(Value::None)
            }
        }
        (Value::Map(mut pairs), key) => {
            let element = pairs.remove(&key.to_string());
            match element {
                Some(element) => Ok(element),
                None => Ok(Value::None),
            }
        }
        (value, _) => Err(format!("index operator not support: {:?}", value)),
    }
}

fn eval_field_expr(map: &Box<Expr>, field: &String, context: &mut Context) -> Result<Value, String> {
    // TODO enhance indent expr get variable use reference
    match eval_expr(map, context)? {
        Value::Map(mut pairs) => {
            let value = pairs.remove(field);
            Ok(match value {
                Some(value) => value,
                None => Value::None,
            })
        }
        map => Err(format!("field operator not support: {:?}", map)),
    }
}

pub fn eval_block_expr(exprs: &[Expr], context: &mut Context) -> Result<Value, String> {
    let mut result = Value::None;
    for expr in exprs {
        result = eval_expr(expr, context)?;
    }
    Ok(result)
}

fn eval_exprs(elements: &[Expr], context: &mut Context) -> Result<Vec<Value>, String> {
    let mut values = Vec::with_capacity(elements.len());
    for element in elements {
        values.push(eval_expr(element, context)?);
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::eval_block_expr;
    use crate::Context;
    use crate::Parser;
    use crate::Value;
    use std::collections::HashMap;

    fn run_eval_tests(tests: Vec<(&str, Value)>) {
        for (text, expect) in tests {
            let source = Parser::new(text).parse().unwrap();
            let mut context = Context::default();
            match eval_block_expr(&source, &mut context) {
                Ok(value) => {
                    println!("{:?} => {} = {}", source, value, expect);
                    assert_eq!(value, expect);
                }
                Err(message) => panic!("machine error: {}", message),
            }
        }
    }

    #[test]
    fn test_let_expr() {
        let tests = vec![
            ("let one = 1; one", Value::Integer(1)),
            ("let one = 1; let two = 2; one + two", Value::Integer(3)),
            ("let one = 1; let two = one + one; one + two", Value::Integer(3)),
            ("let one = 1; one;", Value::Integer(1)),
            ("let one = 1; let two = 2; one + two;", Value::Integer(3)),
            ("let one = 1; let two = one + one; one + two;", Value::Integer(3)),
            ("let one = 1;let one = 2;", Value::Integer(2)),
            ("let one = 1;let one = 2;one", Value::Integer(2)),
            ("let one = 1;let two = 2;let one = 3;one", Value::Integer(3)),
        ];
        run_eval_tests(tests);
    }

    #[test]
    fn test_integer_arithmetic() {
        let tests = vec![
            ("1", Value::Integer(1)),
            ("2", Value::Integer(2)),
            ("1 + 2", Value::Integer(3)),
            ("1 - 2", Value::Integer(-1)),
            ("1 * 2", Value::Integer(2)),
            ("4 / 2", Value::Integer(2)),
            ("7 % 3", Value::Integer(1)),
            ("50 / 2 * 2 + 10 - 5", Value::Integer(55)),
            ("5 * (2 + 10)", Value::Integer(60)),
            ("5 + 5 + 5 + 5 - 10", Value::Integer(10)),
            ("2 * 2 * 2 * 2 * 2", Value::Integer(32)),
            ("5 * 2 + 10", Value::Integer(20)),
            ("5 + 2 * 10", Value::Integer(25)),
            ("5 * (2 + 10)", Value::Integer(60)),
            ("-5", Value::Integer(-5)),
            ("-10", Value::Integer(-10)),
            ("-50 + 100 + -50", Value::Integer(0)),
            ("(5 + 10 * 2 + 15 / 3) * 2 + -10", Value::Integer(50)),
            ("!5", Value::Integer(-6)),
            ("!!5", Value::Integer(5)),
            ("!-3", Value::Integer(2)),
            ("5 ^ 3", Value::Integer(6)),
            ("5 | 3", Value::Integer(7)),
            ("5 & 3", Value::Integer(1)),
            ("5 << 2", Value::Integer(20)),
            ("5 >> 2", Value::Integer(1)),
            ("-5 >> 2", Value::Integer(-2)),
        ];
        run_eval_tests(tests);
    }

    #[test]
    fn test_float_arithmetic() {
        let tests = vec![
            ("1.0", Value::Float(1.0)),
            ("0.2", Value::Float(0.2)),
            ("1.0 + 0.2", Value::Float(1.2)),
            ("1.2 - 1.0", Value::Float(0.19999999999999996)),
            ("0.1 * 0.2", Value::Float(0.020000000000000004)),
            ("4.0 / 2.0", Value::Float(2.0)),
            ("7.2 % 3.0", Value::Float(1.2000000000000002)),
            ("5.0 / 2.0 * 2.0 + 1.0 - 0.5", Value::Float(5.5)),
            ("5.0 * (0.2 + 1.0)", Value::Float(6.0)),
            ("0.5 + 0.5 + 0.5 + 0.5 - 1.0", Value::Float(1.0)),
            ("0.2 * 0.2 * 0.2 * 0.2 * 0.2", Value::Float(0.00032000000000000013)),
            ("0.5 * 2.2 + 1.1", Value::Float(2.2)),
            ("0.5 + 0.2 * 10.0", Value::Float(2.5)),
            ("0.5 * (2.0 + 10.0)", Value::Float(6.0)),
            ("-0.5", Value::Float(-0.5)),
            ("-1.0", Value::Float(-1.0)),
            ("-5.0 + 10.0 + -5.0", Value::Float(0.0)),
            ("(0.5 + 1.5 * 0.2 + 1.5 / 3.0) * 2.0 + -1.0", Value::Float(1.6)),
        ];
        run_eval_tests(tests);
    }

    #[test]
    fn test_boolean_arithmetic() {
        let tests = vec![
            ("true", Value::Boolean(true)),
            ("false", Value::Boolean(false)),
            ("1 < 2", Value::Boolean(true)),
            ("1 > 2", Value::Boolean(false)),
            ("1 < 1", Value::Boolean(false)),
            ("1 > 1", Value::Boolean(false)),
            ("1 <= 2", Value::Boolean(true)),
            ("1 >= 2", Value::Boolean(false)),
            ("1 <= 1", Value::Boolean(true)),
            ("1 >= 1", Value::Boolean(true)),
            ("1 == 1", Value::Boolean(true)),
            ("1 != 1", Value::Boolean(false)),
            ("1 == 2", Value::Boolean(false)),
            ("1 != 2", Value::Boolean(true)),
            ("true == true", Value::Boolean(true)),
            ("false == false", Value::Boolean(true)),
            ("true == false", Value::Boolean(false)),
            ("true != false", Value::Boolean(true)),
            ("false != true", Value::Boolean(true)),
            ("(1 < 2) == true", Value::Boolean(true)),
            ("(1 < 2) == false", Value::Boolean(false)),
            ("(1 > 2) == true", Value::Boolean(false)),
            ("(1 > 2) == false", Value::Boolean(true)),
            ("!true", Value::Boolean(false)),
            ("!false", Value::Boolean(true)),
            ("!!true", Value::Boolean(true)),
            ("!!false", Value::Boolean(false)),
            ("!(if (false) { 5; })", Value::Boolean(true)),
        ];
        run_eval_tests(tests);
    }

    #[test]
    fn test_string_literal() {
        let tests = vec![
            (r#""hello world""#, Value::String(String::from("hello world"))),
            (r#""hello" + " world""#, Value::String(String::from("hello world"))),
            (r#""hello"+" world"+"!""#, Value::String(String::from("hello world!"))),
        ];
        run_eval_tests(tests);
    }

    #[test]
    fn test_logical_expr() {
        let tests = vec![
            ("true || true", Value::Boolean(true)),
            ("true || false", Value::Boolean(true)),
            ("false || true", Value::Boolean(true)),
            ("false || false", Value::Boolean(false)),
            ("\"Cat\" || \"Dog\"", Value::String(String::from("Cat"))),
            ("false || \"Cat\"", Value::String(String::from("Cat"))),
            ("\"Cat\" || false", Value::String(String::from("Cat"))),
            ("\"\" || false", Value::String(String::from(""))),
            ("false || \"\"", Value::String(String::from(""))),
            ("true && true", Value::Boolean(true)),
            ("true &&  false", Value::Boolean(false)),
            ("false && true", Value::Boolean(false)),
            ("false && false", Value::Boolean(false)),
            ("\"Cat\" && \"Dog\"", Value::String(String::from("Dog"))),
            ("false && \"Cat\"", Value::Boolean(false)),
            ("\"Cat\" && false", Value::Boolean(false)),
            ("\"\" && false", Value::Boolean(false)),
            ("false && \"\"", Value::Boolean(false)),
            ("true || false && false", Value::Boolean(true)),
            ("(true || false) && false", Value::Boolean(false)),
            ("true && (false || false)", Value::Boolean(false)),
            ("2 == 3 || (4 < 0 && 1 == 1)", Value::Boolean(false)),
            ("true && false && 1 == 1", Value::Boolean(false)),
            ("let flag = true && false && 1 == 1;", Value::Boolean(false)),
        ];
        run_eval_tests(tests);
    }

    #[test]
    fn test_if_expr() {
        let tests = vec![
            ("if (true) { 10 }", Value::Integer(10)),
            ("if (true) { 10 } else { 20 }", Value::Integer(10)),
            ("if (false) { 10 } else { 20 } ", Value::Integer(20)),
            ("if (1) { 10 }", Value::Integer(10)),
            ("if (1 < 2) { 10 }", Value::Integer(10)),
            ("if (1 < 2) { 10 } else { 20 }", Value::Integer(10)),
            ("if (1 > 2) { 10 } else { 20 }", Value::Integer(20)),
            ("if (1 > 2) { 10 }", Value::None),
            ("if (false) { 10 }", Value::None),
            ("if ((if (false) { 10 })) { 10 } else { 20 }", Value::Integer(20)),
            ("if (true) {} else { 10 }", Value::None),
            ("if (true) { 1; 2 } else { 3 }", Value::Integer(2)),
        ];
        run_eval_tests(tests);
    }

    #[test]
    fn test_call_native() {
        let tests = vec![
            ("length(\"\")", Value::Integer(0)),
            ("length(\"two\")", Value::Integer(3)),
            ("length(\"hello world\")", Value::Integer(11)),
            (
                "length(1)",
                Value::Error(String::from("function length not supported type Integer(1)")),
            ),
            (
                "length(\"one\", \"two\")",
                Value::Error(String::from("wrong number of arguments. got=2, want=1")),
            ),
            ("length([])", Value::Integer(0)),
            ("length([1, 2, 3])", Value::Integer(3)),
        ];
        run_eval_tests(tests);
    }

    #[test]
    fn test_array_literal() {
        let tests = vec![
            ("[]", Value::Array(vec![])),
            (
                "[1, 2, 3]",
                Value::Array(vec![Value::Integer(1), Value::Integer(2), Value::Integer(3)]),
            ),
            (
                "[1 + 2, 3 - 4, 5 * 6]",
                Value::Array(vec![Value::Integer(3), Value::Integer(-1), Value::Integer(30)]),
            ),
        ];
        run_eval_tests(tests);
    }

    #[test]
    fn test_map_literal() {
        let tests = vec![
            ("{}", Value::Map(HashMap::new())),
            (
                "{1: 2, 2: 3}",
                Value::Map(HashMap::from_iter(vec![
                    (String::from("1"), Value::Integer(2)),
                    (String::from("2"), Value::Integer(3)),
                ])),
            ),
            (
                "{1 + 1: 2 * 2, 3 + 3: 4 * 4}",
                Value::Map(HashMap::from_iter(vec![
                    (String::from("2"), Value::Integer(4)),
                    (String::from("6"), Value::Integer(16)),
                ])),
            ),
        ];
        run_eval_tests(tests);
    }

    #[test]
    fn test_index_expr() {
        let tests = vec![
            ("[1, 2, 3][1]", Value::Integer(2)),
            ("[1, 2, 3][0 + 2]", Value::Integer(3)),
            ("[[1, 1, 1]][0][0]", Value::Integer(1)),
            ("[][0]", Value::None),
            ("[1, 2, 3][99]", Value::None),
            ("[1][-1]", Value::None),
            ("{1: 1, 2: 2}[1]", Value::Integer(1)),
            ("{1: 1, 2: 2}[2]", Value::Integer(2)),
            ("{1: 1}[0]", Value::None),
            ("{}[0]", Value::None),
        ];
        run_eval_tests(tests);
    }

    #[test]
    fn test_field_expr() {
        let tests = vec![("{\"a\": 2}.a", Value::Integer(2))];
        run_eval_tests(tests);
    }
}
