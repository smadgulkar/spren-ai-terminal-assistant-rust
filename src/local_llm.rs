//! Local LLM inference using Candle with quantized GGUF models
//!
//! This module provides CPU-based inference for the fine-tuned Qwen 0.5B model,
//! allowing Spren to work without cloud API calls.

#[cfg(feature = "local")]
use anyhow::{anyhow, Result};
#[cfg(feature = "local")]
use candle_core::quantized::gguf_file;
#[cfg(feature = "local")]
use candle_core::{Device, Tensor};
#[cfg(feature = "local")]
use candle_transformers::generation::LogitsProcessor;
#[cfg(feature = "local")]
use candle_transformers::models::quantized_qwen2::ModelWeights as Qwen2;
#[cfg(feature = "local")]
use std::fs::File;
#[cfg(feature = "local")]
use std::path::{Path, PathBuf};
#[cfg(feature = "local")]
use tokenizers::Tokenizer;

/// Model and tokenizer filenames
#[cfg(feature = "local")]
const MODEL_FILENAME: &str = "spren-model.gguf";
#[cfg(feature = "local")]
const TOKENIZER_FILENAME: &str = "tokenizer.json";

/// Local Spren model for shell command generation
#[cfg(feature = "local")]
pub struct LocalSpren {
    model: Qwen2,
    tokenizer: Tokenizer,
    device: Device,
}

#[cfg(feature = "local")]
impl LocalSpren {
    /// Load model from default locations (searches relative to executable, then standard paths)
    pub fn load_default() -> Result<Self> {
        let (model_path, tokenizer_path) = find_model_files()?;
        Self::new(
            &model_path.to_string_lossy(),
            &tokenizer_path.to_string_lossy(),
        )
    }

    /// Load the GGUF model and tokenizer from specific paths
    pub fn new(model_path: &str, tokenizer_path: &str) -> Result<Self> {
        let device = Device::Cpu;

        // Verify files exist
        if !Path::new(model_path).exists() {
            return Err(anyhow!(
                "Model file not found: {}\n\nThe model should be at one of:\n  - Next to the spren executable\n  - ~/.local/share/spren/\n  - /usr/share/spren/",
                model_path
            ));
        }
        if !Path::new(tokenizer_path).exists() {
            return Err(anyhow!(
                "Tokenizer file not found: {}\n\nDownload from: https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct",
                tokenizer_path
            ));
        }

        // Load the GGUF file
        let mut file = File::open(model_path)?;
        let content = gguf_file::Content::read(&mut file)
            .map_err(|e| anyhow!("Failed to read GGUF: {}", e))?;
        let model = Qwen2::from_gguf(content, &mut file, &device)
            .map_err(|e| anyhow!("Failed to load model: {}", e))?;

        // Load the Tokenizer
        let tokenizer = Tokenizer::from_file(tokenizer_path)
            .map_err(|e| anyhow!("Failed to load tokenizer: {}", e))?;

        Ok(Self {
            model,
            tokenizer,
            device,
        })
    }

    /// Generate a shell command from natural language input
    pub fn generate(&mut self, prompt: &str, max_tokens: u32, temperature: f32) -> Result<String> {
        // Format prompt using ChatML format for Qwen Instruct models
        let formatted_prompt = format!(
            "<|im_start|>system\nYou are Spren, a terminal assistant. Convert natural language to shell commands. Reply with DANGEROUS:true/false and COMMAND:the_command<|im_end|>\n<|im_start|>user\n{}<|im_end|>\n<|im_start|>assistant\n",
            prompt
        );

        // Encode tokens
        let encoding = self
            .tokenizer
            .encode(formatted_prompt.as_str(), true)
            .map_err(|e| anyhow!("Tokenization failed: {}", e))?;
        let prompt_tokens = encoding.get_ids().to_vec();
        let mut all_tokens = prompt_tokens.clone();
        let mut output_tokens = vec![];

        // Generation config
        let temp = if temperature <= 0.0 {
            None
        } else {
            Some(temperature as f64)
        };
        let mut logits_processor = LogitsProcessor::new(299792458, temp, None);

        // Qwen2.5 special tokens
        const EOS_TOKEN: u32 = 151643; // <|endoftext|>
        const EOT_TOKEN: u32 = 151645; // <|im_end|>

        // Inference loop
        for i in 0..max_tokens {
            let context_size = if i == 0 { all_tokens.len() } else { 1 };
            let start_pos = all_tokens.len().saturating_sub(context_size);
            let context = &all_tokens[start_pos..];

            let input = Tensor::new(context, &self.device)?.unsqueeze(0)?;
            let logits = self.model.forward(&input, start_pos)?;
            let logits = logits.squeeze(0)?.squeeze(0)?;

            let next_token = logits_processor.sample(&logits)?;

            // Stop on End-of-Turn or End-of-Text tokens
            if next_token == EOS_TOKEN || next_token == EOT_TOKEN {
                break;
            }

            all_tokens.push(next_token);
            output_tokens.push(next_token);
        }

        // Decode output tokens
        let result = self
            .tokenizer
            .decode(&output_tokens, true)
            .map_err(|e| anyhow!("Decoding failed: {}", e))?;

        // Clean up the result
        let clean_result = result
            .trim()
            .replace("<|im_end|>", "")
            .replace("<|endoftext|>", "")
            .trim()
            .to_string();

        Ok(clean_result)
    }

