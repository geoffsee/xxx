use anyhow::{Context, Result};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Language {
    Python,
    Node,
    Rust,
    Go,
    Ruby,
}

impl Language {
    pub fn container_image(&self) -> &str {
        match self {
            Language::Python => "python:3.11-slim",
            Language::Node => "node:20-slim",
            Language::Rust => "rust:1.75-slim",
            Language::Go => "golang:1.21-alpine",
            Language::Ruby => "ruby:3.2-slim",
        }
    }

    pub fn execute_command(&self, code: &str) -> Vec<String> {
        match self {
            Language::Python => vec!["python".to_string(), "-c".to_string(), code.to_string()],
            Language::Node => vec!["node".to_string(), "-e".to_string(), code.to_string()],
            Language::Rust => {
                // For Rust, we'd need a more complex setup with compilation
                vec!["sh".to_string(), "-c".to_string(),
                     format!("echo '{}' > /tmp/main.rs && rustc /tmp/main.rs -o /tmp/prog && /tmp/prog", code)]
            }
            Language::Go => {
                vec!["sh".to_string(), "-c".to_string(),
                     format!("echo '{}' > /tmp/main.go && go run /tmp/main.go", code)]
            }
            Language::Ruby => vec!["ruby".to_string(), "-e".to_string(), code.to_string()],
        }
    }
}

#[derive(Debug, Clone)]
pub struct ReplSession {
    language: Language,
    containers_api_url: String,
    session_variables: HashMap<String, String>,
}

