use assert_fs::prelude::*;
use axum::routing::get;
use axum::Router;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

#[tokio::test]
async fn test_command_repl() -> Result<(), Box<dyn std::error::Error>> {
    let mut child = Command::new("basjoofan")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn child process");
    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all("let x = 1 + 1; println(\"{x}\", x);\n".as_bytes()).await?;
        stdin.write_all("exit".as_bytes()).await?;
    }
    let output = child.wait_with_output().await?;
    println!("output:{}", String::from_utf8_lossy(&output.stdout).trim());
    assert!(String::from_utf8_lossy(&output.stdout).trim() == "2\nnull");
    Ok(())
}

#[tokio::test]
async fn test_command_eval() -> Result<(), Box<dyn std::error::Error>> {
    let mut cmd = Command::new("basjoofan");
    let output = cmd.arg("eval").arg(r#"print("{integer}", 1 + 1 )"#).output().await?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?, "2null\n");
    let mut cmd = Command::new("basjoofan");
    let output = cmd.arg("eval").arg(r#"let x = 1 + 1; print("{integer}", x);"#).output().await?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?, "2null\n");
    let mut cmd = Command::new("basjoofan");
    let output = cmd.arg("eval").arg(r#"println("{string}", "Hello Basjoofan!")"#).output().await?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?, "Hello Basjoofan!\nnull\n");
    Ok(())
}

#[tokio::test]
async fn test_command_test() -> Result<(), Box<dyn std::error::Error>> {
    let router = Router::new().route("/hello", get(|| async { "Hello, World!" }));
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 8888)).await.unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let temp = assert_fs::TempDir::new().unwrap();
    let file = temp.child("request.fan");
    let text = r#"
    let host = "localhost:8888";
    request hello`
        GET http://{host}/hello
        Host: {host}
    `[status == 200];

    test call {
        let response = hello();
        response.status
    }
    "#;
    file.write_str(text)?;
    // command test
    let mut command = Command::new("basjoofan");
    command.current_dir(&temp);
    let output = command.arg("test").output().await?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    println!("stdout:{}", stdout);
    println!("stderr:{}", stderr);
    assert!(stdout.contains("--- PASS  hello ("));
    // command test call
    let mut command = Command::new("basjoofan");
    command.current_dir(&temp);
    let output = command.arg("test").arg("call").output().await?;
    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout)?.contains("--- PASS  hello ("));
    // command test blank
    let mut command = Command::new("basjoofan");
    command.current_dir(&temp);
    let output = command.arg("test").arg("blank").output().await?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?, "Test not found: blank\n");
    Ok(())
}
