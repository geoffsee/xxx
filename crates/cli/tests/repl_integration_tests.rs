use mockito::{Server, ServerGuard};

// Re-export the repl module types for testing
mod repl {
    pub use cli::repl::*;
}

use repl::{Language, ReplClient};

async fn setup_mock_server() -> ServerGuard {
    Server::new_async().await
}

#[tokio::test]
async fn test_list_languages_success() {
    let mut server = setup_mock_server().await;

    let mock = server
        .mock("GET", "/api/repl/languages")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"languages":["Python","Node","Rust","Go","Ruby"]}"#)
        .create_async()
        .await;

    let client = ReplClient::new(server.url());
    let result = client.list_languages().await;

    mock.assert_async().await;
    assert!(result.is_ok());
    let languages = result.unwrap();
    assert_eq!(languages.len(), 5);
    assert_eq!(languages[0], "Python");
    assert_eq!(languages[4], "Ruby");
}

#[tokio::test]
async fn test_list_languages_empty() {
    let mut server = setup_mock_server().await;

    let mock = server
        .mock("GET", "/api/repl/languages")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"languages":[]}"#)
        .create_async()
        .await;

    let client = ReplClient::new(server.url());
    let result = client.list_languages().await;

    mock.assert_async().await;
    assert!(result.is_ok());
    let languages = result.unwrap();
    assert!(languages.is_empty());
}

#[tokio::test]
async fn test_list_languages_error() {
    let mut server = setup_mock_server().await;

    let mock = server
        .mock("GET", "/api/repl/languages")
        .with_status(503)
        .with_body("Service unavailable")
        .create_async()
        .await;

    let client = ReplClient::new(server.url());
    let result = client.list_languages().await;

    mock.assert_async().await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Failed to list languages"));
}

#[tokio::test]
async fn test_execute_repl_python_success() {
    let mut server = setup_mock_server().await;

    let mock = server
        .mock("POST", "/api/repl/execute")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"result":"Executed successfully","success":true}"#)
        .create_async()
        .await;

    let client = ReplClient::new(server.url());
    let result = client
        .execute(Language::Python, "print('hello')".to_string())
        .await;

    mock.assert_async().await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.success);
    assert_eq!(response.result, "Executed successfully");
}

#[tokio::test]
async fn test_execute_repl_node_success() {
    let mut server = setup_mock_server().await;

    let mock = server
        .mock("POST", "/api/repl/execute")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"result":"Output: hello","success":true}"#)
        .create_async()
        .await;

    let client = ReplClient::new(server.url());
    let result = client
        .execute(Language::Node, "console.log('hello')".to_string())
        .await;

    mock.assert_async().await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.success);
    assert_eq!(response.result, "Output: hello");
}

#[tokio::test]
async fn test_execute_repl_failure() {
    let mut server = setup_mock_server().await;

    let mock = server
        .mock("POST", "/api/repl/execute")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"result":"Syntax error in code","success":false}"#)
        .create_async()
        .await;

    let client = ReplClient::new(server.url());
    let result = client
        .execute(Language::Python, "invalid python code".to_string())
        .await;

    mock.assert_async().await;
    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(!response.success);
    assert_eq!(response.result, "Syntax error in code");
}

#[tokio::test]
async fn test_execute_repl_server_error() {
    let mut server = setup_mock_server().await;

    let mock = server
        .mock("POST", "/api/repl/execute")
        .with_status(500)
        .with_body("Internal server error")
        .create_async()
        .await;

    let client = ReplClient::new(server.url());
    let result = client
        .execute(Language::Rust, "fn main() {}".to_string())
        .await;

    mock.assert_async().await;
    assert!(result.is_err());
    let err_msg = result.unwrap_err().to_string();
    assert!(err_msg.contains("Failed to execute REPL code"));
}

#[tokio::test]
async fn test_execute_repl_all_languages() {
    let mut server = setup_mock_server().await;

    // Test Python
    let mock_python = server
        .mock("POST", "/api/repl/execute")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"result":"Python output","success":true}"#)
        .create_async()
        .await;

    let client = ReplClient::new(server.url());
    let result = client
        .execute(Language::Python, "print('test')".to_string())
        .await;
    assert!(result.is_ok());
    mock_python.assert_async().await;

    // Test Node
    let mock_node = server
        .mock("POST", "/api/repl/execute")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"result":"Node output","success":true}"#)
        .create_async()
        .await;

    let result = client
        .execute(Language::Node, "console.log('test')".to_string())
        .await;
    assert!(result.is_ok());
    mock_node.assert_async().await;

    // Test Rust
    let mock_rust = server
        .mock("POST", "/api/repl/execute")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"result":"Rust output","success":true}"#)
        .create_async()
        .await;

    let result = client
        .execute(Language::Rust, "fn main() {}".to_string())
        .await;
    assert!(result.is_ok());
    mock_rust.assert_async().await;

    // Test Go
    let mock_go = server
        .mock("POST", "/api/repl/execute")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"result":"Go output","success":true}"#)
        .create_async()
        .await;

    let result = client
        .execute(Language::Go, "package main".to_string())
        .await;
    assert!(result.is_ok());
    mock_go.assert_async().await;

    // Test Ruby
    let mock_ruby = server
        .mock("POST", "/api/repl/execute")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(r#"{"result":"Ruby output","success":true}"#)
        .create_async()
        .await;

    let result = client
        .execute(Language::Ruby, "puts 'test'".to_string())
        .await;
    assert!(result.is_ok());
    mock_ruby.assert_async().await;
}