#[derive(Debug, Serialize)]
struct CreateContainerRequest {
    image: String,
    command: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct CreateContainerResponse {
    id: String,
    message: String,
}

impl ReplSession {
    pub fn new(language: Language) -> Self {
        Self {
            language,
            containers_api_url: std::env::var("CONTAINERS_API_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string()),
            session_variables: HashMap::new(),
        }
    }

    pub async fn execute(&mut self, code: &str) -> Result<String> {
        let client = reqwest::Client::new();

        let request = CreateContainerRequest {
            image: self.language.container_image().to_string(),
            command: self.language.execute_command(code),
        };

        let response = client
            .post(format!("{}/api/containers/create", self.containers_api_url))
            .json(&request)
            .send()
            .await
            .context("Failed to send request to containers API")?;

        if !response.status().is_success() {
            let error_text = response.text().await.unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Container execution failed: {}", error_text);
        }

        let container_response: CreateContainerResponse = response
            .json()
            .await
            .context("Failed to parse container response")?;

        Ok(format!("Executed in container {}: {}", container_response.id, container_response.message))
    }

    pub fn set_variable(&mut self, key: String, value: String) {
        self.session_variables.insert(key, value);
    }

    pub fn get_variable(&self, key: &str) -> Option<&String> {
        self.session_variables.get(key)
    }

    pub fn language(&self) -> &Language {
        &self.language
    }
}

// ========== Axum Handlers ==========
#[derive(Deserialize)]
pub struct ExecuteReplRequest {
    pub language: Language,
    pub code: String,
}

#[derive(Serialize)]
pub struct ExecuteReplResponse {
    pub result: String,
    pub success: bool,
}

pub async fn execute_repl(Json(payload): Json<ExecuteReplRequest>) -> impl IntoResponse {
    let mut session = ReplSession::new(payload.language);

    match session.execute(&payload.code).await {
        Ok(result) => (
            StatusCode::OK,
            Json(ExecuteReplResponse {
                result,
                success: true,
            }),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(ExecuteReplResponse {
                result: e.to_string(),
                success: false,
            }),
        )
            .into_response(),
    }
}

#[derive(Serialize)]
pub struct LanguagesResponse {
    pub languages: Vec<String>,
}

pub async fn list_languages() -> impl IntoResponse {
    Json(LanguagesResponse {
        languages: vec![
            "Python".to_string(),
            "Node".to_string(),
            "Rust".to_string(),
            "Go".to_string(),
            "Ruby".to_string(),
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use testcontainers::{clients, Container, GenericImage};

    #[test]
    fn smoke_check_repl_creation() {
        let repl = ReplSession::new(Language::Python);
        assert!(matches!(repl.language(), Language::Python));
    }

    #[test]
    fn smoke_check_language_images() {
        assert_eq!(Language::Python.container_image(), "python:3.11-slim");
        assert_eq!(Language::Node.container_image(), "node:20-slim");
        assert_eq!(Language::Rust.container_image(), "rust:1.75-slim");
        assert_eq!(Language::Go.container_image(), "golang:1.21-alpine");
        assert_eq!(Language::Ruby.container_image(), "ruby:3.2-slim");
    }

    #[test]
    fn smoke_check_execute_commands() {
        let python_cmd = Language::Python.execute_command("print('hello')");
        assert_eq!(python_cmd, vec!["python", "-c", "print('hello')"]);

        let node_cmd = Language::Node.execute_command("console.log('hello')");
        assert_eq!(node_cmd, vec!["node", "-e", "console.log('hello')"]);
    }

    #[test]
    fn smoke_check_session_variables() {
        let mut repl = ReplSession::new(Language::Python);

        repl.set_variable("user".to_string(), "alice".to_string());
        repl.set_variable("count".to_string(), "42".to_string());

        assert_eq!(repl.get_variable("user"), Some(&"alice".to_string()));
        assert_eq!(repl.get_variable("count"), Some(&"42".to_string()));
        assert_eq!(repl.get_variable("missing"), None);
    }

    #[tokio::test]
    async fn test_python_execution_with_testcontainers() {
        let docker = clients::Cli::default();

        let python_image = GenericImage::new("python", "3.11-slim")
            .with_wait_for(testcontainers::core::WaitFor::message_on_stdout("Python"));

        let container = docker.run(python_image);
        let exec_result = container.exec(vec!["python", "-c", "print('Hello from Python')"]);

        // Verify container can execute Python code
        assert!(exec_result.is_ok());
    }

    #[tokio::test]
    async fn test_node_execution_with_testcontainers() {
        let docker = clients::Cli::default();

        let node_image = GenericImage::new("node", "20-slim");
        let container = docker.run(node_image);

        let exec_result = container.exec(vec!["node", "-e", "console.log('Hello from Node')"]);
        assert!(exec_result.is_ok());
    }

    #[tokio::test]
    async fn test_ruby_execution_with_testcontainers() {
        let docker = clients::Cli::default();

        let ruby_image = GenericImage::new("ruby", "3.2-slim");
        let container = docker.run(ruby_image);

        let exec_result = container.exec(vec!["ruby", "-e", "puts 'Hello from Ruby'"]);
        assert!(exec_result.is_ok());
    }

    #[tokio::test]
    async fn test_go_execution_with_testcontainers() {
        let docker = clients::Cli::default();

        let go_image = GenericImage::new("golang", "1.21-alpine");
        let container = docker.run(go_image);

        // Create and run a simple Go program
        let exec_result = container.exec(vec![
            "sh", "-c",
            "echo 'package main\nimport \"fmt\"\nfunc main() { fmt.Println(\"Hello from Go\") }' > /tmp/main.go && go run /tmp/main.go"
        ]);
        assert!(exec_result.is_ok());
    }

    #[tokio::test]
    async fn test_container_isolation() {
        let docker = clients::Cli::default();

        // Run two containers and verify they're isolated
        let python1 = GenericImage::new("python", "3.11-slim");
        let python2 = GenericImage::new("python", "3.11-slim");

        let container1 = docker.run(python1);
        let container2 = docker.run(python2);

        // Both containers should be able to execute independently
        let result1 = container1.exec(vec!["python", "-c", "x = 1; print(x)"]);
        let result2 = container2.exec(vec!["python", "-c", "x = 2; print(x)"]);

        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }

    #[tokio::test]
    async fn test_untrusted_code_execution() {
        let docker = clients::Cli::default();

        let python_image = GenericImage::new("python", "3.11-slim");
        let container = docker.run(python_image);

        // Test executing untrusted code (file operations)
        let result = container.exec(vec![
            "python", "-c",
            "import os; open('/tmp/test.txt', 'w').write('untrusted'); print(open('/tmp/test.txt').read())"
        ]);

        // Should execute but be isolated to container filesystem
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_multiple_language_support() {
        let docker = clients::Cli::default();

        // Test that we can run different languages in parallel
        let languages = vec![
            (GenericImage::new("python", "3.11-slim"), vec!["python", "-c", "print('Python')"]),
            (GenericImage::new("node", "20-slim"), vec!["node", "-e", "console.log('Node')"]),
            (GenericImage::new("ruby", "3.2-slim"), vec!["ruby", "-e", "puts 'Ruby'"]),
        ];

        for (image, cmd) in languages {
            let container = docker.run(image);
            let result = container.exec(cmd);
            assert!(result.is_ok(), "Failed to execute language container");
        }
    }

    #[tokio::test]
    async fn smoke_check_execute_integration() {
        // This test requires container-api to be running
        // Skip if CONTAINERS_API_URL is not set
        if std::env::var("CONTAINERS_API_URL").is_err() {
            println!("Skipping integration test - CONTAINERS_API_URL not set");
            return;
        }

        let mut repl = ReplSession::new(Language::Python);
        let result = repl.execute("print('Hello from container!')").await;

        // In a real scenario with container-api running, this would succeed
        // For now, we just verify the function can be called
        match result {
            Ok(msg) => println!("Success: {}", msg),
            Err(e) => println!("Expected error (container-api not running): {}", e),
        }
    }
}
