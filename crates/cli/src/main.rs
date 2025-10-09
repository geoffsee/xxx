use clap::{Parser, Subcommand};
use cli::container::ContainerClient;
use cli::repl::{Language, ReplClient};
use cli::TlsMode;

#[derive(Parser)]
#[command(name = "xxx-cli")]
#[command(about = "CLI for interacting with container and REPL APIs", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Container management commands
    Container {
        #[command(subcommand)]
        command: ContainerCommands,
    },
    /// REPL execution commands
    Repl {
        #[command(subcommand)]
        command: ReplCommands,
    },
}

#[derive(Subcommand)]
enum ContainerCommands {
    /// List all containers
    List {
        /// Container API URL
        #[arg(long, default_value = "http://localhost:3000")]
        api_url: String,
        /// TLS mode (none or self-signed)
        #[arg(long, value_enum, default_value = "none")]
        tls: TlsMode,
    },
    /// Create and start a new container
    Create {
        /// Container image to use
        #[arg(short, long)]
        image: String,
        /// Command to run in the container
        #[arg(short, long, value_delimiter = ' ', num_args = 1..)]
        command: Option<Vec<String>>,
        /// Container API URL
        #[arg(long, default_value = "http://localhost:3000")]
        api_url: String,
        /// TLS mode (none or self-signed)
        #[arg(long, value_enum, default_value = "none")]
        tls: TlsMode,
    },
    /// Remove a container
    Remove {
        /// Container ID to remove
        #[arg(short, long)]
        id: String,
        /// Container API URL
        #[arg(long, default_value = "http://localhost:3000")]
        api_url: String,
        /// TLS mode (none or self-signed)
        #[arg(long, value_enum, default_value = "none")]
        tls: TlsMode,
    },
}

#[derive(Subcommand)]
enum ReplCommands {
    /// List available languages
    Languages {
        /// REPL API URL
        #[arg(long, default_value = "http://localhost:3001")]
        api_url: String,
        /// TLS mode (none or self-signed)
        #[arg(long, value_enum, default_value = "none")]
        tls: TlsMode,
    },
    /// Execute code in a REPL
    Execute {
        /// Programming language
        #[arg(short, long)]
        language: String,
        /// Code to execute
        #[arg(short, long)]
        code: String,
        /// Dependencies to install (can be specified multiple times)
        #[arg(short, long)]
        dependencies: Vec<String>,
        /// REPL API URL
        #[arg(long, default_value = "http://localhost:3001")]
        api_url: String,
        /// TLS mode (none or self-signed)
        #[arg(long, value_enum, default_value = "none")]
        tls: TlsMode,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Container { command } => match command {
            ContainerCommands::List { api_url, tls } => {
                let client = ContainerClient::with_tls(api_url, tls);
                let containers = client.list_containers().await?;

                if containers.is_empty() {
                    println!("No containers found");
                } else {
                    println!("Containers:");
                    for (i, names) in containers.iter().enumerate() {
                        println!("  {}. {}", i + 1, names.join(", "));
                    }
                }
            }
            ContainerCommands::Create {
                image,
                command,
                api_url,
                tls,
            } => {
                let client = ContainerClient::with_tls(api_url, tls);
                println!("Creating container with image: {}", image);
                if let Some(ref cmd) = command {
                    println!("Command: {}", cmd.join(" "));
                }

                let response = client.create_container(image, command).await?;
                println!("✓ {}", response.message);
                println!("Container ID: {}", response.id);
            }
            ContainerCommands::Remove { id, api_url, tls } => {
                let client = ContainerClient::with_tls(api_url, tls);
                println!("Removing container: {}", id);

                let response = client.remove_container(id).await?;
                println!("✓ {}", response.message);
            }
        },
        Commands::Repl { command } => match command {
            ReplCommands::Languages { api_url, tls } => {
                let client = ReplClient::with_tls(api_url, tls);
                let languages = client.list_languages().await?;

                println!("Available languages:");
                for lang in languages {
                    println!("  - {}", lang);
                }
            }
            ReplCommands::Execute {
                language,
                code,
                dependencies,
                api_url,
                tls,
            } => {
                let client = ReplClient::with_tls(api_url, tls);
                let lang: Language = language.parse()?;

                if !dependencies.is_empty() {
                    println!("Installing dependencies: {}", dependencies.join(", "));
                }
                println!("Executing {} code...", language);
                client.execute_stream(lang, code, dependencies).await?;
                println!(); // Add newline after streaming output
            }
        },
    }

    Ok(())
}