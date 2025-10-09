use anyhow::{Context, Result};
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Language {
    Python,
    Node,
    Rust,
    Go,
    Ruby,
}

impl std::str::FromStr for Language {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "python" => Ok(Language::Python),
            "node" => Ok(Language::Node),
            "rust" => Ok(Language::Rust),
            "go" => Ok(Language::Go),
            "ruby" => Ok(Language::Ruby),
            _ => anyhow::bail!("Unknown language: {}", s),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct ExecuteReplRequest {
    pub language: Language,
    pub code: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub dependencies: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExecuteReplResponse {
    pub result: String,
    pub success: bool,
}

#[derive(Debug, Deserialize)]
pub struct LanguagesResponse {
    pub languages: Vec<String>,
}

pub struct ReplClient {
    base_url: String,
    client: reqwest::Client,
}

impl ReplClient {
    pub fn new(base_url: String) -> Self {
        Self::with_tls(base_url, super::TlsMode::None)
    }

    pub fn with_tls(base_url: String, tls_mode: super::TlsMode) -> Self {
        let client = match tls_mode {
            super::TlsMode::None => reqwest::Client::new(),
            super::TlsMode::SelfSigned => {
                reqwest::Client::builder()
                    .danger_accept_invalid_certs(true)
                    .build()
                    .expect("Failed to build HTTP client with self-signed cert support")
            }
        };

        Self { base_url, client }
    }

    pub async fn list_languages(&self) -> Result<Vec<String>> {
        let url = format!("{}/api/repl/languages", self.base_url);
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .context("Failed to send list languages request")?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to list languages: {}", error_text);
        }

        let languages_response: LanguagesResponse = response
            .json()
            .await
            .context("Failed to parse list languages response")?;

        Ok(languages_response.languages)
    }

    pub async fn execute(
        &self,
        language: Language,
        code: String,
        dependencies: Vec<String>,
    ) -> Result<ExecuteReplResponse> {
        let url = format!("{}/api/repl/execute", self.base_url);
        let request = ExecuteReplRequest {
            language,
            code,
            dependencies,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send execute REPL request")?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to execute REPL code: {}", error_text);
        }

        let execute_response: ExecuteReplResponse = response
            .json()
            .await
            .context("Failed to parse execute REPL response")?;

        Ok(execute_response)
    }

