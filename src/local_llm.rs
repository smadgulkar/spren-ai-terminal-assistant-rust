//! Local LLM inference using Candle
//!
//! This module provides CPU-based inference for small language models,
//! allowing Spren to work without cloud API calls.

#[cfg(feature = "local")]
use anyhow::{anyhow, Result};
#[cfg(feature = "local")]
use candle_core::{DType, Device, Tensor};
#[cfg(feature = "local")]
use candle_nn::VarBuilder;
#[cfg(feature = "local")]
use candle_transformers::models::qwen2::{Config as Qwen2Config, ModelForCausalLM as Qwen2Model};
#[cfg(feature = "local")]
use hf_hub::{api::sync::Api, Repo, RepoType};
#[cfg(feature = "local")]
use std::path::PathBuf;
#[cfg(feature = "local")]
use tokenizers::Tokenizer;

#[cfg(feature = "local")]
use crate::config::Config;

/// Manages local LLM model loading and inference
#[cfg(feature = "local")]
pub struct LocalLLM {
    model: Qwen2Model,
    tokenizer: Tokenizer,
    device: Device,
    config: Qwen2Config,
}

#[cfg(feature = "local")]
impl LocalLLM {
    /// Load a model from HuggingFace Hub or local path
    pub fn load(app_config: &Config) -> Result<Self> {
        let device = Device::Cpu;

        let model_id = &app_config.ai.local_model_repo;

        println!("Loading local model: {}...", model_id);

        // Download or locate model files
        let api = Api::new()?;
        let repo = api.repo(Repo::new(model_id.clone(), RepoType::Model));

        let config_path = repo.get("config.json")?;
        let tokenizer_path = repo.get("tokenizer.json")?;
        let weights_path = repo.get("model.safetensors")?;

        // Load config
        let config: Qwen2Config = serde_json::from_str(&std::fs::read_to_string(&config_path)?)?;

        // Load tokenizer
        let tokenizer = Tokenizer::from_file(&tokenizer_path)
            .map_err(|e| anyhow!("Failed to load tokenizer: {}", e))?;

        // Load model weights
        let vb =
            unsafe { VarBuilder::from_mmaped_safetensors(&[weights_path], DType::F32, &device)? };

        let model = Qwen2Model::new(&config, vb)?;

        println!("Model loaded successfully!");

        Ok(Self {
            model,
            tokenizer,
            device,
            config,
        })
    }

    /// Generate a response for the given prompt
    pub fn generate(&self, prompt: &str, max_tokens: u32, temperature: f32) -> Result<String> {
        // Encode the prompt
        let encoding = self
            .tokenizer
            .encode(prompt, true)
            .map_err(|e| anyhow!("Tokenization failed: {}", e))?;

        let mut tokens: Vec<u32> = encoding.get_ids().to_vec();
        let prompt_len = tokens.len();

        // Generate tokens
        for _ in 0..max_tokens {
            let input = Tensor::new(tokens.as_slice(), &self.device)?.unsqueeze(0)?;

            let logits = self.model.forward(&input, prompt_len - 1)?;
            let logits = logits.squeeze(0)?;

            // Get the last token's logits
            let last_logits = logits.get(logits.dim(0)? - 1)?;

            // Apply temperature and sample
            let next_token = if temperature <= 0.0 {
                // Greedy sampling
                last_logits.argmax(0)?.to_scalar::<u32>()?
            } else {
                // Temperature sampling
                let scaled = (last_logits / temperature as f64)?;
                let probs = candle_nn::ops::softmax(&scaled, 0)?;
                sample_from_probs(&probs)?
            };

            // Check for EOS
            if next_token == self.config.eos_token_id.unwrap_or(151643) as u32 {
                break;
            }

            tokens.push(next_token);
        }

        // Decode the generated tokens (skip the prompt)
        let generated_tokens = &tokens[prompt_len..];
        let output = self
            .tokenizer
            .decode(generated_tokens, true)
            .map_err(|e| anyhow!("Decoding failed: {}", e))?;

        Ok(output)
    }
}

#[cfg(feature = "local")]
fn sample_from_probs(probs: &Tensor) -> Result<u32> {
    use rand::Rng;

    let probs_vec: Vec<f32> = probs.to_vec1()?;
    let mut rng = rand::thread_rng();
    let r: f32 = rng.gen();

    let mut cumsum = 0.0;
    for (i, p) in probs_vec.iter().enumerate() {
        cumsum += p;
        if cumsum >= r {
            return Ok(i as u32);
        }
    }

    Ok((probs_vec.len() - 1) as u32)
}

/// Get the path to the local models directory
#[cfg(feature = "local")]
pub fn get_models_dir() -> Result<PathBuf> {
    let home = dirs::home_dir().ok_or_else(|| anyhow!("Could not find home directory"))?;
    let models_dir = home.join(".cache").join("spren").join("models");
    std::fs::create_dir_all(&models_dir)?;
    Ok(models_dir)
}

// Stub for when local feature is not enabled
#[cfg(not(feature = "local"))]
pub struct LocalLLM;

#[cfg(not(feature = "local"))]
impl LocalLLM {
    pub fn load(_config: &crate::config::Config) -> anyhow::Result<Self> {
        anyhow::bail!("Local LLM support not compiled. Rebuild with: cargo build --features local")
    }

    pub fn generate(
        &self,
        _prompt: &str,
        _max_tokens: u32,
        _temperature: f32,
    ) -> anyhow::Result<String> {
        anyhow::bail!("Local LLM support not compiled")
    }
}
