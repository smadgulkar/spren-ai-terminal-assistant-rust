//! Interactive TUI for Spren
//!
//! Provides a rich terminal interface with:
//! - Command editing before execution
//! - History navigation
//! - Visual feedback during AI processing

#[cfg(feature = "tui")]
use anyhow::Result;
#[cfg(feature = "tui")]
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
#[cfg(feature = "tui")]
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame, Terminal,
};
#[cfg(feature = "tui")]
use std::io::{self, Stdout};

/// Application state for the TUI
#[cfg(feature = "tui")]
pub struct App {
    /// Current input query
    pub input: String,
    /// Cursor position in input
    pub cursor: usize,
    /// Suggested command from AI
    pub command: Option<String>,
    /// Whether command is dangerous
    pub is_dangerous: bool,
    /// Current status message
    pub status: String,
    /// Command history
    pub history: Vec<String>,
    /// History navigation index
    pub history_idx: Option<usize>,
    /// Output from last command
    pub output: String,
    /// Whether we're in edit mode (editing the suggested command)
    pub edit_mode: bool,
    /// The command being edited
    pub edited_command: String,
    /// Cursor position in edited command
    pub edit_cursor: usize,
    /// Whether app should quit
    pub should_quit: bool,
    /// Whether we're waiting for AI
    pub loading: bool,
}

#[cfg(feature = "tui")]
impl Default for App {
    fn default() -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            command: None,
            is_dangerous: false,
            status: "Type your request and press Enter".to_string(),
            history: Vec::new(),
            history_idx: None,
            output: String::new(),
            edit_mode: false,
            edited_command: String::new(),
            edit_cursor: 0,
            should_quit: false,
            loading: false,
        }
    }
}

#[cfg(feature = "tui")]
impl App {
    pub fn new() -> Self {
        Self::default()
    }

    /// Handle a key event
    pub fn handle_key(&mut self, key: KeyCode, modifiers: KeyModifiers) {
        match key {
            KeyCode::Char('c') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Char('q') if modifiers.contains(KeyModifiers::CONTROL) => {
                self.should_quit = true;
            }
            KeyCode::Esc => {
                if self.edit_mode {
                    self.edit_mode = false;
                    self.status = "Edit cancelled".to_string();
                } else {
                    self.should_quit = true;
                }
            }
            _ if self.edit_mode => self.handle_edit_key(key),
            _ => self.handle_input_key(key),
        }
    }

    fn handle_input_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char(c) => {
                self.input.insert(self.cursor, c);
                self.cursor += 1;
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    self.cursor -= 1;
                    self.input.remove(self.cursor);
                }
            }
            KeyCode::Delete => {
                if self.cursor < self.input.len() {
                    self.input.remove(self.cursor);
                }
            }
            KeyCode::Left => {
                self.cursor = self.cursor.saturating_sub(1);
            }
            KeyCode::Right => {
                self.cursor = (self.cursor + 1).min(self.input.len());
            }
            KeyCode::Home => {
                self.cursor = 0;
            }
            KeyCode::End => {
                self.cursor = self.input.len();
            }
            KeyCode::Up => {
                // Navigate history
                if !self.history.is_empty() {
                    let idx = match self.history_idx {
                        None => self.history.len() - 1,
                        Some(i) => i.saturating_sub(1),
                    };
                    self.history_idx = Some(idx);
                    self.input = self.history[idx].clone();
                    self.cursor = self.input.len();
                }
            }
            KeyCode::Down => {
                // Navigate history forward
                if let Some(idx) = self.history_idx {
                    if idx + 1 < self.history.len() {
                        self.history_idx = Some(idx + 1);
                        self.input = self.history[idx + 1].clone();
                    } else {
                        self.history_idx = None;
                        self.input.clear();
                    }
                    self.cursor = self.input.len();
                }
            }
            KeyCode::Tab => {
                // Enter edit mode if we have a command
                if self.command.is_some() {
                    self.edit_mode = true;
                    self.edited_command = self.command.clone().unwrap_or_default();
                    self.edit_cursor = self.edited_command.len();
                    self.status = "Editing command (Tab to confirm, Esc to cancel)".to_string();
                }
            }
            _ => {}
        }
    }

    fn handle_edit_key(&mut self, key: KeyCode) {
        match key {
            KeyCode::Char(c) => {
                self.edited_command.insert(self.edit_cursor, c);
                self.edit_cursor += 1;
            }
            KeyCode::Backspace => {
                if self.edit_cursor > 0 {
                    self.edit_cursor -= 1;
                    self.edited_command.remove(self.edit_cursor);
                }
            }
            KeyCode::Delete => {
                if self.edit_cursor < self.edited_command.len() {
                    self.edited_command.remove(self.edit_cursor);
                }
            }
            KeyCode::Left => {
                self.edit_cursor = self.edit_cursor.saturating_sub(1);
            }
            KeyCode::Right => {
                self.edit_cursor = (self.edit_cursor + 1).min(self.edited_command.len());
            }
            KeyCode::Home => {
                self.edit_cursor = 0;
            }
            KeyCode::End => {
                self.edit_cursor = self.edited_command.len();
            }
            KeyCode::Tab | KeyCode::Enter => {
                // Confirm edit
                self.command = Some(self.edited_command.clone());
                self.edit_mode = false;
                self.status =
                    "Command updated. Press Enter to execute, 'y' to confirm.".to_string();
            }
            _ => {}
        }
    }

    /// Set the suggested command
    pub fn set_command(&mut self, cmd: String, dangerous: bool) {
        self.command = Some(cmd.clone());
        self.is_dangerous = dangerous;
        self.edited_command = cmd;
        self.edit_cursor = self.edited_command.len();
        if dangerous {
            self.status =
                "DANGEROUS command! Press 'y' to execute, Tab to edit, Esc to cancel".to_string();
        } else {
            self.status = "Press 'y' to execute, Tab to edit, Esc to cancel".to_string();
        }
    }

    /// Set command output
    pub fn set_output(&mut self, output: String) {
        self.output = output;
    }

    /// Clear for new query
    pub fn clear_for_new_query(&mut self) {
        if !self.input.is_empty() {
            self.history.push(self.input.clone());
        }
        self.input.clear();
        self.cursor = 0;
        self.command = None;
        self.is_dangerous = false;
        self.history_idx = None;
        self.edit_mode = false;
        self.status = "Type your request and press Enter".to_string();
    }

    /// Get current command (edited or original)
    pub fn get_command(&self) -> Option<&str> {
        self.command.as_deref()
    }
}

