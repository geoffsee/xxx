pub mod container;
pub mod repl;

use clap::ValueEnum;

#[derive(Debug, Clone, ValueEnum)]
pub enum TlsMode {
    /// No TLS (HTTP)
    None,
    /// Accept self-signed certificates (HTTPS)
    SelfSigned,
}