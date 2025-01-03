use crate::http;
use crate::native;
use crate::Assert;
use crate::Expr;
use crate::Kind;
use crate::Record;
use crate::Records;
use crate::Token;
use crate::Value;
use std::collections::HashMap;

#[derive(Default, Clone)]
pub struct Context {
    inner: HashMap<String, Value>,
    requests: HashMap<String, (String, Vec<Expr>)>,
}

impl Context {
    pub fn new() -> Self {
        Self {
            inner: HashMap::new(),
            requests: HashMap::new(),
        }
    }

    pub fn from(inner: HashMap<String, Value>) -> Self {
        Self {
            inner,
            requests: HashMap::new(),
        }
    }

    pub fn extend(&mut self, requests: HashMap<String, (String, Vec<Expr>)>) {
        self.requests.extend(requests);
    }

    pub fn eval(&mut self, exprs: &[Expr], records: &mut Records) -> Result<Value, String> {
        eval_block(exprs, self, records)
    }
}

fn eval_expr(expr: &Expr, context: &mut Context, records: &mut Records) -> Result<Value, String> {
    match expr {
        Expr::Integer(integer) => eval_integer_literal(integer),
        Expr::Float(float) => eval_float_literal(float),
        Expr::Boolean(boolean) => eval_boolean_literal(boolean),
        Expr::String(string) => eval_string_literal(string),
        Expr::Array(items) => eval_array_literal(items, context, records),
        Expr::Map(pairs) => eval_map_literal(pairs, context, records),
        Expr::Index(value, index) => eval_index_expr(value, index, context, records),
        Expr::Field(map, field) => eval_field_expr(map, field, context, records),
        Expr::Ident(ident) => eval_ident_expr(ident, context),
        Expr::Let(name, expr) => eval_let_expr(name, expr, context, records),
        Expr::Unary(token, right) => eval_unary_expr(token, right, context, records),
        Expr::Binary(token, left, right) => eval_binary_expr(token, left, right, context, records),
        Expr::Paren(expr) => eval_expr(expr, context, records),
        Expr::If(condition, consequence, alternative) => eval_if_expr(condition, consequence, alternative, context, records),
        Expr::Call(name, arguments) => eval_call_expr(name, arguments, context, records),
    }
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

fn eval_array_literal(items: &[Expr], context: &mut Context, records: &mut Records) -> Result<Value, String> {
    Ok(Value::Array(eval_list(items, context, records)?))
}

fn eval_map_literal(pairs: &Vec<(Expr, Expr)>, context: &mut Context, records: &mut Records) -> Result<Value, String> {
    let mut map = HashMap::new();
    for (key, value) in pairs {
        let key = eval_expr(key, context, records)?;
        let value = eval_expr(value, context, records)?;
        map.insert(key.to_string(), value);
    }
    Ok(Value::Map(map))
}

fn eval_index_expr(value: &Expr, index: &Expr, context: &mut Context, records: &mut Records) -> Result<Value, String> {
    // TODO enhance indent expr get variable use reference
    let value = eval_expr(value, context, records)?;
    let index = eval_expr(index, context, records)?;
    match (value, index) {
        (Value::Array(mut items), Value::Integer(index)) => {
            let index = index as usize;
            if index < items.len() {
                let item = items.remove(index);
                Ok(item)
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

fn eval_field_expr(map: &Expr, field: &String, context: &mut Context, records: &mut Records) -> Result<Value, String> {
    // TODO enhance indent expr get variable use reference
    match eval_expr(map, context, records)? {
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

fn eval_ident_expr(ident: &String, context: &mut Context) -> Result<Value, String> {
    match context.inner.get(ident) {
        Some(value) => Ok(value.to_owned()),
        None => Err(format!("ident:{} not found", ident)),
    }
}

fn eval_let_expr(name: &String, expr: &Expr, context: &mut Context, records: &mut Records) -> Result<Value, String> {
    let value = eval_expr(expr, context, records)?;
    context.inner.insert(name.to_owned(), value.to_owned());
    Ok(value)
}

fn eval_unary_expr(token: &Token, right: &Expr, context: &mut Context, records: &mut Records) -> Result<Value, String> {
    let right = eval_expr(right, context, records)?;
    match (token.kind, right) {
        (Kind::Not, Value::Boolean(false)) | (Kind::Not, Value::None) => Ok(Value::Boolean(true)),
        (Kind::Not, Value::Integer(integer)) => Ok(Value::Integer(!integer)),
        (Kind::Not, _) => Ok(Value::Boolean(false)),
        (Kind::Sub, Value::Integer(integer)) => Ok(Value::Integer(-integer)),
        (Kind::Sub, Value::Float(float)) => Ok(Value::Float(-float)),
        (_, right) => Err(format!("unknown operator: {}{:?}", token, right)),
    }
}

fn eval_binary_expr(token: &Token, left: &Expr, right: &Expr, context: &mut Context, records: &mut Records) -> Result<Value, String> {
    match token.kind {
        Kind::Add => Ok(eval_expr(left, context, records)? + eval_expr(right, context, records)?),
        Kind::Sub => Ok(eval_expr(left, context, records)? - eval_expr(right, context, records)?),
        Kind::Mul => Ok(eval_expr(left, context, records)? * eval_expr(right, context, records)?),
        Kind::Div => Ok(eval_expr(left, context, records)? / eval_expr(right, context, records)?),
        Kind::Rem => Ok(eval_expr(left, context, records)? % eval_expr(right, context, records)?),
        Kind::Bx => Ok(eval_expr(left, context, records)? ^ eval_expr(right, context, records)?),
        Kind::Bo => Ok(eval_expr(left, context, records)? | eval_expr(right, context, records)?),
        Kind::Ba => Ok(eval_expr(left, context, records)? & eval_expr(right, context, records)?),
        Kind::Sl => Ok(eval_expr(left, context, records)? << eval_expr(right, context, records)?),
        Kind::Sr => Ok(eval_expr(left, context, records)? >> eval_expr(right, context, records)?),
        Kind::Lo => match eval_expr(left, context, records)? {
            Value::Boolean(false) | Value::None => eval_expr(right, context, records),
            left => Ok(left),
        },
        Kind::La => match eval_expr(left, context, records)? {
            left @ (Value::Boolean(false) | Value::None) => Ok(left),
            _ => eval_expr(right, context, records),
        },
        Kind::Lt => Ok(Value::Boolean(
            eval_expr(left, context, records)? < eval_expr(right, context, records)?,
        )),
        Kind::Gt => Ok(Value::Boolean(
            eval_expr(left, context, records)? > eval_expr(right, context, records)?,
        )),
        Kind::Le => Ok(Value::Boolean(
            eval_expr(left, context, records)? <= eval_expr(right, context, records)?,
        )),
        Kind::Ge => Ok(Value::Boolean(
            eval_expr(left, context, records)? >= eval_expr(right, context, records)?,
        )),
        Kind::Eq => Ok(Value::Boolean(
            eval_expr(left, context, records)? == eval_expr(right, context, records)?,
        )),
        Kind::Ne => Ok(Value::Boolean(
            eval_expr(left, context, records)? != eval_expr(right, context, records)?,
        )),
        _ => Err(format!("not support operator: {} {} {}", left, token, right)),
    }
}

fn eval_if_expr(
    condition: &Expr,
    consequence: &[Expr],
    alternative: &[Expr],
    context: &mut Context,
    records: &mut Records,
) -> Result<Value, String> {
    let condition = eval_expr(condition, context, records)?;
    match condition {
        Value::Boolean(false) | Value::None => eval_block(alternative, context, records),
        _ => eval_block(consequence, context, records),
    }
}

fn eval_call_expr(name: &str, arguments: &[Expr], context: &mut Context, records: &mut Records) -> Result<Value, String> {
    let arguments = eval_list(arguments, context, records)?;
    match context.requests.get(name) {
        Some((message, asserts)) => {
            let name = name.to_string();
            let regex = regex::Regex::new(r"\{\s*([a-zA-Z_][a-zA-Z0-9_]*)\s*\}").unwrap();
            let matches = regex.find_iter(message);
            let mut ranges = Vec::new();
            matches.for_each(|m| ranges.push((m.as_str()[1..m.as_str().len() - 1].trim(), m.range())));
            ranges.reverse();
            let mut message = message.to_string();
            for (variable, range) in ranges.into_iter() {
                let variable = match context.inner.get(variable) {
                    Some(variable) => variable.to_string(),
                    None => Value::None.to_string(),
                };
                message.replace_range(range, variable.as_str());
            }
            let client = http::Client::default();
            let (request, response, time, error) = client.send(message.as_str());
            let map = response.to_map();
            let mut context = Context::from(map);
            let asserts = asserts
                .iter()
                .filter_map(|assert| match assert {
                    Expr::Binary(token, left, right) => {
                        let left = eval_expr(left, &mut context, records).unwrap_or(Value::None);
                        let right = eval_expr(right, &mut context, records).unwrap_or(Value::None);
                        match token.kind {
                            Kind::Lt => Some(left < right),
                            Kind::Gt => Some(left > right),
                            Kind::Le => Some(left <= right),
                            Kind::Ge => Some(left >= right),
                            Kind::Eq => Some(left == right),
                            Kind::Ne => Some(left != right),
                            _ => None,
                        }
                        .map(|result| Assert {
                            expr: format!("{} {} {}", left, token, right),
                            left: left.to_string(),
                            compare: token.to_string(),
                            right: right.to_string(),
                            result,
                        })
                    }
                    _ => None,
                })
                .collect::<Vec<Assert>>();
            records.push(Record {
                name,
                request,
                response,
                time,
                error,
                asserts,
            });
            Ok(Value::Map(context.inner))
        }
        None => match name {
            "println" => Ok(native::println(arguments)),
            "print" => Ok(native::print(arguments)),
            "format" => Ok(native::format(arguments)),
            "length" => Ok(native::length(arguments)),
            "append" => Ok(native::append(arguments)),
            _ => Err(format!("function {} not found", name)),
        },
    }
}

fn eval_block(exprs: &[Expr], context: &mut Context, records: &mut Records) -> Result<Value, String> {
    let mut result = Value::None;
    for expr in exprs {
        result = eval_expr(expr, context, records)?;
    }
    Ok(result)
}

fn eval_list(items: &[Expr], context: &mut Context, records: &mut Records) -> Result<Vec<Value>, String> {
    let mut values = Vec::with_capacity(items.len());
    for item in items {
        values.push(eval_expr(item, context, records)?);
    }
    Ok(values)
}

#[cfg(test)]
mod tests {
    use super::eval_block;
    use crate::parser::Parser;
    use crate::parser::Source;
    use crate::Context;
    use crate::Records;
    use crate::Value;
    use std::collections::HashMap;

    fn run_eval_tests(tests: Vec<(&str, Value)>) {
        for (text, expect) in tests {
            let Source { exprs, requests, .. } = Parser::new(text).parse().unwrap();
            let mut context = Context::new();
            let mut records = Records::new();
            context.extend(requests);
            match eval_block(&exprs, &mut context, &mut records) {
                Ok(value) => {
                    println!("{:?} => {} = {}", exprs, value, expect);
                    assert_eq!(value, expect);
                }
                Err(message) => panic!("machine error: {}", message),
            }
        }
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
    fn test_request_literal() {
        let tests = vec![(
            r#"
            request get`
                GET http://{host}/get
                Host: {host}
                Connection: close
            `[];
            let host = "httpbin.org";
            let response = get();
            response.status
            "#,
            Value::Integer(200),
        )];
        run_eval_tests(tests);
    }

    #[test]
    fn test_request_asserts() {
        let tests = vec![(
            r#"
            request get`
                GET http://{host}/get
                Host: {host}
                Connection: close
            `[status == 200];
            let host = "httpbin.org";
            let response = get();
            response.status
            "#,
            Value::Integer(200),
        )];
        run_eval_tests(tests);
    }
}
