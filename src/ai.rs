use crate::config::{AIProvider, Config};
#[cfg(feature = "local")]
use crate::local_llm::LocalSpren;
use crate::shell::ShellType;
use anyhow::{anyhow, Result};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION, CONTENT_TYPE};
use serde::{Deserialize, Serialize};
#[cfg(feature = "local")]
use std::sync::Mutex;

#[cfg(feature = "local")]
use once_cell::sync::Lazy;

#[cfg(feature = "local")]
static LOCAL_LLM: Lazy<Mutex<Option<LocalSpren>>> = Lazy::new(|| Mutex::new(None));

// ============================================================================
// Anthropic Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicResponse {
    content: Option<Vec<AnthropicContent>>,
    error: Option<AnthropicError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicContent {
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct AnthropicError {
    message: String,
    #[serde(rename = "type")]
    error_type: Option<String>,
}

// ============================================================================
// OpenAI Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIResponse {
    choices: Option<Vec<OpenAIChoice>>,
    error: Option<OpenAIError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIError {
    message: String,
    #[serde(rename = "type")]
    error_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenAIMessage {
    content: String,
}

// ============================================================================
// Gemini Types
// ============================================================================

#[derive(Debug, Serialize, Deserialize)]
struct GeminiResponse {
    candidates: Option<Vec<GeminiCandidate>>,
    error: Option<GeminiError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiCandidate {
    content: GeminiContent,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiPart {
    text: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct GeminiError {
    message: String,
    status: Option<String>,
}

// ============================================================================
// Public API
// ============================================================================

pub async fn get_command_suggestion(query: &str, config: &Config) -> Result<(String, bool)> {
    match config.ai.provider {
        AIProvider::Anthropic => get_anthropic_command(query, config).await,
        AIProvider::OpenAI => get_openai_command(query, config).await,
        AIProvider::Gemini => get_gemini_command(query, config).await,
        #[cfg(feature = "local")]
        AIProvider::Local => get_local_command(query, config).await,
    }
}

pub async fn get_error_suggestion(
    command: &str,
    stdout: &str,
    stderr: &str,
    config: &Config,
) -> Result<String> {
    match config.ai.provider {
        AIProvider::Anthropic => get_anthropic_error(command, stdout, stderr, config).await,
        AIProvider::OpenAI => get_openai_error(command, stdout, stderr, config).await,
        AIProvider::Gemini => get_gemini_error(command, stdout, stderr, config).await,
        #[cfg(feature = "local")]
        AIProvider::Local => get_local_error(command, stdout, stderr, config).await,
    }
}

/// Get a fixed command based on the error output
/// Returns (fixed_command, is_dangerous)
#[cfg(feature = "local")]
pub async fn get_fix_command(
    original_command: &str,
    stdout: &str,
    stderr: &str,
    config: &Config,
) -> Result<(String, bool)> {
    get_local_fix(original_command, stdout, stderr, config).await
}

// ============================================================================
// Anthropic Implementation
// ============================================================================

async fn get_anthropic_command(query: &str, config: &Config) -> Result<(String, bool)> {
    let api_key = config
        .ai
        .anthropic_api_key
        .as_ref()
        .ok_or_else(|| anyhow!("Anthropic API key not configured. Set 'anthropic_api_key' in config."))?;

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
    headers.insert("x-api-key", HeaderValue::from_str(api_key)?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let shell_type = ShellType::detect();
    let shell_name = shell_type.get_shell_name();

    let prompt = build_command_prompt(shell_name, query);
    let model = get_model_or_default(config, "claude-3-5-haiku-20241022");

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .headers(headers)
        .json(&serde_json::json!({
            "model": model,
            "max_tokens": config.ai.max_tokens,
            "system": "You are Spren, a helpful command-line assistant. Respond only in the specified format.",
            "messages": [{
                "role": "user",
                "content": prompt
            }]
        }))
        .send()
        .await?
        .json::<AnthropicResponse>()
        .await?;

    if let Some(error) = response.error {
        return Err(anyhow!("Anthropic API error: {}", error.message));
    }

    let content = response
        .content
        .ok_or_else(|| anyhow!("Anthropic API returned no content"))?;

    if content.is_empty() {
        return Err(anyhow!("Anthropic API returned empty content"));
    }

    parse_ai_response(&content[0].text)
}

async fn get_anthropic_error(
    command: &str,
    stdout: &str,
    stderr: &str,
    config: &Config,
) -> Result<String> {
    let api_key = config
        .ai
        .anthropic_api_key
        .as_ref()
        .ok_or_else(|| anyhow!("Anthropic API key not configured. Set 'anthropic_api_key' in config."))?;

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert("anthropic-version", HeaderValue::from_static("2023-06-01"));
    headers.insert("x-api-key", HeaderValue::from_str(api_key)?);
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let shell_type = ShellType::detect();
    let shell_name = shell_type.get_shell_name();

    let prompt = build_error_prompt(shell_name, command, stdout, stderr);
    let model = get_model_or_default(config, "claude-3-5-haiku-20241022");

    let response = client
        .post("https://api.anthropic.com/v1/messages")
        .headers(headers)
        .json(&serde_json::json!({
            "model": model,
            "max_tokens": config.ai.max_tokens,
            "system": "You are Spren, a helpful command-line assistant. Provide clear and concise explanations.",
            "messages": [{
                "role": "user",
                "content": prompt
            }]
        }))
        .send()
        .await?
        .json::<AnthropicResponse>()
        .await?;

    if let Some(error) = response.error {
        return Err(anyhow!("Anthropic API error: {}", error.message));
    }

    let content = response
        .content
        .ok_or_else(|| anyhow!("Anthropic API returned no content"))?;

    if content.is_empty() {
        return Err(anyhow!("Anthropic API returned empty content"));
    }

    Ok(content[0].text.trim().to_string())
}

// ============================================================================
// OpenAI Implementation
// ============================================================================

async fn get_openai_command(query: &str, config: &Config) -> Result<(String, bool)> {
    let api_key = config
        .ai
        .openai_api_key
        .as_ref()
        .ok_or_else(|| anyhow!("OpenAI API key not configured. Set 'openai_api_key' in config."))?;

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let shell_type = ShellType::detect();
    let shell_name = shell_type.get_shell_name();

    let prompt = build_command_prompt(shell_name, query);
    let model = get_model_or_default(config, "gpt-4o-mini");

    // Use max_completion_tokens for newer models, fall back to max_tokens for compatibility
    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .headers(headers)
        .json(&serde_json::json!({
            "model": model,
            "max_completion_tokens": config.ai.max_tokens,
            "messages": [
                {
                    "role": "system",
                    "content": "You are Spren, a helpful command-line assistant. Respond only in the specified format."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        }))
        .send()
        .await?
        .json::<OpenAIResponse>()
        .await?;

    if let Some(error) = response.error {
        return Err(anyhow!("OpenAI API error: {}", error.message));
    }

    let choices = response
        .choices
        .ok_or_else(|| anyhow!("OpenAI API returned no choices"))?;

    if choices.is_empty() {
        return Err(anyhow!("OpenAI API returned empty choices"));
    }

    parse_ai_response(&choices[0].message.content)
}

async fn get_openai_error(
    command: &str,
    stdout: &str,
    stderr: &str,
    config: &Config,
) -> Result<String> {
    let api_key = config
        .ai
        .openai_api_key
        .as_ref()
        .ok_or_else(|| anyhow!("OpenAI API key not configured. Set 'openai_api_key' in config."))?;

    let client = reqwest::Client::new();
    let mut headers = HeaderMap::new();
    headers.insert(
        AUTHORIZATION,
        HeaderValue::from_str(&format!("Bearer {}", api_key))?,
    );
    headers.insert(CONTENT_TYPE, HeaderValue::from_static("application/json"));

    let shell_type = ShellType::detect();
    let shell_name = shell_type.get_shell_name();

    let prompt = build_error_prompt(shell_name, command, stdout, stderr);
    let model = get_model_or_default(config, "gpt-4o-mini");

    let response = client
        .post("https://api.openai.com/v1/chat/completions")
        .headers(headers)
        .json(&serde_json::json!({
            "model": model,
            "max_completion_tokens": config.ai.max_tokens,
            "messages": [
                {
                    "role": "system",
                    "content": "You are Spren, a helpful command-line assistant. Provide clear and concise explanations."
                },
                {
                    "role": "user",
                    "content": prompt
                }
            ]
        }))
        .send()
        .await?
        .json::<OpenAIResponse>()
        .await?;

    if let Some(error) = response.error {
        return Err(anyhow!("OpenAI API error: {}", error.message));
    }

    let choices = response
        .choices
        .ok_or_else(|| anyhow!("OpenAI API returned no choices"))?;

    if choices.is_empty() {
        return Err(anyhow!("OpenAI API returned empty choices"));
    }

    Ok(choices[0].message.content.trim().to_string())
}

// ============================================================================
// Gemini Implementation
// ============================================================================

async fn get_gemini_command(query: &str, config: &Config) -> Result<(String, bool)> {
    let api_key = config
        .ai
        .gemini_api_key
        .as_ref()
        .ok_or_else(|| anyhow!("Gemini API key not configured. Set 'gemini_api_key' in config."))?;

    let client = reqwest::Client::new();

    let shell_type = ShellType::detect();
    let shell_name = shell_type.get_shell_name();

    let prompt = format!(
        "You are Spren, a helpful command-line assistant. Respond only in the specified format.\n\n{}",
        build_command_prompt(shell_name, query)
    );
    let model = get_model_or_default(config, "gemini-2.0-flash");

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let response = client
        .post(&url)
        .header(CONTENT_TYPE, "application/json")
        .json(&serde_json::json!({
            "contents": [{
                "parts": [{
                    "text": prompt
                }]
            }],
            "generationConfig": {
                "temperature": config.ai.temperature,
                "maxOutputTokens": config.ai.max_tokens
            }
        }))
        .send()
        .await?
        .json::<GeminiResponse>()
        .await?;

    if let Some(error) = response.error {
        return Err(anyhow!("Gemini API error: {}", error.message));
    }

    let candidates = response
        .candidates
        .ok_or_else(|| anyhow!("Gemini API returned no candidates"))?;

    if candidates.is_empty() {
        return Err(anyhow!("Gemini API returned empty candidates"));
    }

    if candidates[0].content.parts.is_empty() {
        return Err(anyhow!("Gemini API returned empty parts"));
    }

    parse_ai_response(&candidates[0].content.parts[0].text)
}

async fn get_gemini_error(
    command: &str,
    stdout: &str,
    stderr: &str,
    config: &Config,
) -> Result<String> {
    let api_key = config
        .ai
        .gemini_api_key
        .as_ref()
        .ok_or_else(|| anyhow!("Gemini API key not configured. Set 'gemini_api_key' in config."))?;

    let client = reqwest::Client::new();

    let shell_type = ShellType::detect();
    let shell_name = shell_type.get_shell_name();

    let prompt = format!(
        "You are Spren, a helpful command-line assistant. Provide clear and concise explanations.\n\n{}",
        build_error_prompt(shell_name, command, stdout, stderr)
    );
    let model = get_model_or_default(config, "gemini-2.0-flash");

    let url = format!(
        "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
        model, api_key
    );

    let response = client
        .post(&url)
        .header(CONTENT_TYPE, "application/json")
        .json(&serde_json::json!({
            "contents": [{
                "parts": [{
                    "text": prompt
                }]
            }],
            "generationConfig": {
                "temperature": config.ai.temperature,
                "maxOutputTokens": config.ai.max_tokens
            }
        }))
        .send()
        .await?
        .json::<GeminiResponse>()
        .await?;

    if let Some(error) = response.error {
        return Err(anyhow!("Gemini API error: {}", error.message));
    }

    let candidates = response
        .candidates
        .ok_or_else(|| anyhow!("Gemini API returned no candidates"))?;

    if candidates.is_empty() {
        return Err(anyhow!("Gemini API returned empty candidates"));
    }

    if candidates[0].content.parts.is_empty() {
        return Err(anyhow!("Gemini API returned empty parts"));
    }

    Ok(candidates[0].content.parts[0].text.trim().to_string())
}

// ============================================================================
// Helper Functions
// ============================================================================

fn get_model_or_default<'a>(config: &'a Config, default: &'a str) -> &'a str {
    if config.ai.model.is_empty() {
        default
    } else {
        &config.ai.model
    }
}

fn build_command_prompt(shell_name: &str, query: &str) -> String {
    format!(
        r#"Convert to a {} command: {}

Reply ONLY in this exact format (2 lines, no explanation):
DANGEROUS:false
COMMAND:your_command_here

Set DANGEROUS:true only for destructive commands (rm -rf, format, dd, etc)."#,
        shell_name, query
    )
}

fn build_error_prompt(shell_name: &str, command: &str, stdout: &str, stderr: &str) -> String {
    format!(
        "Analyze briefly. {} command: {}\nOutput: {}\nError: {}\nOne short paragraph max.",
        shell_name, command, stdout, stderr
    )
}

fn parse_ai_response(response: &str) -> Result<(String, bool)> {
    let response = response.trim();

    // Try to find DANGEROUS line
    let is_dangerous = response.to_lowercase().contains("dangerous:true")
        || response.to_lowercase().contains("dangerous: true");

    // Try multiple patterns to extract the command
    let command = extract_command(response)?;

    Ok((command, is_dangerous))
}

fn extract_command(response: &str) -> Result<String> {
    let response = response.trim();

    // Handle empty response
    if response.is_empty() {
        return Err(anyhow!("Empty response from AI"));
    }

    // Pattern 1: COMMAND:xxx or COMMAND: xxx (case insensitive)
    for line in response.lines() {
        let lower = line.to_lowercase();
        if lower.starts_with("command:") {
            let cmd = line[8..].trim();
            if !cmd.is_empty() {
                return Ok(strip_backticks(cmd));
            }
        }
    }

    // Pattern 2: Look for command after "COMMAND" anywhere in line
    for line in response.lines() {
        if let Some(pos) = line.to_lowercase().find("command:") {
            let cmd = line[pos + 8..].trim();
            if !cmd.is_empty() {
                return Ok(strip_backticks(cmd));
            }
        }
    }

    // Pattern 3: Look for ```bash or ``` code blocks
    if let Some(start) = response.find("```") {
        let after_fence = &response[start + 3..];
        // Skip language identifier (bash, sh, etc.)
        let code_start = after_fence.find('\n').map(|i| i + 1).unwrap_or(0);
        if let Some(end) = after_fence[code_start..].find("```") {
            let cmd = after_fence[code_start..code_start + end].trim();
            if !cmd.is_empty() {
                return Ok(cmd.to_string());
            }
        }
    }

    // Pattern 4: Look for single backtick-wrapped command
    if let Some(start) = response.find('`') {
        if let Some(end) = response[start + 1..].find('`') {
            let cmd = &response[start + 1..start + 1 + end];
            if !cmd.is_empty() && !cmd.contains('\n') {
                return Ok(cmd.to_string());
            }
        }
    }

    // Pattern 5: If response is just 2 lines, second line is probably the command
    let lines: Vec<&str> = response.lines().collect();
    if lines.len() == 2 {
        let second = lines[1].trim();
        if !second.to_lowercase().starts_with("dangerous") {
            return Ok(strip_backticks(second));
        }
    }

    // Pattern 6: If it's a single line that looks like a command (starts with common commands)
    if lines.len() == 1 {
        let line = lines[0].trim();
        if looks_like_command(line) {
            return Ok(strip_backticks(line));
        }
    }

    // Pattern 7: Find any line that looks like a shell command
    for line in response.lines() {
        let trimmed = line.trim();
        if looks_like_command(trimmed) && !trimmed.to_lowercase().contains("dangerous") {
            return Ok(strip_backticks(trimmed));
        }
    }

    Err(anyhow!("Could not extract command from response:\n{}", response))
}

fn strip_backticks(s: &str) -> String {
    let s = s.trim();
    if s.starts_with('`') && s.ends_with('`') {
        s[1..s.len()-1].to_string()
    } else {
        s.to_string()
    }
}

fn looks_like_command(s: &str) -> bool {
    let common_prefixes = [
        "ls", "cd", "cat", "grep", "find", "du", "df", "free", "top", "ps",
        "kill", "mkdir", "rm", "cp", "mv", "chmod", "chown", "sudo", "apt",
        "yum", "dnf", "pacman", "brew", "npm", "yarn", "cargo", "git", "docker",
        "kubectl", "curl", "wget", "ssh", "scp", "tar", "zip", "unzip", "head",
        "tail", "sort", "uniq", "wc", "awk", "sed", "echo", "printf", "touch",
        "nano", "vim", "vi", "systemctl", "journalctl", "htop", "ncdu", "tree",
    ];

    let lower = s.to_lowercase();
    common_prefixes.iter().any(|&prefix| {
        lower.starts_with(prefix) &&
        (lower.len() == prefix.len() || lower.chars().nth(prefix.len()) == Some(' '))
    })
}

// ============================================================================
// Local LLM Implementation
// ============================================================================

#[cfg(feature = "local")]
fn init_local_llm(_config: &Config) -> Result<()> {
    let mut llm_guard = LOCAL_LLM.lock().map_err(|e| anyhow!("Lock error: {}", e))?;

    if llm_guard.is_none() {
        println!("Loading local AI model...");
        let spren = LocalSpren::load_default()?;
        *llm_guard = Some(spren);
        println!("Model loaded!");
    }

    Ok(())
}

#[cfg(feature = "local")]
async fn get_local_command(query: &str, config: &Config) -> Result<(String, bool)> {
    use crate::context::LocalContext;

    // Initialize LLM if not already done
    init_local_llm(config)?;

    // Gather local context (current directory, files, git status)
    let ctx = LocalContext::gather();
    let context_str = ctx.format_for_prompt();

    let max_tokens = config.ai.max_tokens.min(100);
    let temperature = config.ai.temperature;

    let mut llm_guard = LOCAL_LLM.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
    let llm = llm_guard.as_mut().ok_or_else(|| anyhow!("LLM not initialized"))?;

    let response = llm.generate_with_context(query, Some(&context_str), max_tokens, temperature)?;
    parse_ai_response(&response)
}

#[cfg(feature = "local")]
async fn get_local_error(
    command: &str,
    stdout: &str,
    stderr: &str,
    config: &Config,
) -> Result<String> {
    // Initialize LLM if not already done
    init_local_llm(config)?;

    let mut llm_guard = LOCAL_LLM.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
    let llm = llm_guard.as_mut().ok_or_else(|| anyhow!("LLM not initialized"))?;

    llm.analyze_error(command, stdout, stderr)
}

#[cfg(feature = "local")]
async fn get_local_fix(
    command: &str,
    stdout: &str,
    stderr: &str,
    config: &Config,
) -> Result<(String, bool)> {
    use crate::context::LocalContext;

    init_local_llm(config)?;

    // Gather context for better fix suggestions
    let ctx = LocalContext::gather();
    let context_str = ctx.format_for_prompt();

    let fix_prompt = format!(
        "Command '{}' failed.\nOutput: {}\nError: {}\nProvide a fixed command.",
        command, stdout, stderr
    );

    let max_tokens = config.ai.max_tokens.min(100);
    let temperature = config.ai.temperature;

    let mut llm_guard = LOCAL_LLM.lock().map_err(|e| anyhow!("Lock error: {}", e))?;
    let llm = llm_guard.as_mut().ok_or_else(|| anyhow!("LLM not initialized"))?;

    let response = llm.generate_with_context(&fix_prompt, Some(&context_str), max_tokens, temperature)?;
    parse_ai_response(&response)
}
