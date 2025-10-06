pub mod container;
pub mod repl;

use clap::{Parser, Subcommand};
use container::ContainerClient;
use repl::{Language, ReplClient};

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
    },
    /// Remove a container
    Remove {
        /// Container ID to remove
        #[arg(short, long)]
        id: String,
        /// Container API URL
        #[arg(long, default_value = "http://localhost:3000")]
        api_url: String,
    },
}

#[derive(Subcommand)]
enum ReplCommands {
    /// List available languages
    Languages {
        /// REPL API URL
        #[arg(long, default_value = "http://localhost:3001")]
        api_url: String,
    },
    /// Execute code in a REPL
    Execute {
        /// Programming language
        #[arg(short, long)]
        language: String,
        /// Code to execute
        #[arg(short, long)]
        code: String,
        /// REPL API URL
        #[arg(long, default_value = "http://localhost:3001")]
        api_url: String,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Container { command } => match command {
            ContainerCommands::List { api_url } => {
                let client = ContainerClient::new(api_url);
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
            } => {
                let client = ContainerClient::new(api_url);
                println!("Creating container with image: {}", image);
                if let Some(ref cmd) = command {
                    println!("Command: {}", cmd.join(" "));
                }

                let response = client.create_container(image, command).await?;
                println!("✓ {}", response.message);
                println!("Container ID: {}", response.id);
            }
            ContainerCommands::Remove { id, api_url } => {
                let client = ContainerClient::new(api_url);
                println!("Removing container: {}", id);

                let response = client.remove_container(id).await?;
                println!("✓ {}", response.message);
            }
        },
        Commands::Repl { command } => match command {
            ReplCommands::Languages { api_url } => {
                let client = ReplClient::new(api_url);
                let languages = client.list_languages().await?;

                println!("Available languages:");
                for lang in languages {
                    println!("  - {}", lang);
                }
            }
            ReplCommands::Execute {
                language,
                code,
                api_url,
            } => {
                let client = ReplClient::new(api_url);
                let lang: Language = language.parse()?;

                println!("Executing {} code...", language);
                let response = client.execute(lang, code).await?;

                if response.success {
                    println!("✓ Success!");
                    println!("{}", response.result);
                } else {
                    println!("✗ Failed!");
                    println!("{}", response.result);
                }
            }
        },
    }

    Ok(())
}