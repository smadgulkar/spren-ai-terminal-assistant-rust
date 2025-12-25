//! Context gathering for intelligent command generation
//!
//! This module provides local context (current directory, files, git status)
//! to help the LLM generate more accurate commands.

use std::path::PathBuf;
use std::process::Command;

/// Gathered context about the current environment
#[derive(Debug, Default)]
pub struct LocalContext {
    pub cwd: PathBuf,
    pub files: Vec<String>,
    pub git_branch: Option<String>,
    pub is_git_repo: bool,
}

impl LocalContext {
    /// Gather context about the current directory
    pub fn gather() -> Self {
        let cwd = std::env::current_dir().unwrap_or_default();
        let files = list_directory_fast(&cwd);
        let (is_git_repo, git_branch) = get_git_info(&cwd);

        Self {
            cwd,
            files,
            git_branch,
            is_git_repo,
        }
    }

    /// Format context for injection into the prompt
    pub fn format_for_prompt(&self) -> String {
        let mut parts = Vec::new();

        // Current directory
        parts.push(format!("CWD: {}", self.cwd.display()));

        // File listing (limit to first 20 items to keep prompt small)
        if !self.files.is_empty() {
            let files_preview: Vec<&str> = self.files.iter().take(20).map(|s| s.as_str()).collect();
            let suffix = if self.files.len() > 20 {
                format!(" (+{} more)", self.files.len() - 20)
            } else {
                String::new()
            };
            parts.push(format!("Files: [{}]{}", files_preview.join(", "), suffix));
        }

        // Git info
        if self.is_git_repo {
            if let Some(ref branch) = self.git_branch {
                parts.push(format!("Git: branch '{}'", branch));
            } else {
                parts.push("Git: yes".to_string());
            }
        }

        parts.join("\n")
    }
}

/// Fast directory listing using ls -F style output
fn list_directory_fast(path: &PathBuf) -> Vec<String> {
    let mut entries = Vec::new();

    if let Ok(read_dir) = std::fs::read_dir(path) {
        for entry in read_dir.filter_map(|e| e.ok()).take(50) {
            let name = entry.file_name().to_string_lossy().to_string();

            // Add type indicator like ls -F
            let indicator = if let Ok(ft) = entry.file_type() {
                if ft.is_dir() {
                    "/"
                } else if ft.is_symlink() {
                    "@"
                } else {
                    ""
                }
            } else {
                ""
            };

            entries.push(format!("{}{}", name, indicator));
        }
    }

    // Sort: directories first, then files
    entries.sort_by(|a, b| {
        let a_is_dir = a.ends_with('/');
        let b_is_dir = b.ends_with('/');
        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.to_lowercase().cmp(&b.to_lowercase()),
        }
    });

    entries
}

/// Get git repository info (fast)
fn get_git_info(path: &PathBuf) -> (bool, Option<String>) {
    // Check if .git exists (faster than running git command)
    let git_dir = path.join(".git");
    if !git_dir.exists() {
        // Check parent directories
        let mut current = path.clone();
        loop {
            if current.join(".git").exists() {
                break;
            }
            if !current.pop() {
                return (false, None);
            }
        }
    }

    // Get current branch name
    let branch = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(path)
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                Some(String::from_utf8_lossy(&output.stdout).trim().to_string())
            } else {
                None
            }
        });

    (true, branch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gather_context() {
        let ctx = LocalContext::gather();
        assert!(!ctx.cwd.as_os_str().is_empty());
        println!("Context:\n{}", ctx.format_for_prompt());
    }
}
