mod security;
pub use security::{validate_code, CodeValidationResult, SecurityViolation};

use anyhow::{Context, Result};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::response::sse::{Event, Sse};
use axum::Json;
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use service_registry::get_service_endpoint;
use std::collections::HashMap;
use std::convert::Infallible;

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

    pub fn install_dependencies_command(&self, dependencies: &[String]) -> Option<String> {
        if dependencies.is_empty() {
            return None;
        }

        let deps = dependencies.join(" ");
        let cmd = match self {
            Language::Python => format!("pip install --quiet {}", deps),
            Language::Node => format!("npm install --global --quiet {}", deps),
            Language::Rust => {
                // For Rust, we install binaries via cargo install
                format!("cargo install --quiet {}", deps)
            }
            Language::Go => {
                // For Go, we use go get (deprecated in Go 1.17+ but still works for simple cases)
                // In a real implementation, you might want to initialize a Go module
                format!("go install {}@latest", deps)
            }
            Language::Ruby => format!("gem install --quiet {}", deps),
        };

        Some(cmd)
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

    pub fn build_command_with_dependencies(
        &self,
        code: &str,
        dependencies: &[String],
    ) -> Vec<String> {
        let install_cmd = self.install_dependencies_command(dependencies);
        let execute_cmd_parts = self.execute_command(code);

        match install_cmd {
            Some(install) => {
                // Combine install and execute commands
                let combined = if execute_cmd_parts[0] == "sh" && execute_cmd_parts.len() == 3 {
                    // Already using shell, append to existing command
                    format!("{} && {}", install, execute_cmd_parts[2])
                } else {
                    // Need to wrap in shell
                    let exec_part = execute_cmd_parts.join(" ");
                    format!("{} && {}", install, exec_part)
                };

                vec!["sh".to_string(), "-c".to_string(), combined]
            }
            None => execute_cmd_parts,
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
    output: Option<String>,
}

impl ReplSession {
    pub fn new(language: Language) -> Self {
        Self::new_with_endpoint(language, None)
    }

    pub fn new_with_endpoint(language: Language, endpoint: Option<String>) -> Self {
        Self {
            language,
            containers_api_url: endpoint.unwrap_or_else(|| {
                std::env::var("CONTAINERS_API_URL")
                    .unwrap_or_else(|_| "http://localhost:3000".to_string())
            }),
            session_variables: HashMap::new(),
        }
    }

    pub async fn execute(&mut self, code: &str) -> Result<String> {
        self.execute_with_dependencies(code, &[]).await
    }

    pub async fn execute_with_dependencies(
        &mut self,
        code: &str,
        dependencies: &[String],
    ) -> Result<String> {
        let client = reqwest::Client::new();

        let request = CreateContainerRequest {
            image: self.language.container_image().to_string(),
            command: self
                .language
                .build_command_with_dependencies(code, dependencies),
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

        Ok(container_response.output.unwrap_or_else(|| {
            format!(
                "Executed in container {}: {}",
                container_response.id, container_response.message
            )
        }))
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
    #[serde(default)]
    pub dependencies: Vec<String>,
}

#[derive(Serialize)]
pub struct ExecuteReplResponse {
    pub result: String,
    pub success: bool,
}

pub async fn execute_repl(Json(payload): Json<ExecuteReplRequest>) -> impl IntoResponse {
    // Validate code for security violations
    let language_str = format!("{:?}", payload.language);
    let validation = validate_code(&payload.code, &language_str, &payload.dependencies);

    if !validation.is_safe {
        let violations_msg = validation
            .violations
            .iter()
            .filter(|v| v.should_block)
            .map(|v| v.description.clone())
            .collect::<Vec<_>>()
            .join("; ");

        tracing::warn!(
            "Code execution blocked due to security violations: {}",
            violations_msg
        );

        return (
            StatusCode::FORBIDDEN,
            Json(ExecuteReplResponse {
                result: format!("Code execution blocked: {}", violations_msg),
                success: false,
            }),
        )
            .into_response();
    }

    // Log warnings for non-blocking violations
    for violation in validation.violations.iter().filter(|v| !v.should_block) {
        tracing::warn!("Security warning: {}", violation.description);
    }

    // Try to get container-api endpoint from service registry
    let endpoint = get_service_endpoint("container-api").await;

    let mut session = ReplSession::new_with_endpoint(payload.language, endpoint);

    match session
        .execute_with_dependencies(&payload.code, &payload.dependencies)
        .await
    {
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

pub async fn execute_repl_stream(
    Json(payload): Json<ExecuteReplRequest>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let stream = async_stream::stream! {
        // Validate code for security violations
        let language_str = format!("{:?}", payload.language);
        let validation = validate_code(&payload.code, &language_str, &payload.dependencies);

        if !validation.is_safe {
            let violations_msg = validation
                .violations
                .iter()
                .filter(|v| v.should_block)
                .map(|v| v.description.clone())
                .collect::<Vec<_>>()
                .join("; ");

            tracing::warn!(
                "Code execution blocked due to security violations: {}",
                violations_msg
            );

            yield Ok(Event::default().data(format!("ERROR: Code execution blocked: {}", violations_msg)));
            return;
        }

        // Log warnings for non-blocking violations
        for violation in validation.violations.iter().filter(|v| !v.should_block) {
            tracing::warn!("Security warning: {}", violation.description);
        }

        // Try to get container-api endpoint from service registry
        let endpoint = get_service_endpoint("container-api").await;
        let containers_api_url = endpoint.unwrap_or_else(|| {
            std::env::var("CONTAINERS_API_URL")
                .unwrap_or_else(|_| "http://localhost:3000".to_string())
        });

        let request = CreateContainerRequest {
            image: payload.language.container_image().to_string(),
            command: payload
                .language
                .build_command_with_dependencies(&payload.code, &payload.dependencies),
        };

        let client = reqwest::Client::new();
        let response = match client
            .post(format!("{}/api/containers/create/stream", containers_api_url))
            .json(&request)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                yield Ok(Event::default().data(format!("ERROR: Failed to connect to container API: {}", e)));
                return;
            }
        };

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            yield Ok(Event::default().data(format!("ERROR: Container execution failed: {}", error_text)));
            return;
        }

        // Stream the SSE events from the container API
        let mut event_source = response.bytes_stream();
        use futures_util::StreamExt;

        while let Some(chunk_result) = event_source.next().await {
            match chunk_result {
                Ok(chunk) => {
                    let text = String::from_utf8_lossy(&chunk);
                    // Forward the SSE data
                    for line in text.lines() {
                        if line.starts_with("data:") {
                            let data = line.strip_prefix("data:").unwrap_or("").trim();
                            if !data.is_empty() {
                                yield Ok(Event::default().data(data.to_string()));
                            }
                        } else if line.starts_with("event:") {
                            // Handle event type if needed
                            let event_type = line.strip_prefix("event:").unwrap_or("").trim();
                            if event_type == "done" {
                                yield Ok(Event::default().event("done").data(""));
                                break;
                            }
                        }
                    }
                }
                Err(e) => {
                    yield Ok(Event::default().data(format!("ERROR: Stream error: {}", e)));
                    break;
                }
            }
        }
    };

    Sse::new(stream)
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

    #[test]
    fn test_install_dependencies_command_python() {
        let deps = vec!["requests".to_string(), "numpy".to_string()];
        let cmd = Language::Python.install_dependencies_command(&deps);
        assert_eq!(cmd, Some("pip install --quiet requests numpy".to_string()));
    }

    #[test]
    fn test_install_dependencies_command_node() {
        let deps = vec!["express".to_string(), "lodash".to_string()];
        let cmd = Language::Node.install_dependencies_command(&deps);
        assert_eq!(
            cmd,
            Some("npm install --global --quiet express lodash".to_string())
        );
    }

    #[test]
    fn test_install_dependencies_command_ruby() {
        let deps = vec!["rails".to_string(), "sinatra".to_string()];
        let cmd = Language::Ruby.install_dependencies_command(&deps);
        assert_eq!(cmd, Some("gem install --quiet rails sinatra".to_string()));
    }

    #[test]
    fn test_install_dependencies_command_rust() {
        let deps = vec!["ripgrep".to_string()];
        let cmd = Language::Rust.install_dependencies_command(&deps);
        assert_eq!(cmd, Some("cargo install --quiet ripgrep".to_string()));
    }

    #[test]
    fn test_install_dependencies_command_go() {
        let deps = vec!["github.com/spf13/cobra".to_string()];
        let cmd = Language::Go.install_dependencies_command(&deps);
        assert_eq!(
            cmd,
            Some("go install github.com/spf13/cobra@latest".to_string())
        );
    }

    #[test]
    fn test_install_dependencies_command_empty() {
        let deps: Vec<String> = vec![];
        let cmd = Language::Python.install_dependencies_command(&deps);
        assert_eq!(cmd, None);
    }

    #[test]
    fn test_build_command_with_dependencies_python() {
        let code = "import requests; print('hello')";
        let deps = vec!["requests".to_string()];
        let cmd = Language::Python.build_command_with_dependencies(code, &deps);
        assert_eq!(cmd.len(), 3);
        assert_eq!(cmd[0], "sh");
        assert_eq!(cmd[1], "-c");
        assert!(cmd[2].contains("pip install --quiet requests"));
        assert!(cmd[2].contains("python -c"));
    }

    #[test]
    fn test_build_command_with_dependencies_node() {
        let code = "console.log('hello')";
        let deps = vec!["lodash".to_string()];
        let cmd = Language::Node.build_command_with_dependencies(code, &deps);
        assert_eq!(cmd.len(), 3);
        assert_eq!(cmd[0], "sh");
        assert_eq!(cmd[1], "-c");
        assert!(cmd[2].contains("npm install --global --quiet lodash"));
        assert!(cmd[2].contains("node -e"));
    }

    #[test]
    fn test_build_command_with_dependencies_no_deps() {
        let code = "print('hello')";
        let deps: Vec<String> = vec![];
        let cmd = Language::Python.build_command_with_dependencies(code, &deps);
        // Should be the same as execute_command when no dependencies
        let expected = Language::Python.execute_command(code);
        assert_eq!(cmd, expected);
    }

    #[test]
    fn test_build_command_with_dependencies_go() {
        let code = "package main\nfunc main() { println(\"hello\") }";
        let deps = vec!["github.com/spf13/cobra".to_string()];
        let cmd = Language::Go.build_command_with_dependencies(code, &deps);
        assert_eq!(cmd.len(), 3);
        assert_eq!(cmd[0], "sh");
        assert_eq!(cmd[1], "-c");
        assert!(cmd[2].contains("go install github.com/spf13/cobra@latest"));
        assert!(cmd[2].contains("go run"));
    }
}