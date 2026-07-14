use assert_fs::prelude::*;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

#[test]
fn dsl_fixture_parses_v1_declarations() {
    let input =
        std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/dsl.fan"))
            .unwrap();
    let source = lib::Parser::new(&input).parse().unwrap();
    assert_eq!(source.environments.len(), 2);
    assert_eq!(source.apis.get("user").unwrap().requests.len(), 9);
    assert_eq!(source.tests.len(), 1);
    let api = source.apis.get("user").unwrap();
    assert_eq!(api.requests["get"].params_def[0].kind, "int");
    assert!(matches!(
        api.requests["query"].body,
        lib::api::Body::Json(_)
    ));
    assert!(matches!(
        api.requests["submitForm"].body,
        lib::api::Body::Form(_)
    ));
    assert!(matches!(
        api.requests["updateAvatar"].body,
        lib::api::Body::Part(_)
    ));
    assert!(matches!(
        api.requests["webhook"].body,
        lib::api::Body::Text(_)
    ));
    assert!(matches!(
        api.requests["sendFile"].body,
        lib::api::Body::File(_)
    ));
    let test = source.test("createUser").unwrap();
    assert_eq!(test.tags, ["smoke", "users"]);
    assert_eq!(test.body.len(), 8);
}

#[tokio::test]
async fn cli_requires_environment() -> Result<(), Box<dyn std::error::Error>> {
    let temp = assert_fs::TempDir::new()?;
    temp.child("api.fan").write_str(
        "env local { scheme: http, host: \"example.test\" }\n@integration test flow { expect true; }",
    )?;
    let output = command()
        .arg("test")
        .arg("--path")
        .arg(temp.path())
        .output()
        .await?;
    assert!(!output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("--env is required"));
    Ok(())
}

#[tokio::test]
async fn cli_filters_by_tag() -> Result<(), Box<dyn std::error::Error>> {
    let temp = assert_fs::TempDir::new()?;
    temp.child("tests.fan")
        .write_str("@fast test selected { expect true; }\n@slow test skipped { expect true; }")?;
    let output = command()
        .arg("test")
        .arg("@fast")
        .arg("--path")
        .arg(temp.path())
        .output()
        .await?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("selected"));
    assert!(!stdout.contains("skipped"));
    Ok(())
}

#[tokio::test]
async fn eval_executes_expressions() -> Result<(), Box<dyn std::error::Error>> {
    let output = command()
        .arg("eval")
        .arg("let value = 1 + 1; value")
        .output()
        .await?;
    assert!(output.status.success());
    assert_eq!(String::from_utf8(output.stdout)?, "2\n");
    Ok(())
}

#[tokio::test]
async fn repl_recovers_after_invalid_delimiter() -> Result<(), Box<dyn std::error::Error>> {
    let mut child = command()
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()?;
    child
        .stdin
        .take()
        .unwrap()
        .write_all(b"let bad = 1; }\n1 + 1\nexit\n")
        .await?;
    let output = child.wait_with_output().await?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("unexpected token }"), "{stdout}");
    assert!(stdout.ends_with("2\n"), "{stdout}");
    Ok(())
}

#[tokio::test]
async fn cli_fails_for_false_expectation_and_unknown_test() -> Result<(), Box<dyn std::error::Error>>
{
    let temp = assert_fs::TempDir::new()?;
    temp.child("tests.fan")
        .write_str("test failing { expect false; }")?;
    let failed = command()
        .arg("test")
        .arg("--path")
        .arg(temp.path())
        .output()
        .await?;
    assert!(!failed.status.success());
    assert!(String::from_utf8_lossy(&failed.stdout).contains("expectation failed"));

    let missing = command()
        .arg("test")
        .arg("missing")
        .arg("--path")
        .arg(temp.path())
        .output()
        .await?;
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stdout).contains("Test not found"));
    Ok(())
}

#[tokio::test]
async fn cli_loads_fan_files_recursively_and_ignores_other_files()
-> Result<(), Box<dyn std::error::Error>> {
    let temp = assert_fs::TempDir::new()?;
    temp.child("a.fan")
        .write_str("@fast test first { expect true; }")?;
    temp.child("nested/b.fan")
        .write_str("@fast test second { expect true; }")?;
    temp.child("ignored.txt").write_str("test broken {")?;
    let output = command()
        .arg("test")
        .arg("@fast")
        .arg("--path")
        .arg(temp.path())
        .output()
        .await?;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout)?;
    assert!(stdout.contains("first"));
    assert!(stdout.contains("second"));
    Ok(())
}

#[tokio::test]
async fn cli_runs_dsl_against_a_real_http_server() -> Result<(), Box<dyn std::error::Error>> {
    use axum::Json;
    use axum::Router;
    use axum::http::StatusCode;
    use axum::routing::post;

    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0)).await?;
    let port = listener.local_addr()?.port();
    let app =
        Router::new().route(
            "/users",
            post(|Json(value): Json<serde_json::Value>| async move {
                (StatusCode::CREATED, Json(value))
            }),
        );
    let server = tokio::spawn(async move { axum::serve(listener, app).await });

    let temp = assert_fs::TempDir::new()?;
    temp.child("scenario.fan").write_str(&format!(
        r#"env local {{ scheme: http, host: "127.0.0.1", port: {port} }}
        api user {{
            scheme: env.scheme,
            host: env.host,
            port: env.port,
            create(name: string) {{ method: POST, path: "/users", json: {{ name: name }} }}
        }}
        @e2e test createUser {{
            let response = user.create("Gauss");
            expect response.status == 201;
            expect response.json.name == "Gauss";
            expect response.request.method == "POST";
        }}"#,
    ))?;
    let output = command()
        .arg("test")
        .arg("createUser")
        .arg("--env")
        .arg("local")
        .arg("--path")
        .arg(temp.path())
        .output()
        .await?;
    server.abort();
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("PASS  createUser (3 expects)"));
    Ok(())
}

fn command() -> Command {
    let path = std::env::var_os("CARGO_BIN_EXE_basjoofan")
        .map(PathBuf::from)
        .filter(|path| path.is_file())
        .unwrap_or_else(|| {
            let mut path = std::env::current_exe().unwrap();
            path.pop();
            if path.ends_with("deps") {
                path.pop();
            }
            path.join(format!("basjoofan{}", std::env::consts::EXE_SUFFIX))
        });
    Command::new(path)
}