    /// Generate a command suggestion (convenience wrapper)
    pub fn get_command(&mut self, query: &str) -> Result<(String, bool)> {
        let response = self.generate(query, 100, 0.1)?;
        parse_response(&response)
    }

    /// Analyze an error (convenience wrapper)
    pub fn analyze_error(&mut self, command: &str, stdout: &str, stderr: &str) -> Result<String> {
        let prompt = format!(
            "Command '{}' produced:\nOutput: {}\nError: {}\nExplain briefly.",
            command, stdout, stderr
        );
        self.generate(&prompt, 150, 0.3)
    }
}

/// Find model files in standard locations
#[cfg(feature = "local")]
fn find_model_files() -> Result<(PathBuf, PathBuf)> {
    let search_paths = get_search_paths();

    for base_path in &search_paths {
        let model_path = base_path.join(MODEL_FILENAME);
        let tokenizer_path = base_path.join(TOKENIZER_FILENAME);

        if model_path.exists() && tokenizer_path.exists() {
            return Ok((model_path, tokenizer_path));
        }
    }

    // Return error with helpful message
    let paths_tried: Vec<String> = search_paths
        .iter()
        .map(|p| format!("  - {}", p.display()))
        .collect();

    Err(anyhow!(
        "Could not find model files ({} and {})\n\nSearched in:\n{}\n\nPlease place the model files in one of these locations.",
        MODEL_FILENAME,
        TOKENIZER_FILENAME,
        paths_tried.join("\n")
    ))
}

/// Get list of paths to search for model files
#[cfg(feature = "local")]
fn get_search_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // 1. Next to the executable
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(exe_dir) = exe_path.parent() {
            paths.push(exe_dir.to_path_buf());
            paths.push(exe_dir.join("models"));
        }
    }

    // 2. Current working directory
    if let Ok(cwd) = std::env::current_dir() {
        paths.push(cwd.clone());
        paths.push(cwd.join("models"));
    }

    // 3. User data directory (~/.local/share/spren on Linux, AppData on Windows)
    if let Some(data_dir) = dirs::data_local_dir() {
        paths.push(data_dir.join("spren"));
    }

    // 4. Home directory
    if let Some(home) = dirs::home_dir() {
        paths.push(home.join(".spren"));
    }

    // 5. System-wide (Linux/macOS)
    #[cfg(unix)]
    {
        paths.push(PathBuf::from("/usr/share/spren"));
        paths.push(PathBuf::from("/usr/local/share/spren"));
    }

    paths
}

/// Parse the model response to extract command and danger flag
#[cfg(feature = "local")]
fn parse_response(response: &str) -> Result<(String, bool)> {
    let response = response.trim();

    // Check for dangerous flag
    let is_dangerous = response.to_lowercase().contains("dangerous:true")
        || response.to_lowercase().contains("dangerous: true");

    // Extract command
    let command = extract_command(response)?;

    Ok((command, is_dangerous))
}

/// Extract the command from the response
#[cfg(feature = "local")]
fn extract_command(response: &str) -> Result<String> {
    // Try COMMAND: pattern first
    for line in response.lines() {
        let lower = line.to_lowercase();
        if lower.starts_with("command:") {
            let cmd = line[8..].trim();
            if !cmd.is_empty() {
                return Ok(cmd.to_string());
            }
        }
    }

    // Try to find command anywhere in line
    for line in response.lines() {
        if let Some(pos) = line.to_lowercase().find("command:") {
            let cmd = line[pos + 8..].trim();
            if !cmd.is_empty() {
                return Ok(cmd.to_string());
            }
        }
    }

    // If only one line and looks like a command, use it
    let lines: Vec<&str> = response.lines().collect();
    if lines.len() == 1 {
        return Ok(lines[0].trim().to_string());
    }

    // Last resort: find a line that looks like a shell command
    for line in response.lines() {
        let trimmed = line.trim();
        if !trimmed.to_lowercase().starts_with("dangerous") && !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    Err(anyhow!("Could not extract command from: {}", response))
}

// ============================================================================
// Stub implementation when local feature is disabled
// ============================================================================

#[cfg(not(feature = "local"))]
pub struct LocalSpren;

#[cfg(not(feature = "local"))]
impl LocalSpren {
    pub fn load_default() -> anyhow::Result<Self> {
        anyhow::bail!("Local LLM support not compiled. Rebuild with: cargo build --features local")
    }

    pub fn new(_model_path: &str, _tokenizer_path: &str) -> anyhow::Result<Self> {
        anyhow::bail!("Local LLM support not compiled. Rebuild with: cargo build --features local")
    }

    pub fn generate(
        &mut self,
        _prompt: &str,
        _max_tokens: u32,
        _temperature: f32,
    ) -> anyhow::Result<String> {
        anyhow::bail!("Local LLM support not compiled")
    }

    pub fn get_command(&mut self, _query: &str) -> anyhow::Result<(String, bool)> {
        anyhow::bail!("Local LLM support not compiled")
    }

    pub fn analyze_error(
        &mut self,
        _command: &str,
        _stdout: &str,
        _stderr: &str,
    ) -> anyhow::Result<String> {
        anyhow::bail!("Local LLM support not compiled")
    }
}
