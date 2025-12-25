use anyhow::Result;
use dirs::home_dir;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub ai: AIConfig,
    #[serde(default)]
    pub security: SecurityConfig,
    #[serde(default)]
    pub display: DisplayConfig,
    #[serde(default)]
    pub shell: ShellConfig,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AIConfig {
    #[serde(default)]
    pub provider: AIProvider,
    #[serde(default)]
    pub anthropic_api_key: Option<String>,
    #[serde(default)]
    pub openai_api_key: Option<String>,
    #[serde(default)]
    pub gemini_api_key: Option<String>,
    #[serde(default = "default_model")]
    pub model: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default = "default_temperature")]
    pub temperature: f32,
    // Local LLM settings
    #[serde(default)]
    pub local_model_path: Option<String>,
    #[serde(default = "default_local_model_repo")]
    pub local_model_repo: String,
}

fn default_local_model_repo() -> String {
    "Qwen/Qwen2.5-0.5B-Instruct".to_string()
}

fn default_model() -> String {
    "claude-3-5-haiku-20241022".to_string()
}

fn default_max_tokens() -> u32 {
    1024
}

fn default_temperature() -> f32 {
    0.7
}

impl Default for AIConfig {
    fn default() -> Self {
        Self {
            provider: AIProvider::default(),
            anthropic_api_key: None,
            openai_api_key: None,
            gemini_api_key: None,
            model: default_model(),
            max_tokens: default_max_tokens(),
            temperature: default_temperature(),
            local_model_path: None,
            local_model_repo: default_local_model_repo(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
#[serde(rename_all = "lowercase")]
pub enum AIProvider {
    Anthropic,
    OpenAI,
    Gemini,
    #[cfg(feature = "local")]
    Local,
}

// Default to Local when compiled with local feature, otherwise Anthropic
impl Default for AIProvider {
    fn default() -> Self {
        #[cfg(feature = "local")]
        {
            AIProvider::Local
        }
        #[cfg(not(feature = "local"))]
        {
            AIProvider::Anthropic
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SecurityConfig {
    #[serde(default = "default_dangerous_commands")]
    pub dangerous_commands: HashSet<String>,
    #[serde(default = "default_true")]
    pub require_confirmation: bool,
    #[serde(default = "default_max_output_size")]
    pub max_output_size: usize,
    #[serde(default = "default_allowed_directories")]
    pub allowed_directories: Vec<String>,
    #[serde(default)]
    pub disable_dangerous_commands: bool,
}

fn default_true() -> bool {
    true
}

fn default_max_output_size() -> usize {
    1024 * 1024 // 1MB
}

fn default_allowed_directories() -> Vec<String> {
    vec!["~".to_string(), "./".to_string()]
}

fn default_dangerous_commands() -> HashSet<String> {
    [
        // Unix/Linux dangerous commands
        "rm -rf",
        "mkfs",
        "dd",
        "shutdown",
        "reboot",
        "> /dev",
        "format",
        // PowerShell dangerous commands
        "Remove-Item -Recurse",
        "Format-Volume",
        "Stop-Computer",
        "Restart-Computer",
        "Remove-Item -Force",
        // CMD dangerous commands
        "rmdir /s",
        "format ",
        "del /f",
        "shutdown",
    ]
    .iter()
    .map(|&s| s.to_string())
    .collect()
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            dangerous_commands: default_dangerous_commands(),
            require_confirmation: true,
            max_output_size: default_max_output_size(),
            allowed_directories: default_allowed_directories(),
            disable_dangerous_commands: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DisplayConfig {
    #[serde(default = "default_true")]
    pub show_execution_time: bool,
    #[serde(default = "default_true")]
    pub color_output: bool,
    #[serde(default)]
    pub verbose_mode: bool,
    #[serde(default = "default_true")]
    pub show_command_preview: bool,
    #[serde(default = "default_prompt_symbol")]
    pub prompt_symbol: String,
}

fn default_prompt_symbol() -> String {
    "❯".to_string()
}

impl Default for DisplayConfig {
    fn default() -> Self {
        Self {
            show_execution_time: true,
            color_output: true,
            verbose_mode: false,
            show_command_preview: true,
            prompt_symbol: default_prompt_symbol(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ShellConfig {
    #[serde(default)]
    pub preferred_shell: Option<String>,
    #[serde(default)]
    pub shell_aliases: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub environment_variables: std::collections::HashMap<String, String>,
    #[serde(default = "default_history_size")]
    pub history_size: usize,
    #[serde(default = "default_true")]
    pub enable_auto_correction: bool,
}

fn default_history_size() -> usize {
    1000
}

impl Default for ShellConfig {
    fn default() -> Self {
        Self {
            preferred_shell: None,
            shell_aliases: std::collections::HashMap::new(),
            environment_variables: std::collections::HashMap::new(),
            history_size: default_history_size(),
            enable_auto_correction: true,
        }
    }
}

impl Config {
    pub fn load(config_path: &PathBuf) -> Result<Self> {
        let config_str = fs::read_to_string(config_path)?;
        let config: Config = toml::from_str(&config_str)?;
        Ok(config)
    }

    pub fn create_default(config_path: &PathBuf) -> Result<()> {
        if let Some(dir) = config_path.parent() {
            fs::create_dir_all(dir)?;
        }

        let default_config = Config {
            ai: AIConfig {
                provider: AIProvider::Anthropic,
                anthropic_api_key: Some("your-anthropic-api-key-here".to_string()),
                openai_api_key: Some("your-openai-api-key-here".to_string()),
                gemini_api_key: Some("your-gemini-api-key-here".to_string()),
                model: "claude-3-5-haiku-20241022".to_string(),
                max_tokens: 1024,
                temperature: 0.7,
                local_model_path: None,
                local_model_repo: "Qwen/Qwen2.5-0.5B-Instruct".to_string(),
            },
            security: SecurityConfig::default(),
            display: DisplayConfig::default(),
            shell: ShellConfig::default(),
        };

        let toml_string = toml::to_string_pretty(&default_config)?;
        fs::write(config_path, toml_string)?;
        Ok(())
    }

    pub fn update(&self, config_path: &PathBuf) -> Result<()> {
        let toml_string = toml::to_string_pretty(&self)?;
        fs::write(config_path, toml_string)?;
        Ok(())
    }

    /// Get the appropriate model for the configured provider
    pub fn get_default_model_for_provider(&self) -> &str {
        match self.ai.provider {
            AIProvider::Anthropic => "claude-3-5-haiku-20241022",
            AIProvider::OpenAI => "gpt-4o-mini",
            AIProvider::Gemini => "gemini-2.0-flash",
            #[cfg(feature = "local")]
            AIProvider::Local => "Qwen/Qwen2.5-0.5B-Instruct",
        }
    }
}

pub fn get_config_path() -> Result<PathBuf> {
    let home = home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    Ok(home.join(".config").join("spren").join("config.toml"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_config_creation() -> Result<()> {
        let temp_dir = tempdir()?;
        let config_path = temp_dir.path().join("config.toml");

        Config::create_default(&config_path)?;
        assert!(config_path.exists());

        let config = Config::load(&config_path)?;
        assert_eq!(config.ai.provider, AIProvider::Anthropic);
        assert!(config.security.require_confirmation);

        Ok(())
    }

    #[test]
    fn test_dangerous_commands() -> Result<()> {
        let temp_dir = tempdir()?;
        let config_path = temp_dir.path().join("config.toml");

        Config::create_default(&config_path)?;
        let config = Config::load(&config_path)?;

        assert!(config.security.dangerous_commands.contains("rm -rf"));
        assert!(config.security.dangerous_commands.contains("Format-Volume"));

        Ok(())
    }

    #[test]
    fn test_minimal_config() -> Result<()> {
        let temp_dir = tempdir()?;
        let config_path = temp_dir.path().join("config.toml");

        // Write a minimal config with just the provider
        fs::write(
            &config_path,
            r#"
[ai]
provider = "openai"
openai_api_key = "sk-test"
"#,
        )?;

        let config = Config::load(&config_path)?;
        assert_eq!(config.ai.provider, AIProvider::OpenAI);
        assert_eq!(config.ai.max_tokens, 1024); // default
        assert_eq!(config.ai.temperature, 0.7); // default

        Ok(())
    }

    #[test]
    fn test_gemini_provider() -> Result<()> {
        let temp_dir = tempdir()?;
        let config_path = temp_dir.path().join("config.toml");

        fs::write(
            &config_path,
            r#"
[ai]
provider = "gemini"
gemini_api_key = "test-key"
model = "gemini-2.0-flash"
"#,
        )?;

        let config = Config::load(&config_path)?;
        assert_eq!(config.ai.provider, AIProvider::Gemini);

        Ok(())
    }
}
