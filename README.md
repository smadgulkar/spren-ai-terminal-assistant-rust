# Spren

**Natural language to shell commands. Runs locally. No API keys needed.**

[![GitHub release](https://img.shields.io/github/v/release/smadgulkar/spren-ai-terminal-assistant-rust)](https://github.com/smadgulkar/spren-ai-terminal-assistant-rust/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-green.svg)](LICENSE)
[![OS](https://img.shields.io/badge/OS-Linux%20%7C%20macOS%20%7C%20Windows-blue)]()

```
spren> what is using my disk space

Suggested command: (5s)
du -h --max-depth=1 / 2>/dev/null | sort -hr

Execute? [y/N] y
```

## Why Spren?

- **100% Local** - Runs entirely on your CPU. No cloud, no API keys, no internet required
- **Zero Config** - Download, extract, run. That's it
- **Fast** - ~5 second inference on modern CPUs
- **Private** - Your commands never leave your machine
- **Cross-Platform** - Linux, macOS, Windows (Bash, Zsh, PowerShell, CMD)

## Quick Start

### Linux
```bash
curl -LO https://github.com/smadgulkar/spren-ai-terminal-assistant-rust/releases/latest/download/spren-linux-amd64.tar.gz
tar xzf spren-linux-amd64.tar.gz
./spren
```

### macOS
```bash
curl -LO https://github.com/smadgulkar/spren-ai-terminal-assistant-rust/releases/latest/download/spren-macos-amd64.tar.gz
tar xzf spren-macos-amd64.tar.gz
./spren
```

### Windows
Download `spren-windows-amd64.zip` from [releases](https://github.com/smadgulkar/spren-ai-terminal-assistant-rust/releases), extract, and run `spren.exe`.

## Examples

```
spren> find all python files modified today
Suggested command: find . -name "*.py" -mtime 0

spren> show me running docker containers
Suggested command: docker ps

spren> compress this folder
Suggested command: tar -czvf folder.tar.gz folder/

spren> kill process on port 3000  
Suggested command: kill $(lsof -t -i:3000)

spren> what's my public IP
Suggested command: curl -s ifconfig.me
```

## How It Works

Spren uses a fine-tuned [Qwen2.5-0.5B](https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct) model, quantized to 4-bit (Q4_K_M) for efficient CPU inference. The model was trained on 20,000+ shell command examples covering:

- File operations (ls, find, cp, mv, rm)
- Process management (ps, kill, top)
- Networking (curl, wget, ssh, ping)
- Package managers (apt, brew, pacman)
- Git, Docker, and more

The model runs via [Candle](https://github.com/huggingface/candle), Hugging Face's Rust ML framework.

## Requirements

- ~400MB disk space (model included)
- ~500MB RAM during inference
- Any modern CPU (no GPU required)

## Cloud Mode (Optional)

If you prefer cloud APIs for faster/smarter responses, Spren also supports:

- **Anthropic** (Claude)
- **OpenAI** (GPT-4o)
- **Google** (Gemini)

Create a config file at `~/.config/spren/config.toml`:

```toml
[ai]
provider = "openai"  # or "anthropic" or "gemini"
openai_api_key = "sk-..."
```

## Building from Source

```bash
# Clone
git clone https://github.com/smadgulkar/spren-ai-terminal-assistant-rust.git
cd spren-ai-terminal-assistant-rust

# Download model files
mkdir -p models
curl -L -o models/spren-model.gguf "https://huggingface.co/smadgulkar/spren-shell-model/resolve/main/spren-model.gguf"
curl -L -o models/tokenizer.json "https://huggingface.co/Qwen/Qwen2.5-0.5B-Instruct/resolve/main/tokenizer.json"

# Build with local LLM support
cargo build --release --features local

# Run
./target/release/spren
```

## Safety

Spren flags dangerous commands (like `rm -rf`) and always asks for confirmation before execution. You stay in control.

```
spren> delete everything in this folder

Suggested command: rm -rf ./* [DANGEROUS]

This command has been identified as potentially dangerous.
Execute? [y/N]
```

## License

MIT

## Links

- [Model on Hugging Face](https://huggingface.co/smadgulkar/spren-shell-model)
- [Report Issues](https://github.com/smadgulkar/spren-ai-terminal-assistant-rust/issues)
