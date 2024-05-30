use am::command::NAME;
use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn test_command_repl() -> Result<(), Box<dyn std::error::Error>> {
    let mut child = Command::cargo_bin(NAME)?
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all("let add = fn(x, y) { x + y; }; println(add(5, 5));\n".as_bytes())
            .expect("Failed to write to stdin");
        stdin.write_all("exit".as_bytes()).expect("Failed to write to stdin");
    }
    let output = child.wait_with_output().expect("Failed to read stdout");
    assert!(String::from_utf8_lossy(&output.stdout).trim() == "10");
    println!("output:{}", String::from_utf8_lossy(&output.stdout).trim());
    Ok(())
}

#[test]
fn test_command_run_closure() -> Result<(), Box<dyn std::error::Error>> {
    let file = assert_fs::NamedTempFile::new("closure.am")?;
    let input = r#"
    let first = 10;
    let second = 10;
    let third = 10;
    
    let ourFunction = fn(first) {
    let second = 20;
    
    first + second + third;
    };
    
    println(ourFunction(20) + first + second);
    "#;
    file.write_str(input)?;

    let mut cmd = Command::cargo_bin(NAME)?;
    cmd.arg("run").arg(file.path());
    cmd.assert().success().stdout(predicate::str::contains("70"));
    Ok(())
}

#[test]
fn test_command_run_fibonacci() -> Result<(), Box<dyn std::error::Error>> {
    let file = assert_fs::NamedTempFile::new("fibonacci.am")?;
    let input = r#"
    let fibonacci = fn (x) {
        if (x == 0) {
          0
        } else {
          if (x == 1) {
            1
          } else {
            fibonacci(x - 1) + fibonacci(x -2)
          }
        }
      };  
    println(fibonacci(10));
    "#;
    file.write_str(input)?;

    let mut cmd = Command::cargo_bin(NAME)?;
    cmd.arg("run").arg(file.path());
    cmd.assert().success().stdout(predicate::str::contains("55"));
    Ok(())
}

#[test]
fn test_command_run_dir() -> Result<(), Box<dyn std::error::Error>> {
    let temp = assert_fs::TempDir::new().unwrap();
    let function = temp.child("function.am");
    function.write_str("fn add(a, b){ a + b }")?;
    let call = temp.child("call.am");
    call.write_str("println(add(1, 1));")?;
    let mut cmd = Command::cargo_bin(NAME)?;
    assert!(function.path().parent() == Some(temp.path()));
    cmd.arg("run").arg(function.path().parent().unwrap());
    cmd.assert().success().stdout(predicate::str::contains("2"));
    Ok(())
}

#[test]
fn test_command_test() -> Result<(), Box<dyn std::error::Error>> {
    let temp = assert_fs::TempDir::new().unwrap();
    let file = temp.child("request.am");
    let input = r#"
    #[test, tag]
    rq request`
      GET http://${host}/get
      Host: ${host}
    `[status == 200];
    let host = "httpbin.org";
    #[test, function]
    fn call() {
      let response = request().response;
      response.status
    }
    "#;
    file.write_str(input)?;
    let mut cmd = Command::cargo_bin(NAME)?;
    cmd.current_dir(temp.to_path_buf());
    cmd.arg("test");
    cmd.assert().success().stdout(predicate::str::contains("--- PASS  request ("));
    Ok(())
}

#[test]
fn test_command_call() -> Result<(), Box<dyn std::error::Error>> {
    let temp = assert_fs::TempDir::new().unwrap();
    let file = temp.child("request.am");
    let input = r#"
    #[test, tag]
    rq request`
      GET http://${host}/get
      Host: ${host}
    `[status == 200];
    let host = "httpbin.org";
    #[test, function]
    fn call() {
      let response = request().response;
      response.status
    }
    "#;
    file.write_str(input)?;
    let mut cmd = Command::cargo_bin(NAME)?;
    cmd.current_dir(temp.to_path_buf());
    cmd.arg("call").arg("call");
    cmd.assert().success().stdout(predicate::str::contains("--- PASS  request ("));

    let mut cmd = Command::cargo_bin(NAME)?;
    cmd.current_dir(temp.to_path_buf());
    cmd.arg("call").arg("blank");
    cmd.assert().success().stdout(predicate::str::contains("not found"));
    Ok(())
}
