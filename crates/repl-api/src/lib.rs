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