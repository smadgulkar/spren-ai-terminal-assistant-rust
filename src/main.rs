use anyhow::Result;
use clap::Parser;
use colored::*;
use std::io::{self, Write};
use std::time::Instant;

mod ai;
mod config;
#[cfg(feature = "local")]
mod context;
mod executor;
#[cfg(feature = "local")]
mod local_llm;
mod shell;
#[cfg(feature = "tui")]
mod tui;

#[derive(Parser)]
#[command(name = "spren", about = "AI-powered shell assistant")]
struct Args {
    /// Enable interactive TUI mode
    #[cfg(feature = "tui")]
    #[arg(long)]
    tui: bool,

    /// Single query mode (non-interactive)
    #[arg(short, long)]
    query: Option<String>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    let config = load_or_default_config();

    // Single query mode
    if let Some(query) = args.query {
        return process_query(&query, &config).await;
    }

    // TUI mode
    #[cfg(feature = "tui")]
    if args.tui {
        return run_tui(config).await;
    }

    // Default: simple REPL mode
    run_repl(config).await
}

/// Run the simple REPL interface
async fn run_repl(config: config::Config) -> Result<()> {
    let shell_type = shell::ShellType::detect();

    println!("{}", "Spren - Your AI Shell Assistant".green().bold());
    println!("Shell Type: {}", format!("{:?}", shell_type).blue());

    #[cfg(feature = "local")]
    println!("Mode: {}", "Local AI".cyan());
    #[cfg(not(feature = "local"))]
    println!("Mode: {}", "Cloud AI".cyan());

    #[cfg(feature = "tui")]
    println!("Tip: Run with {} for interactive mode", "--tui".cyan());

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

/// Run the interactive TUI
#[cfg(feature = "tui")]
async fn run_tui(config: config::Config) -> Result<()> {
    use crossterm::event::{Event, KeyCode, KeyEventKind};

    let mut terminal = tui::init_terminal()?;
    let mut app = tui::App::new();

    loop {
        // Draw UI
        terminal.draw(|f| tui::draw(f, &app))?;

        // Handle events
        if let Some(event) = tui::poll_event(100)? {
            if let Event::Key(key) = event {
                if key.kind != KeyEventKind::Press {
                    continue;
                }

                match key.code {
                    KeyCode::Enter if !app.edit_mode => {
                        if app.command.is_some() {
                            // We have a command, this is confirmation
                            // Do nothing here, 'y' handles execution
                        } else if !app.input.is_empty() {
                            // Get command from AI
                            app.loading = true;
                            app.status = "Thinking...".to_string();
                            terminal.draw(|f| tui::draw(f, &app))?;

                            match ai::get_command_suggestion(&app.input, &config).await {
                                Ok((cmd, dangerous)) => {
                                    app.set_command(cmd, dangerous);
                                }
                                Err(e) => {
                                    app.status = format!("Error: {}", e);
                                }
                            }
                            app.loading = false;
                        }
                    }
                    KeyCode::Char('y') | KeyCode::Char('Y') if app.command.is_some() && !app.edit_mode => {
                        // Execute command - clone to avoid borrow issues
                        let cmd = app.get_command().map(|s| s.to_string());
                        if let Some(cmd) = cmd {
                            app.status = "Executing...".to_string();
                            terminal.draw(|f| tui::draw(f, &app))?;

                            match executor::execute_command(&cmd).await {
                                Ok(output) => {
                                    let mut result = String::new();
                                    if !output.stdout.is_empty() {
                                        result.push_str(&output.stdout);
                                    }
                                    if !output.stderr.is_empty() {
                                        if !result.is_empty() {
                                            result.push_str("\n");
                                        }
                                        if output.success {
                                            result.push_str(&format!("Note: {}", output.stderr));
                                        } else {
                                            result.push_str(&format!("Error: {}", output.stderr));
                                        }
                                    }
                                    if result.is_empty() {
                                        result = "Command completed successfully".to_string();
                                    }
                                    app.set_output(result);
                                    app.status = "Done. Enter new query or Ctrl+C to quit".to_string();
                                }
                                Err(e) => {
                                    app.set_output(format!("Execution error: {}", e));
                                    app.status = "Command failed".to_string();
                                }
                            }
                            app.clear_for_new_query();
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') if app.command.is_some() && !app.edit_mode => {
                        // Cancel command
                        app.clear_for_new_query();
                        app.status = "Cancelled. Enter new query.".to_string();
                    }
                    _ => {
                        app.handle_key(key.code, key.modifiers);
                    }
                }

                if app.should_quit {
                    break;
                }
            }
        }
    }

    tui::restore_terminal(&mut terminal)?;
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

    // Auto-fix loop: retry failed commands up to 3 times
    let mut current_command = command;
    let mut attempts = 0;
    const MAX_RETRIES: u32 = 3;

    loop {
        let exec_start = Instant::now();
        match executor::execute_command(&current_command).await {
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

                        attempts += 1;
                        if attempts >= MAX_RETRIES {
                            println!("\n{}", "Max retries reached.".red());
                            break;
                        }

                        // Try to get a fixed command
                        #[cfg(feature = "local")]
                        {
                            println!("\n{}", "Attempting to fix...".yellow());
                            match ai::get_fix_command(
                                &current_command,
                                &output.stdout,
                                &output.stderr,
                                config
                            ).await {
                                Ok((fixed_cmd, is_dangerous)) => {
                                    println!("{} {}", "Fixed command:".blue().bold(), &fixed_cmd);
                                    if is_dangerous {
                                        println!("{}", "[DANGEROUS]".red().bold());
                                    }

                                    print!("Try fixed command? [y/N] ");
                                    io::stdout().flush()?;

                                    let mut resp = String::new();
                                    io::stdin().read_line(&mut resp)?;

                                    if resp.trim().to_lowercase() == "y" {
                                        current_command = fixed_cmd;
                                        continue;
                                    }
                                }
                                Err(e) => {
                                    println!("{}: {}", "Could not generate fix".red(), e);
                                }
                            }
                        }
                    }
                }
                break;
            }
            Err(e) => {
                println!("\n{}: {}", "System Error".red().bold(), e);
                break;
            }
        }
    }

    Ok(())
}
