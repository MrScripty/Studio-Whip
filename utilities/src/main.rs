use anyhow::Result;
use clap::{Parser, Subcommand};
use log::info;
use std::path::PathBuf;

mod parser;
mod config;

use parser::CodeParser;
use config::ParserConfig;

#[derive(Parser)]
#[command(name = "whip-doc-parser")]
#[command(about = "A configurable Rust code parser for Studio-Whip documentation generation")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[derive(Subcommand)]
enum Commands {
    /// Parse project and generate architecture documentation
    Architecture {
        /// Path to the whip_ui project
        #[arg(short, long, default_value = "../whip_ui")]
        project_path: PathBuf,
        
        /// Output format (json, markdown)
        #[arg(short, long, default_value = "markdown")]
        format: String,
    },
    /// Parse project with custom configuration
    Parse {
        /// Path to the whip_ui project
        #[arg(short, long, default_value = "../whip_ui")]
        project_path: PathBuf,
        
        /// Configuration file path
        #[arg(short, long)]
        config: Option<PathBuf>,
        
        /// Output format (json, markdown)
        #[arg(short, long, default_value = "json")]
        format: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Initialize logging
    match cli.verbose {
        0 => env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init(),
        1 => env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init(),
        _ => env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("debug")).init(),
    }
    
    match cli.command {
        Commands::Architecture { project_path, format } => {
            info!("Generating architecture documentation for {:?}", project_path);
            let config = ParserConfig::architecture_default();
            let parser = CodeParser::new(project_path, config)?;
            let result = parser.parse()?;
            
            match format.as_str() {
                "json" => println!("{}", serde_json::to_string_pretty(&result)?),
                "markdown" => println!("{}", result.to_markdown()),
                _ => anyhow::bail!("Unsupported format: {}", format),
            }
        }
        Commands::Parse { project_path, config, format } => {
            info!("Parsing project {:?} with custom config", project_path);
            let config = if let Some(config_path) = config {
                ParserConfig::from_file(config_path)?
            } else {
                ParserConfig::default()
            };
            
            let parser = CodeParser::new(project_path, config)?;
            let result = parser.parse()?;
            
            match format.as_str() {
                "json" => println!("{}", serde_json::to_string_pretty(&result)?),
                "markdown" => println!("{}", result.to_markdown()),
                _ => anyhow::bail!("Unsupported format: {}", format),
            }
        }
    }
    
    Ok(())
}