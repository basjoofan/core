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
            .write_all("let add = fn(x, y) { x + y; }; println(\"{integer}\", add(5, 5));\n".as_bytes())
            .expect("Failed to write to stdin");
        stdin.write_all("exit".as_bytes()).expect("Failed to write to stdin");
    }
    let output = child.wait_with_output().expect("Failed to read stdout");
    println!("output:{}", String::from_utf8_lossy(&output.stdout).trim());
    assert!(String::from_utf8_lossy(&output.stdout).trim() == "10\nnone");
    Ok(())
}

#[test]
fn test_command_eval() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin(NAME)?;
    cmd.arg("eval").arg(r#"print("{integer}", 1 + 1 )"#);
    cmd.assert().success().stdout(predicate::str::diff("2none\n"));
    let mut cmd = Command::cargo_bin(NAME)?;
    cmd.arg("eval")
        .arg(r#"let add = fn(x, y) { x + y; }; print("{integer}", add(1, 1));"#);
    cmd.assert().success().stdout(predicate::str::diff("2none\n"));
    let mut cmd = Command::cargo_bin(NAME)?;
    cmd.arg("eval").arg(r#"println("{string}", "Hello Am!")"#);
    cmd.assert().success().stdout(predicate::str::diff("Hello Am!\nnone\n"));
    Ok(())
}

#[test]
fn test_command_run_closure() -> Result<(), Box<dyn std::error::Error>> {
    let file = assert_fs::NamedTempFile::new("closure.am")?;
    let text = r#"
    let first = 10;
    let second = 10;
    let third = 10;
    
    let ourFunction = fn(first) {
    let second = 20;
    
    first + second + third;
    };
    
    println("{integer}", ourFunction(20) + first + second);
    "#;
    file.write_str(text)?;

    let mut cmd = Command::cargo_bin(NAME)?;
    cmd.arg("run").arg(file.path());
    cmd.assert().success().stdout(predicate::str::contains("70"));
    Ok(())
}

#[test]
fn test_command_run_fibonacci() -> Result<(), Box<dyn std::error::Error>> {
    let file = assert_fs::NamedTempFile::new("fibonacci.am")?;
    let text = r#"
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
    println("{integer}", fibonacci(10));
    "#;
    file.write_str(text)?;

    let mut cmd = Command::cargo_bin(NAME)?;
    cmd.arg("run").arg(file.path());
    cmd.assert().success().stdout(predicate::str::contains("55"));
    Ok(())
}

#[test]
fn test_command_run_dir() -> Result<(), Box<dyn std::error::Error>> {
    let temp = assert_fs::TempDir::new().unwrap();
    let function = temp.child("function.am");
    function.write_str("let add = fn(a, b){ a + b }")?;
    let call = temp.child("call.am");
    call.write_str(r#"println("{integer}", add(1, 1));"#)?;
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
    let text = r#"
    let host = "httpbin.org";
    rq request()`
        GET http://{host}/get
        Host: {host}
        Connection: close
    `[status == 200];

    test call {
        let response = request();
        response.status
    }
    "#;
    file.write_str(text)?;
    let mut cmd = Command::cargo_bin(NAME)?;
    cmd.current_dir(&temp);
    cmd.arg("test");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--- PASS  call/request ("));
    Ok(())
}

#[test]
fn test_command_call() -> Result<(), Box<dyn std::error::Error>> {
    let temp = assert_fs::TempDir::new().unwrap();
    let file = temp.child("request.am");
    let text = r#"
    let host = "httpbin.org";
    rq request()`
        GET http://{host}/get
        Host: {host}
        Connection: close
    `[status == 200];

    test call {
        let response = request();
        response.status
    }
    "#;
    file.write_str(text)?;
    let mut cmd = Command::cargo_bin(NAME)?;
    cmd.current_dir(&temp);
    cmd.arg("test").arg("call");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("--- PASS  call/request ("));

    let mut cmd = Command::cargo_bin(NAME)?;
    cmd.current_dir(&temp);
    cmd.arg("test").arg("blank");
    cmd.assert().success().stdout(predicate::str::contains("not found"));
    Ok(())
}
