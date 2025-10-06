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
                vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    format!(
                        "echo '{}' > /tmp/main.rs && rustc /tmp/main.rs -o /tmp/prog && /tmp/prog",
                        code
                    ),
                ]
            }
            Language::Go => {
                vec![
                    "sh".to_string(),
                    "-c".to_string(),
                    format!("echo '{}' > /tmp/main.go && go run /tmp/main.go", code),
                ]
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
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Container execution failed: {}", error_text);
        }

        let container_response: CreateContainerResponse = response
            .json()
            .await
            .context("Failed to parse container response")?;

        Ok(format!(
            "Executed in container {}: {}",
            container_response.id, container_response.message
        ))
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

    #[test]
    fn test_language_container_image() {
        assert_eq!(Language::Python.container_image(), "python:3.11-slim");
        assert_eq!(Language::Node.container_image(), "node:20-slim");
        assert_eq!(Language::Rust.container_image(), "rust:1.75-slim");
        assert_eq!(Language::Go.container_image(), "golang:1.21-alpine");
        assert_eq!(Language::Ruby.container_image(), "ruby:3.2-slim");
    }

    #[test]
    fn test_language_execute_command_python() {
        let code = "print('hello')";
        let command = Language::Python.execute_command(code);
        assert_eq!(command, vec!["python", "-c", "print('hello')"]);
    }

    #[test]
    fn test_language_execute_command_node() {
        let code = "console.log('hello')";
        let command = Language::Node.execute_command(code);
        assert_eq!(command, vec!["node", "-e", "console.log('hello')"]);
    }

    #[test]
    fn test_language_execute_command_ruby() {
        let code = "puts 'hello'";
        let command = Language::Ruby.execute_command(code);
        assert_eq!(command, vec!["ruby", "-e", "puts 'hello'"]);
    }

    #[test]
    fn test_language_execute_command_rust() {
        let code = "fn main() { println!(\"hello\"); }";
        let command = Language::Rust.execute_command(code);
        assert_eq!(command.len(), 3);
        assert_eq!(command[0], "sh");
        assert_eq!(command[1], "-c");
        assert!(command[2].contains("rustc"));
        assert!(command[2].contains("/tmp/main.rs"));
    }

    #[test]
    fn test_language_execute_command_go() {
        let code = "package main\nfunc main() { println(\"hello\") }";
        let command = Language::Go.execute_command(code);
        assert_eq!(command.len(), 3);
        assert_eq!(command[0], "sh");
        assert_eq!(command[1], "-c");
        assert!(command[2].contains("go run"));
        assert!(command[2].contains("/tmp/main.go"));
    }

    #[test]
    fn test_repl_session_new() {
        let session = ReplSession::new(Language::Python);
        assert!(matches!(session.language(), Language::Python));
    }

    #[test]
    fn test_repl_session_variables() {
        let mut session = ReplSession::new(Language::Python);

        // Test setting and getting variables
        session.set_variable("x".to_string(), "42".to_string());
        assert_eq!(session.get_variable("x"), Some(&"42".to_string()));

        // Test getting non-existent variable
        assert_eq!(session.get_variable("y"), None);

        // Test overwriting variable
        session.set_variable("x".to_string(), "100".to_string());
        assert_eq!(session.get_variable("x"), Some(&"100".to_string()));
    }

    #[test]
    fn test_repl_session_language_getter() {
        let session = ReplSession::new(Language::Ruby);
        assert!(matches!(session.language(), Language::Ruby));
    }
}