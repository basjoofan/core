use assert_fs::prelude::*;
use axum::Router;
use axum::routing::get;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

#[tokio::test]
async fn test_command_repl() -> Result<(), Box<dyn std::error::Error>> {
    let mut command = new_command();
    command.stdout(Stdio::piped());
    command.stdin(Stdio::piped());
    let mut child = command.spawn().expect("Failed to spawn child process");
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all("let x = 1 + 1; println(\"{x}\");\n".as_bytes())
            .await?;
        stdin.write_all("exit".as_bytes()).await?;
    }
    let output = child.wait_with_output().await?;
    println!("output:{}", String::from_utf8_lossy(&output.stdout).trim());
    assert!(String::from_utf8_lossy(&output.stdout).trim() == "2\nnull");
    Ok(())
}

#[tokio::test]
async fn test_command_repl_loads_yaml_then_evaluates_fan() -> Result<(), Box<dyn std::error::Error>>
{
    let router = Router::new().route("/hello", get(|| async { "Hello, World!" }));
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 8889))
        .await
        .unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    let mut command = new_command();
    command.stdout(Stdio::piped());
    command.stdin(Stdio::piped());
    let mut child = command.spawn().expect("Failed to spawn child process");
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(
                r#"name: user
scheme: http
host: 127.0.0.1:8889
requests:
  - hello:
      path: /hello
      method: GET
      headers:
        - Connection: close
      asserts:
        - status == 200

let response = user.hello(); response.status
exit"#
                    .as_bytes(),
            )
            .await?;
    }
    let output = child.wait_with_output().await?;
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("loaded client: user"));
    assert!(stdout.contains("200"));
    Ok(())
}

#[tokio::test]
async fn test_command_repl_flushes_yaml_on_exit() -> Result<(), Box<dyn std::error::Error>> {
    let mut command = new_command();
    command.stdout(Stdio::piped());
    command.stdin(Stdio::piped());
    let mut child = command.spawn().expect("Failed to spawn child process");
    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(
                r#"name: user
scheme: http
host: example.com
requests:
  - get:
      path: /get
      method: GET
exit"#
                    .as_bytes(),
            )
            .await?;
    }
    let output = child.wait_with_output().await?;
    assert_eq!(String::from_utf8(output.stdout)?, "loaded client: user\n");
    Ok(())
}

#[tokio::test]
async fn test_command_eval() -> Result<(), Box<dyn std::error::Error>> {
    let mut command = new_command();
    let output = command
        .arg("eval")
        .arg(r#"let integer = 1 + 1; print("{integer}")"#)
        .output()
        .await?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?, "2null\n");
    let mut command = new_command();
    let output = command
        .arg("eval")
        .arg(r#"fn add(x, y) { x + y; }; let integer = add(1, 1); print("{integer}");"#)
        .output()
        .await?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?, "2null\n");
    let mut command = new_command();
    let output = command
        .arg("eval")
        .arg(r#""🍀 Hello Basjoofan!""#)
        .output()
        .await?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?, "🍀 Hello Basjoofan!\n");
    Ok(())
}

#[tokio::test]
async fn test_command_eval_loads_yaml_client_definition() -> Result<(), Box<dyn std::error::Error>>
{
    let mut command = new_command();
    let output = command
        .arg("eval")
        .arg(
            r#"name: user
scheme: http
host: example.com
requests:
  - get:
      path: /get
      method: GET"#,
        )
        .output()
        .await?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?, "loaded client: user\n");
    Ok(())
}

#[tokio::test]
async fn test_command_eval_reports_invalid_yaml_shape() -> Result<(), Box<dyn std::error::Error>> {
    let mut command = new_command();
    let output = command
        .arg("eval")
        .arg(
            r#"name: user
scheme: http
host: example.com
requests:"#,
        )
        .output()
        .await?;
    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout)?.contains("request list"));
    Ok(())
}

#[tokio::test]
async fn test_command_eval_rejects_mixed_yaml_and_fan() -> Result<(), Box<dyn std::error::Error>> {
    let mut command = new_command();
    let output = command
        .arg("eval")
        .arg(
            r#"name: user
scheme: http
host: example.com
requests:
  - get:
      path: /get
      method: GET
let response = user.get();"#,
        )
        .output()
        .await?;
    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout)?.contains("must be submitted separately"));
    Ok(())
}

#[tokio::test]
async fn test_command_test() -> Result<(), Box<dyn std::error::Error>> {
    let router = Router::new().route("/hello", get(|| async { "Hello, World!" }));
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 8888))
        .await
        .unwrap();
    tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    let temp = assert_fs::TempDir::new().unwrap();
    let file = temp.child("request.fan");
    let text = r#"
    test call {
        let host = "localhost:8888";
        let response = user.hello();
        response.status
    }
    "#;
    file.write_str(text)?;
    let file = temp.child("client.yaml");
    let text = r#"
name: user
scheme: http
host: {host}
requests:
  - hello:
      path: /hello
      method: GET
      headers:
        - Connection: close
      asserts:
        - status == 200
    "#;
    file.write_str(text)?;
    // command test
    let mut command = new_command();
    command.current_dir(&temp);
    let output = command.arg("test").output().await?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    let stderr = String::from_utf8(output.stderr)?;
    println!("stdout:{stdout}");
    println!("stderr:{stderr}");
    assert!(stdout.contains("--- PASS  user.hello ("));
    // command test call
    let mut command = new_command();
    command.current_dir(&temp);
    let output = command.arg("test").arg("call").output().await?;
    assert!(output.status.success());
    assert!(String::from_utf8(output.stdout)?.contains("--- PASS  user.hello ("));
    // command test blank
    let mut command = new_command();
    command.current_dir(&temp);
    let output = command.arg("test").arg("blank").output().await?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?, "Test not found: blank\n");
    Ok(())
}

fn new_command() -> Command {
    Command::new(cargo_bin("basjoofan"))
}

/// Look up the path to a cargo-built binary within an integration test.
fn cargo_bin<S: AsRef<str>>(name: S) -> PathBuf {
    cargo_bin_str(name.as_ref())
}

fn cargo_bin_str(name: &str) -> PathBuf {
    let env_var = format!("CARGO_BIN_EXE_{name}");
    std::env::var_os(env_var)
        .map(|p| p.into())
        .unwrap_or_else(|| target_dir().join(format!("{}{}", name, std::env::consts::EXE_SUFFIX)))
}

// Adapted from
// https://github.com/rust-lang/cargo/blob/485670b3983b52289a2f353d589c57fae2f60f82/tests/testsuite/support/mod.rs#L507
fn target_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .map(|mut path| {
            path.pop();
            if path.ends_with("deps") {
                path.pop();
            }
            path
        })
        .expect("this should only be used where a `current_exe` can be set")
}
