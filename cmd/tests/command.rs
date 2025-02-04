use assert_cmd::prelude::*;
use assert_fs::prelude::*;
use predicates::prelude::*;
use std::io::Write;
use std::process::{Command, Stdio};

#[test]
#[allow(clippy::zombie_processes)]
fn test_command_repl() -> Result<(), Box<dyn std::error::Error>> {
    let mut child = Command::cargo_bin("fan")?
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all("let x = 1 + 1; println(\"{x}\", x);\n".as_bytes())
            .expect("Failed to write to stdin");
        stdin.write_all("exit".as_bytes()).expect("Failed to write to stdin");
    }
    let output = child.wait_with_output().expect("Failed to read stdout");
    println!("output:{}", String::from_utf8_lossy(&output.stdout).trim());
    assert!(String::from_utf8_lossy(&output.stdout).trim() == "2\nnull");
    Ok(())
}

#[test]
fn test_command_eval() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::cargo_bin("fan")?;
    cmd.arg("eval").arg(r#"print("{integer}", 1 + 1 )"#);
    cmd.assert().success().stdout(predicate::str::diff("2null\n"));
    let mut cmd = Command::cargo_bin("fan")?;
    cmd.arg("eval").arg(r#"let x = 1 + 1; print("{integer}", x);"#);
    cmd.assert().success().stdout(predicate::str::diff("2null\n"));
    let mut cmd = Command::cargo_bin("fan")?;
    cmd.arg("eval").arg(r#"println("{string}", "Hello Basjoofan!")"#);
    cmd.assert().success().stdout(predicate::str::diff("Hello Basjoofan!\nnull\n"));
    Ok(())
}

#[test]
fn test_command_test() -> Result<(), Box<dyn std::error::Error>> {
    let temp = assert_fs::TempDir::new().unwrap();
    let file = temp.child(format!("request.{}", "fan"));
    let text = r#"
    let host = "httpbin.org";
    request get`
        GET http://{host}/get
        Host: {host}
        Connection: close
    `[status == 200];

    test call {
        let response = get();
        response.status
    }
    "#;
    file.write_str(text)?;
    // command test
    let mut command = Command::cargo_bin("fan")?;
    command.current_dir(&temp);
    command.arg("test");
    command.assert().success().stdout(predicate::str::contains("--- PASS  get ("));
    // command test call
    let mut command = Command::cargo_bin("fan")?;
    command.current_dir(&temp);
    command.arg("test").arg("call");
    command.assert().success().stdout(predicate::str::contains("--- PASS  get ("));
    // command test blank
    let mut command = Command::cargo_bin("fan")?;
    command.current_dir(&temp);
    command.arg("test").arg("blank");
    command.assert().success().stdout(predicate::str::diff("Test not found: blank\n"));

    Ok(())
}