    pub async fn execute_stream(
        &self,
        language: Language,
        code: String,
        dependencies: Vec<String>,
    ) -> Result<()> {
        let url = format!("{}/api/repl/execute/stream", self.base_url);
        let request = ExecuteReplRequest {
            language,
            code,
            dependencies,
        };

        let response = self
            .client
            .post(&url)
            .json(&request)
            .send()
            .await
            .context("Failed to send execute REPL stream request")?;

        if !response.status().is_success() {
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            anyhow::bail!("Failed to execute REPL code: {}", error_text);
        }

        // Stream the response
        let mut stream = response.bytes_stream();
        let mut buffer = String::new();

        while let Some(chunk_result) = stream.next().await {
            match chunk_result {
                Ok(chunk) => {
                    let text = String::from_utf8_lossy(&chunk);
                    buffer.push_str(&text);

                    // Process complete SSE events
                    while let Some(event_end) = buffer.find("\n\n") {
                        let event: String = buffer.drain(..event_end).collect();
                        buffer.drain(..2); // remove the separator (the +2 part)

                        // Parse SSE event
                        for line in event.lines() {
                            if line.starts_with("data:") {
                                let data = line.strip_prefix("data:").unwrap_or("").trim();
                                if !data.is_empty() {
                                    // Print the output as it streams
                                    if data.starts_with("ERROR:") {
                                        eprintln!("{}", data);
                                    } else {
                                        print!("{}", data);
                                        use std::io::Write;
                                        std::io::stdout().flush().unwrap();
                                    }
                                }
                            } else if line.starts_with("event:") {
                                let event_type = line.strip_prefix("event:").unwrap_or("").trim();
                                if event_type == "done" {
                                    return Ok(());
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    anyhow::bail!("Stream error: {}", e);
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_str_valid() {
        assert!(matches!("python".parse::<Language>().unwrap(), Language::Python));
        assert!(matches!("Python".parse::<Language>().unwrap(), Language::Python));
        assert!(matches!("PYTHON".parse::<Language>().unwrap(), Language::Python));

        assert!(matches!("node".parse::<Language>().unwrap(), Language::Node));
        assert!(matches!("Node".parse::<Language>().unwrap(), Language::Node));

        assert!(matches!("rust".parse::<Language>().unwrap(), Language::Rust));
        assert!(matches!("Rust".parse::<Language>().unwrap(), Language::Rust));

        assert!(matches!("go".parse::<Language>().unwrap(), Language::Go));
        assert!(matches!("Go".parse::<Language>().unwrap(), Language::Go));

        assert!(matches!("ruby".parse::<Language>().unwrap(), Language::Ruby));
        assert!(matches!("Ruby".parse::<Language>().unwrap(), Language::Ruby));
    }

    #[test]
    fn test_language_from_str_invalid() {
        assert!("javascript".parse::<Language>().is_err());
        assert!("java".parse::<Language>().is_err());
        assert!("cpp".parse::<Language>().is_err());
        assert!("".parse::<Language>().is_err());
        assert!("unknown".parse::<Language>().is_err());
    }

    #[test]
    fn test_language_from_str_error_message() {
        let result = "javascript".parse::<Language>();
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Unknown language"));
        assert!(err_msg.contains("javascript"));
    }

    #[test]
    fn test_execute_repl_request_serialization() {
        let request = ExecuteReplRequest {
            language: Language::Python,
            code: "print('hello')".to_string(),
            dependencies: vec![],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("Python"));
        assert!(json.contains("print('hello')"));
        // Empty dependencies should not be serialized
        assert!(!json.contains("dependencies"));
    }

    #[test]
    fn test_execute_repl_request_serialization_with_dependencies() {
        let request = ExecuteReplRequest {
            language: Language::Python,
            code: "import requests".to_string(),
            dependencies: vec!["requests".to_string(), "numpy".to_string()],
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("Python"));
        assert!(json.contains("import requests"));
        assert!(json.contains("dependencies"));
        assert!(json.contains("requests"));
        assert!(json.contains("numpy"));
    }

    #[test]
    fn test_execute_repl_response_deserialization() {
        let json = r#"{"result":"Output here","success":true}"#;
        let response: ExecuteReplResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.result, "Output here");
        assert!(response.success);
    }

    #[test]
    fn test_execute_repl_response_deserialization_failure() {
        let json = r#"{"result":"Error message","success":false}"#;
        let response: ExecuteReplResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.result, "Error message");
        assert!(!response.success);
    }

    #[test]
    fn test_languages_response_deserialization() {
        let json = r#"{"languages":["Python","Node","Rust","Go","Ruby"]}"#;
        let response: LanguagesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.languages.len(), 5);
        assert_eq!(response.languages[0], "Python");
        assert_eq!(response.languages[4], "Ruby");
    }

    #[test]
    fn test_repl_client_creation() {
        let client = ReplClient::new("http://localhost:3001".to_string());
        assert_eq!(client.base_url, "http://localhost:3001");
    }

    #[test]
    fn test_language_serialization() {
        assert_eq!(serde_json::to_string(&Language::Python).unwrap(), r#""Python""#);
        assert_eq!(serde_json::to_string(&Language::Node).unwrap(), r#""Node""#);
        assert_eq!(serde_json::to_string(&Language::Rust).unwrap(), r#""Rust""#);
        assert_eq!(serde_json::to_string(&Language::Go).unwrap(), r#""Go""#);
        assert_eq!(serde_json::to_string(&Language::Ruby).unwrap(), r#""Ruby""#);
    }

    #[test]
    fn test_language_deserialization() {
        assert!(matches!(
            serde_json::from_str::<Language>(r#""Python""#).unwrap(),
            Language::Python
        ));
        assert!(matches!(
            serde_json::from_str::<Language>(r#""Node""#).unwrap(),
            Language::Node
        ));
        assert!(matches!(
            serde_json::from_str::<Language>(r#""Rust""#).unwrap(),
            Language::Rust
        ));
        assert!(matches!(
            serde_json::from_str::<Language>(r#""Go""#).unwrap(),
            Language::Go
        ));
        assert!(matches!(
            serde_json::from_str::<Language>(r#""Ruby""#).unwrap(),
            Language::Ruby
        ));
    }
}