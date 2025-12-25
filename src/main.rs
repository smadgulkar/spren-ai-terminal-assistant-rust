use anyhow::Result;
use colored::*;
use std::io::{self, Write};
use std::time::Instant;

mod ai;
mod config;
mod executor;
#[cfg(feature = "local")]
mod local_llm;
mod shell;

#[tokio::main]
async fn main() -> Result<()> {
    // Try to load config, or use defaults (zero-config mode)
    let config = load_or_default_config();
    let shell_type = shell::ShellType::detect();

    println!("{}", "Spren - Your AI Shell Assistant".green().bold());
    println!("Shell Type: {}", format!("{:?}", shell_type).blue());

    #[cfg(feature = "local")]
    println!("Mode: {}", "Local AI".cyan());
    #[cfg(not(feature = "local"))]
    println!("Mode: {}", "Cloud AI".cyan());

    println!("Type 'exit' to quit\n");

    loop {
        print!("spren> ");
        io::stdout().flush()?;

        let mut query = String::new();
        io::stdin().read_line(&mut query)?;
        let query = query.trim();

        if query.is_empty() {
            continue;
        }

        if query == "exit" || query == "quit" {
            break;
        }

        match process_query(query, &config).await {
            Ok(_) => continue,
            Err(e) => eprintln!("{}: {}", "Error".red().bold(), e),
        }
    }

    Ok(())
}

/// Load config from file, or return sensible defaults for zero-config operation
fn load_or_default_config() -> config::Config {
    // Try to load existing config
    if let Ok(config_path) = config::get_config_path() {
        if config_path.exists() {
            if let Ok(config) = config::Config::load(&config_path) {
                return config;
            }
        }
    }

    // Return default config (local mode if compiled with local feature)
    config::Config::default()
}

async fn process_query(query: &str, config: &config::Config) -> Result<()> {
    let start = Instant::now();

    // Get command suggestion from AI
    let (command, is_dangerous) = ai::get_command_suggestion(query, config).await?;

    let inference_time = start.elapsed();

    println!("\n{} {}", "Suggested command:".blue().bold(), format!("({:.0?})", inference_time).dimmed());
    if is_dangerous {
        println!("{} {}", command, "[DANGEROUS]".red().bold());
        println!("\n{}", "This command has been identified as potentially dangerous.".yellow());
    } else {
        println!("{}", command);
    }

    // Always ask for confirmation
    print!("\nExecute? [y/N] ");
    io::stdout().flush()?;

    let mut response = String::new();
    io::stdin().read_line(&mut response)?;

    if response.trim().to_lowercase() != "y" {
        return Ok(());
    }

    let exec_start = Instant::now();
    match executor::execute_command(&command).await {
        Ok(output) => {
            println!("{}", format!("Execution time: {:?}", exec_start.elapsed()).dimmed());

            if !output.stdout.is_empty() {
                println!("\n{}", output.stdout);
            }

            if !output.stderr.is_empty() {
                if output.success {
                    println!("{}: {}", "Note".yellow().bold(), output.stderr);
                } else {
                    println!("{}: {}", "Error".red().bold(), output.stderr);

                    // Get error analysis
                    if let Ok(suggestion) = ai::get_error_suggestion(
                        &command,
                        &output.stdout,
                        &output.stderr,
                        config
                    ).await {
                        println!("\n{}", "Suggestion:".yellow().bold());
                        println!("{}", suggestion);
                    }
                }
            }
        }
        Err(e) => {
            println!("\n{}: {}", "System Error".red().bold(), e);
        }
    }

    Ok(())
}