/// Initialize the terminal for TUI mode
#[cfg(feature = "tui")]
pub fn init_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore the terminal to normal mode
#[cfg(feature = "tui")]
pub fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

/// Draw the UI
#[cfg(feature = "tui")]
pub fn draw(frame: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(3), // Title
            Constraint::Length(3), // Input
            Constraint::Length(5), // Command
            Constraint::Min(5),    // Output
            Constraint::Length(3), // Status
        ])
        .split(frame.area());

    // Title
    let title = Paragraph::new("Spren - AI Shell Assistant")
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(Block::default().borders(Borders::ALL));
    frame.render_widget(title, chunks[0]);

    // Input field
    let input_style = if app.edit_mode {
        Style::default().fg(Color::DarkGray)
    } else {
        Style::default().fg(Color::White)
    };
    let input = Paragraph::new(app.input.as_str())
        .style(input_style)
        .block(Block::default().borders(Borders::ALL).title("Query"));
    frame.render_widget(input, chunks[1]);

    // Show cursor in input field if not in edit mode
    if !app.edit_mode && !app.loading {
        frame.set_cursor_position((chunks[1].x + app.cursor as u16 + 1, chunks[1].y + 1));
    }

    // Command display/edit
    let cmd_block = Block::default()
        .borders(Borders::ALL)
        .title(if app.edit_mode {
            "Command (editing)"
        } else {
            "Suggested Command"
        });

    if let Some(ref cmd) = app.command {
        let display_cmd = if app.edit_mode {
            &app.edited_command
        } else {
            cmd
        };

        let cmd_style = if app.is_dangerous {
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)
        } else if app.edit_mode {
            Style::default().fg(Color::Yellow)
        } else {
            Style::default().fg(Color::Green)
        };

        let spans = if app.is_dangerous && !app.edit_mode {
            vec![
                Span::styled(display_cmd, cmd_style),
                Span::styled(
                    " [DANGEROUS]",
                    Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                ),
            ]
        } else {
            vec![Span::styled(display_cmd, cmd_style)]
        };

        let command = Paragraph::new(Line::from(spans))
            .block(cmd_block)
            .wrap(Wrap { trim: false });
        frame.render_widget(command, chunks[2]);

        // Show cursor in edit mode
        if app.edit_mode {
            frame.set_cursor_position((chunks[2].x + app.edit_cursor as u16 + 1, chunks[2].y + 1));
        }
    } else if app.loading {
        let loading = Paragraph::new("Thinking...")
            .style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::ITALIC),
            )
            .block(cmd_block);
        frame.render_widget(loading, chunks[2]);
    } else {
        let empty = Paragraph::new("").block(cmd_block);
        frame.render_widget(empty, chunks[2]);
    }

    // Output area
    let output = Paragraph::new(app.output.as_str())
        .style(Style::default().fg(Color::White))
        .block(Block::default().borders(Borders::ALL).title("Output"))
        .wrap(Wrap { trim: false });
    frame.render_widget(output, chunks[3]);

    // Status bar
    let status_style = if app.is_dangerous {
        Style::default().fg(Color::Red)
    } else {
        Style::default().fg(Color::Cyan)
    };
    let status = Paragraph::new(app.status.as_str())
        .style(status_style)
        .block(Block::default().borders(Borders::ALL).title("Status"));
    frame.render_widget(status, chunks[4]);
}

/// Poll for events with timeout
#[cfg(feature = "tui")]
pub fn poll_event(timeout_ms: u64) -> Result<Option<Event>> {
    if event::poll(std::time::Duration::from_millis(timeout_ms))? {
        Ok(Some(event::read()?))
    } else {
        Ok(None)
    }
}
