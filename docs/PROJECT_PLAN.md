# Spren: AI Shell Assistant

## Overview

Spren is a terminal-native AI assistant that converts natural language queries into shell commands. It's designed to be fast, lightweight, and work across all major platforms (Linux, macOS, Windows).

```
spren> what is using my disk space
Suggested command: du -h --max-depth=1 / 2>/dev/null | sort -hr
Execute? [y/N]
```

## Current State (v0.2.2)

### Features
- Natural language to shell command conversion
- Dangerous command detection and confirmation prompts
- Error analysis and suggestions when commands fail
- Cross-platform shell detection (Bash, Zsh, PowerShell, CMD)
- Multiple cloud AI providers:
  - Anthropic (Claude)
  - OpenAI (GPT-4o-mini)
  - Google (Gemini)

### Architecture
```
src/
├── main.rs        # CLI loop and user interaction
├── ai.rs          # AI provider implementations
├── config.rs      # Configuration management (TOML)
├── executor.rs    # Shell command execution
├── shell.rs       # Shell type detection
└── local_llm.rs   # Local LLM inference (feature-gated)
```

### Limitations
- Requires API keys and internet connection
- Response latency depends on cloud provider (500ms-2s)
- API costs accumulate with usage

---

## The Vision: Local-First AI

We're building toward a version of Spren that runs entirely offline using a small, fine-tuned language model optimized specifically for shell commands.

### Why Local?

| Aspect | Cloud API | Local LLM |
|--------|-----------|-----------|
| Latency | 500ms-2s | 50-200ms |
| Privacy | Commands sent to cloud | Everything stays local |
| Cost | Pay per request | Free after download |
| Offline | No | Yes |
| Dependencies | API key required | Just the binary |

### Target Specs
- **Binary size**: < 50MB (model weights embedded or downloaded once)
- **RAM usage**: < 500MB during inference
- **Response time**: < 200ms on modern CPU
- **No GPU required**: Pure CPU inference

---

## Implementation Plan

### Phase 1: Infrastructure (Complete)
- [x] Core CLI application
- [x] Multiple cloud provider support
- [x] Robust response parsing
- [x] Cross-platform shell detection
- [x] Configuration system with sensible defaults

### Phase 2: Local LLM Integration (In Progress)
- [x] Add Candle (Rust ML framework) integration
- [x] Implement local provider in config
- [x] Model download from HuggingFace Hub
- [ ] Test with base Qwen2.5-0.5B model
- [ ] Optimize inference speed

Branch: `feature/local-llm`

### Phase 3: Fine-Tuning Pipeline
- [ ] Create shell command dataset (10-50k examples)
- [ ] Cover major categories:
  - File operations (ls, cd, cp, mv, rm, find)
  - Process management (ps, kill, top, htop)
  - Network (curl, wget, ssh, ping, netstat)
  - Package managers (apt, yum, brew, pacman)
  - Git operations
  - Docker/Kubernetes
  - System info (df, du, free, uname)
- [ ] Include dangerous command labels
- [ ] Train LoRA adapter on Qwen2.5-0.5B
- [ ] Evaluate accuracy on held-out test set

### Phase 4: Model Optimization
- [ ] Quantize to Q4_K_M (4-bit) for smaller size
- [ ] Benchmark inference speed across platforms
- [ ] Consider smaller models if needed:
  - SmolLM2-360M (~700MB)
  - TinyLlama-1.1B (~2GB)
- [ ] Implement model caching and warm-up

### Phase 5: Distribution
- [ ] Embed model weights in binary OR
- [ ] First-run model download with progress bar
- [ ] GitHub releases with platform-specific binaries
- [ ] Installation scripts (curl | bash style)
- [ ] Package managers (brew, cargo install, apt)

---

## Fine-Tuning Dataset Structure

### Format
```jsonl
{"messages": [{"role": "user", "content": "list all files including hidden"}, {"role": "assistant", "content": "DANGEROUS:false\nCOMMAND:ls -la"}]}
{"messages": [{"role": "user", "content": "delete the entire home directory"}, {"role": "assistant", "content": "DANGEROUS:true\nCOMMAND:rm -rf ~"}]}
```

### Categories to Cover

1. **File System** (~30%)
   - Listing, navigation, creation, deletion
   - Permissions, ownership
   - Finding files, searching content

2. **Process Management** (~15%)
   - Viewing processes
   - Killing processes
   - Background jobs

3. **Networking** (~15%)
   - HTTP requests
   - SSH/SCP
   - Network diagnostics

4. **System Information** (~10%)
   - Disk usage
   - Memory usage
   - System stats

5. **Text Processing** (~10%)
   - grep, sed, awk
   - sort, uniq, wc
   - head, tail, cat

6. **Package Management** (~10%)
   - apt, yum, brew, pacman
   - npm, pip, cargo

7. **Version Control** (~5%)
   - Git operations

8. **Containers** (~5%)
   - Docker, kubectl basics

### Dangerous Command Coverage
Ensure dataset includes examples of:
- `rm -rf` variations
- `dd` operations
- Format/partition commands
- System shutdown/reboot
- Permission changes (chmod 777)
- Recursive deletions

---

## Technical Decisions

### Why Candle over llama.cpp?
- Pure Rust: No C++ build toolchain required
- Smaller binaries
- Native HuggingFace integration
- Active maintenance by HuggingFace

### Why Qwen2.5-0.5B?
- Good instruction-following capability
- Small enough for CPU inference
- Apache 2.0 license
- Strong multilingual support
- Proven fine-tuning results

### Fallback Strategy
If local model quality is insufficient:
1. Use local for common commands
2. Fall back to cloud for complex queries
3. Let user configure threshold

---

## Success Metrics

### Accuracy
- 95%+ correct commands for common operations
- 99%+ dangerous command detection rate
- < 1% false positive rate on safe commands

### Performance
- < 200ms response time on 4-core CPU
- < 500MB RAM during inference
- < 50MB binary size (without embedded model)

### User Experience
- Zero configuration for local mode
- Works offline after first model download
- Graceful degradation if model unavailable

---

## Contributing

### Current Priorities
1. Testing local LLM branch
2. Building fine-tuning dataset
3. Benchmarking inference performance

### How to Help
- Submit shell command examples for dataset
- Test on different platforms
- Report edge cases where parsing fails
- Suggest common commands we might have missed

---

## Timeline

This is a side project, so no hard deadlines, but rough goals:

- **Q1 2025**: Local LLM working with base model
- **Q2 2025**: Fine-tuned model with good accuracy
- **Q3 2025**: Optimized distribution, v1.0 release
