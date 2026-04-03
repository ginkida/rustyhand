<p align="center">
  <img src="crates/rusty-hand-api/static/logo.svg" width="160" alt="RustyHand Logo" />
</p>

<h1 align="center">RustyHand</h1>
<h3 align="center">The Agent Operating System</h3>

<p align="center">
  Open-source Agent OS built in Rust. 132K LOC. 10 crates. 1,500+ tests. Zero clippy warnings.<br/>
  <strong>One binary. Battle-tested. Agents that actually work for you.</strong>
</p>

<p align="center">
  <a href="https://rustyhand.sh/docs">Documentation</a> &bull;
  <a href="https://rustyhand.sh/docs/getting-started">Quick Start</a> &bull;
  <a href="https://discord.gg/sSJqgNnq6X">Discord</a> &bull;
  <a href="https://x.com/rustyhandg">Twitter / X</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/language-Rust-orange?style=flat-square" alt="Rust" />
  <img src="https://img.shields.io/badge/license-MIT-blue?style=flat-square" alt="MIT" />
  <img src="https://img.shields.io/badge/version-0.1.0-green?style=flat-square" alt="v0.1.0" />
  <img src="https://img.shields.io/badge/tests-1,500%2B%20passing-brightgreen?style=flat-square" alt="Tests" />
  <img src="https://img.shields.io/badge/clippy-0%20warnings-brightgreen?style=flat-square" alt="Clippy" />
</p>

---

> **v0.1.0 — First Release (February 2026)**
>
> RustyHand is feature-complete but this is the first public release. You may encounter rough edges or breaking changes between minor versions. Pin to a specific commit for production use until v1.0. [Report issues here.](https://github.com/ginkida/rustyhand/issues)

---

## Origin

This project is based on [OpenFang](https://github.com/RightNow-AI/openfang) by RightNow-AI, modified and extended for custom use cases.

## Table of Contents

- [What is RustyHand?](#what-is-rustyhand)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Configuration](#configuration)
- [CLI Reference](#cli-reference)
- [Autonomous Templates](#autonomous-templates)
- [30 Pre-built Agent Templates](#30-pre-built-agent-templates)
- [37 Channel Adapters](#37-channel-adapters)
- [27 LLM Providers — 130+ Models](#27-llm-providers--130-models)
- [Architecture](#architecture)
- [API Endpoints](#api-endpoints)
- [Dashboard](#dashboard)
- [Security](#security)
- [Deployment](#deployment)
- [Development](#development)
- [Benchmarks](#benchmarks)
- [License](#license)

---

## What is RustyHand?

RustyHand is an **open-source Agent Operating System** — not a chatbot framework, not a Python wrapper around an LLM. It is a full operating system for autonomous agents, built from scratch in Rust.

Traditional agent frameworks wait for you to type something. RustyHand runs **autonomous agents that work for you** — on schedules, 24/7, building knowledge graphs, monitoring targets, generating leads, managing social media, and reporting results to your dashboard.

The entire system compiles to a **single ~32MB binary**. One install, one command, your agents are live.

---

## Installation

### One-liner (Linux / macOS / WSL)

```bash
curl -fsSL https://rustyhand.sh/install | sh
```

Environment variables:
- `RUSTY_HAND_INSTALL_DIR` — custom install path (default: `~/.rustyhand/bin`)
- `RUSTY_HAND_VERSION` — pin a specific version tag

### Windows (PowerShell)

```powershell
irm https://rustyhand.sh/install.ps1 | iex
```

### From source

```bash
git clone https://github.com/ginkida/rustyhand.git
cd rustyhand
cargo build --release -p rusty-hand-cli
# Binary: target/release/rustyhand (or rustyhand.exe on Windows)
```

Requires Rust 1.75+ (stable). The `rust-toolchain.toml` in the repo will auto-select the right toolchain.

### Docker

```bash
docker compose up --build
# Dashboard at http://localhost:4200
```

Or build manually:

```bash
docker build -t rustyhand .
docker run -p 4200:4200 \
  -e MINIMAX_API_KEY=your-key \
  -v rustyhand-data:/data \
  rustyhand
```

---

## Quick Start

```bash
# 1. Initialize — creates ~/.rustyhand/ and walks you through provider setup
rustyhand init

# 2. Start the daemon (API + kernel)
rustyhand start
# Dashboard is live at http://localhost:4200

# 3. Chat with the default agent
rustyhand chat

# 4. Spawn a pre-built agent
rustyhand agent new coder

# 5. Create an autonomous agent from the dashboard
# Open http://localhost:4200 → Agents → Templates → choose an autonomous template

# 6. Send a one-shot message
rustyhand message researcher "What are the emerging trends in AI agent frameworks?"

# 7. Launch the interactive TUI dashboard
rustyhand tui

# 8. Run diagnostics
rustyhand doctor
```

---

## Configuration

### Config file

Location: `~/.rustyhand/config.toml`

```toml
# API server settings
api_key = "your-bearer-token"          # Recommended for non-localhost access
api_listen = "127.0.0.1:4200"          # HTTP bind address

[default_model]
provider = "minimax"                   # minimax, anthropic, gemini, openai, groq, ollama, etc.
model = "MiniMax-M2.7"                 # Model identifier (or MiniMax-M2.7-highspeed)
api_key_env = "MINIMAX_API_KEY"        # Env var holding the API key
# base_url = "https://api.minimax.io/v1"  # Optional: override endpoint

[memory]
decay_rate = 0.05                      # Memory confidence decay
# sqlite_path = "~/.rustyhand/data/rustyhand.db"

[network]
listen_addr = "127.0.0.1:4200"        # RHP P2P listen address
# shared_secret = ""                  # Required for P2P authentication

# Session compaction (LLM-based context management)
[compaction]
threshold = 80                         # Compact when messages exceed this count
keep_recent = 20                       # Keep this many recent messages
max_summary_tokens = 1024

# Usage display in chat responses
# usage_footer = "Full"               # Off, Tokens, Cost, Full

# Channel adapters (tokens via env vars)
[telegram]
bot_token_env = "TELEGRAM_BOT_TOKEN"
allowed_users = []                     # Empty = allow all

[discord]
bot_token_env = "DISCORD_BOT_TOKEN"
# guild_ids = []

[slack]
bot_token_env = "SLACK_BOT_TOKEN"
app_token_env = "SLACK_APP_TOKEN"

# MCP server connections
[[mcp_servers]]
name = "filesystem"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"]
```

### Environment variables

Copy `.env.example` to `~/.rustyhand/.env` and fill in the keys you need:

```bash
# LLM providers (set the ones you use)
MINIMAX_API_KEY=eyJ...             # MiniMax M2.7 ($0.30/M input — recommended)
ANTHROPIC_API_KEY=sk-ant-...
OPENAI_API_KEY=sk-...
GROQ_API_KEY=gsk_...
DEEPSEEK_API_KEY=sk-...

# Local LLM providers (no key needed)
OLLAMA_BASE_URL=http://localhost:11434
VLLM_BASE_URL=http://localhost:8000
LMSTUDIO_BASE_URL=http://localhost:1234

# Channel tokens
TELEGRAM_BOT_TOKEN=123456:ABC-...
DISCORD_BOT_TOKEN=...
SLACK_BOT_TOKEN=xoxb-...
SLACK_APP_TOKEN=xapp-...

# Log level
RUST_LOG=info
# RUST_LOG=rusty_hand=debug            # Debug RustyHand only
```

### Manage config from the CLI

```bash
rustyhand config show                              # Print current config
rustyhand config edit                              # Open in $EDITOR
rustyhand config get default_model.provider        # Read a key
rustyhand config set default_model.provider groq   # Write a key
rustyhand config set-key groq                      # Interactively save API key
rustyhand config test-key groq                     # Verify connectivity
```

---

## CLI Reference

### Core commands

| Command | Description |
|---------|-------------|
| `rustyhand init` | Initialize `~/.rustyhand/` and default config |
| `rustyhand start` | Start the daemon (API server + kernel) |
| `rustyhand stop` | Stop the running daemon |
| `rustyhand status [--json]` | Show kernel status |
| `rustyhand health [--json]` | Quick daemon health check |
| `rustyhand doctor [--repair]` | Run diagnostic checks |
| `rustyhand tui` | Launch interactive TUI dashboard |
| `rustyhand dashboard` | Open web dashboard in browser |
| `rustyhand chat [agent]` | Quick chat with an agent |
| `rustyhand message <agent> <text>` | Send a one-shot message |
| `rustyhand logs [--follow] [--lines N]` | Tail the log file |
| `rustyhand reset [--confirm]` | Reset local config and state |

### Agents

| Command | Description |
|---------|-------------|
| `rustyhand agent new [template]` | Spawn from a template (interactive picker if omitted) |
| `rustyhand agent spawn <manifest.toml>` | Spawn from a manifest file |
| `rustyhand agent list [--json]` | List running agents |
| `rustyhand agent chat <id>` | Interactive chat with an agent by ID |
| `rustyhand agent kill <id>` | Kill an agent |

### Channels

| Command | Description |
|---------|-------------|
| `rustyhand channel list` | List configured channels and status |
| `rustyhand channel setup [name]` | Interactive channel setup wizard |
| `rustyhand channel test <name>` | Send a test message |
| `rustyhand channel enable <name>` | Enable a channel |
| `rustyhand channel disable <name>` | Disable a channel |

### Models

| Command | Description |
|---------|-------------|
| `rustyhand models list [--provider X]` | Browse available models |
| `rustyhand models aliases` | Show model shorthand names |
| `rustyhand models providers` | List providers and their auth status |
| `rustyhand models set [model]` | Set the default model |

### Skills

| Command | Description |
|---------|-------------|
| `rustyhand skill install <source>` | Install from ClawHub, local path, or git URL |
| `rustyhand skill list` | List installed skills |
| `rustyhand skill search <query>` | Search ClawHub marketplace |
| `rustyhand skill remove <name>` | Remove a skill |
| `rustyhand skill create` | Scaffold a new skill |

### Workflows & scheduling

| Command | Description |
|---------|-------------|
| `rustyhand workflow list` | List workflows |
| `rustyhand workflow create <file.json>` | Create from JSON |
| `rustyhand workflow run <id> <input>` | Run a workflow |
| `rustyhand trigger list [--agent-id X]` | List event triggers |
| `rustyhand trigger create <agent-id> <pattern-json>` | Create a trigger |
| `rustyhand cron list` | List scheduled jobs |

### Integrations (MCP)

| Command | Description |
|---------|-------------|
| `rustyhand add <name> [--key TOKEN]` | Install an integration (e.g., `github`, `notion`) |
| `rustyhand remove <name>` | Remove an integration |
| `rustyhand integrations [query]` | List / search integrations |

### Security & vault

| Command | Description |
|---------|-------------|
| `rustyhand vault init` | Initialize the credential vault (AES-256-GCM) |
| `rustyhand vault set <key>` | Store a credential |
| `rustyhand vault list` | List stored keys (values hidden) |
| `rustyhand vault remove <key>` | Remove a credential |
| `rustyhand security audit` | View the audit trail |
| `rustyhand security rbac` | Manage access control |

### Other

| Command | Description |
|---------|-------------|
| `rustyhand mcp` | Start MCP server over stdio |
| `rustyhand sessions [agent]` | List conversation sessions |
| `rustyhand approvals list` | List pending approval requests |
| `rustyhand qr` | Generate device pairing QR code |
| `rustyhand onboard` | Interactive onboarding wizard |
| `rustyhand completion <shell>` | Generate shell completions (bash/zsh/fish/powershell) |
| `rustyhand new skill\|integration` | Scaffold a new skill or integration |

All list/status commands support `--json` for scripting.

---

## Autonomous Templates

RustyHand ships autonomous templates as agent presets in the dashboard. They are not a separate runtime entity: each template creates a normal agent, and you can optionally attach a cron schedule during creation.

Each autonomous template bundles:
- **Agent preset** — model, profile, prompt, and capabilities
- **Schedule defaults** — suggested cron expression and trigger message
- **Operational playbook** — multi-phase prompt for recurring work
- **Guardrails** — approval and tool constraints where needed

### Bundled Autonomous Templates

| Template | What It Does |
|------|-------------|
| **GitHub Monitor** | Monitors repositories, runs tests, detects regressions, and files issues on a schedule. |
| **Web Researcher** | Runs recurring research sweeps, cross-references sources, and produces structured reports. |
| **Content Clipper** | Processes long-form video into short clips with captions and packaging. |
| **Lead Generator** | Discovers and enriches qualified leads on a recurring schedule. |
| **Intel Collector** | Monitors targets, detects changes, and updates a living knowledge base. |
| **Predictor** | Collects signals, updates forecasts, and tracks prediction accuracy. |
| **Twitter Manager** | Creates, schedules, and reviews social content with approval controls. |
| **Web Browser** | Executes recurring browser automation tasks with strict purchase approval gates. |

Use the dashboard to launch one: **Agents → Templates** or **Create Agent → enable schedule**.

---

## 30 Pre-built Agent Templates

Spawn any template with `rustyhand agent new <name>`:

| Template | Description |
|----------|-------------|
| `analyst` | Data analysis and reporting |
| `architect` | System design and architecture |
| `assistant` | General-purpose assistant |
| `code-reviewer` | Code review and feedback |
| `coder` | Software development |
| `customer-support` | Customer support |
| `data-scientist` | Data science and ML |
| `debugger` | Bug investigation |
| `devops-lead` | DevOps and infrastructure |
| `doc-writer` | Documentation |
| `email-assistant` | Email drafting and management |
| `health-tracker` | Health and fitness tracking |
| `hello-world` | Starter agent for new users |
| `home-automation` | Smart home control |
| `legal-assistant` | Legal document review |
| `meeting-assistant` | Meeting notes and follow-ups |
| `ops` | Operations management |
| `orchestrator` | Multi-agent orchestration |
| `personal-finance` | Financial tracking |
| `planner` | Project planning |
| `recruiter` | Recruiting and screening |
| `researcher` | Research and analysis |
| `sales-assistant` | Sales support |
| `security-auditor` | Security analysis |
| `social-media` | Social media management |
| `test-engineer` | Testing and QA |
| `translator` | Multi-language translation |
| `travel-planner` | Travel planning |
| `tutor` | Education and tutoring |
| `writer` | Content writing |

### Agent manifest format (`agent.toml`)

```toml
name = "hello-world"
version = "0.1.0"
description = "A friendly greeting agent"
author = "rusty-hand"
module = "builtin:chat"

[model]
provider = "groq"
model = "llama-3.3-70b-versatile"
max_tokens = 4096
temperature = 0.6
system_prompt = """Your system prompt here..."""

[resources]
max_llm_tokens_per_hour = 100000

[capabilities]
tools = ["file_read", "file_list", "web_fetch", "web_search", "memory_store", "memory_recall"]
network = ["*"]
memory_read = ["*"]
memory_write = ["self.*"]
agent_spawn = false
```

---

## 37 Channel Adapters

Connect agents to every platform your users are on. Each adapter supports per-channel model overrides, DM/group policies, rate limiting, and output formatting.

**Core:** Telegram, Discord, Slack, WhatsApp, Signal, Matrix
**Enterprise:** Microsoft Teams, Mattermost, Google Chat, Webex, Feishu/Lark, Zulip
**Social:** LINE, Viber, Facebook Messenger, Mastodon, Bluesky, Reddit, LinkedIn, Twitch
**Community:** IRC, Guilded, Revolt, Keybase, Discourse, Gitter, Rocket.Chat
**Privacy:** Threema, Mumble, Nextcloud Talk, Ntfy, Gotify
**Workplace:** Pumble, Flock, Twist, DingTalk, Webhooks

### Channel policies

Each channel supports fine-grained control in `config.toml`:

```toml
[telegram]
bot_token_env = "TELEGRAM_BOT_TOKEN"
allowed_users = [123456789]            # Restrict to specific users
dm_policy = "Respond"                  # Respond | AllowedOnly | Ignore
group_policy = "MentionOnly"           # All | MentionOnly | CommandsOnly | Ignore
output_format = "TelegramHtml"         # Markdown | TelegramHtml | SlackMrkdwn | PlainText
```

---

## 27 LLM Providers — 130+ Models

3 native drivers (Anthropic, Gemini, OpenAI-compatible) route to 27 providers:

**Cloud:** Anthropic, OpenAI, Gemini, Groq, DeepSeek, OpenRouter, Together, Mistral, Fireworks, Cohere, Perplexity, xAI, AI21, Cerebras, SambaNova, HuggingFace, Replicate, Qwen, MiniMax, Zhipu, Moonshot, Qianfan, Bedrock, Copilot
**Local:** Ollama, vLLM, LM Studio

Features:
- Intelligent routing with task complexity scoring
- Automatic fallback between providers
- Per-model pricing and cost tracking
- Per-agent budget limits

```bash
rustyhand models list                 # Browse all models
rustyhand models list --provider groq # Filter by provider
rustyhand models set claude-sonnet    # Set default model
```

---

## Architecture

10 Rust crates with a modular kernel design:

```
rusty-hand-types       Core types, traits, config, taint tracking, Ed25519 manifest signing
    |
    +-- rusty-hand-memory      SQLite persistence, vector embeddings, session compaction
    +-- rusty-hand-wire        RHP P2P protocol (JSON-RPC over TCP, HMAC-SHA256 auth)
    +-- rusty-hand-channels    37 messaging adapters with rate limiting
    +-- rusty-hand-skills      Skill system + ClawHub marketplace
    +-- rusty-hand-extensions  25 MCP integrations, AES-256-GCM credential vault, OAuth2
    |
    +-- rusty-hand-runtime     Agent loop, 3 LLM drivers, 53+ tools, WASM sandbox, MCP, A2A
    |
    +-- rusty-hand-kernel      Orchestration: lifecycle, scheduling, metering, RBAC, workflows
    |
    +-- rusty-hand-api         Axum HTTP daemon, 108+ endpoints, WebSocket, SSE, OpenAI-compat
    |
    +-- rusty-hand-cli         CLI binary + TUI dashboard (ratatui)
```

### Key internals

| Concept | Implementation |
|---------|---------------|
| **Agent loop** | `rusty-hand-runtime` — iterative LLM call → tool execution → response cycle |
| **Kernel** | `RustyHandKernel` struct (130+ fields) — central orchestration for all subsystems |
| **AppState** | Bridges kernel to HTTP routes via `Arc<RustyHandKernel>` in Axum state |
| **Sandbox** | WASM (wasmtime) with fuel metering + epoch interruption + watchdog thread |
| **Memory** | SQLite + vector embeddings for semantic search + knowledge graph |
| **Metering** | Per-agent token/cost tracking with budget enforcement and alerts |
| **P2P** | RHP (RustyHand Protocol) — JSON-RPC over TCP, Ed25519 identity, nonce-based auth |
| **A2A** | Agent-to-Agent protocol for cross-instance agent communication |

---

## API Endpoints

Default: `http://127.0.0.1:4200`. All endpoints return JSON. Authenticate with `Authorization: Bearer <api_key>` when `api_key` is set in config.

### Health & status

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/health` | GET | Health check |
| `/api/status` | GET | Full kernel status |

### Agents

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/agents` | GET | List all agents |
| `/api/agents` | POST | Spawn a new agent |
| `/api/agents/{id}` | GET | Agent details |
| `/api/agents/{id}` | DELETE | Kill an agent |
| `/api/agents/{id}/message` | POST | Send message (triggers LLM) |

### Budget

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/budget` | GET | Global budget status |
| `/api/budget` | PUT | Update budget settings |
| `/api/budget/agents` | GET | Per-agent cost ranking |
| `/api/budget/agents/{id}` | GET | Single agent budget detail |

### Network & P2P

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/network/status` | GET | RHP network status |
| `/api/peers` | GET | Connected peers |

### A2A (Agent-to-Agent)

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/api/a2a/agents` | GET | External A2A agents |
| `/api/a2a/discover` | POST | Discover agent at URL |
| `/api/a2a/send` | POST | Send task to external agent |
| `/api/a2a/tasks/{id}/status` | GET | Check task status |

### OpenAI-compatible

Drop-in replacement for OpenAI API:

```bash
curl -X POST http://localhost:4200/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{
    "model": "researcher",
    "messages": [{"role": "user", "content": "Analyze Q4 market trends"}],
    "stream": true
  }'
```

Full REST/WS/SSE endpoints cover agents, memory, workflows, channels, models, skills, sessions, approvals, triggers, crons, security, and more (140+ total).

---

## Dashboard

The web dashboard is served at `http://localhost:4200` when the daemon is running. Built with Alpine.js — no build step, no node_modules.

### Sections

| Section | What you see |
|---------|-------------|
| **Chat** | Real-time agent conversations |
| **Monitor** | Overview, analytics charts, system logs |
| **Agents** | Session history, pending approval requests |
| **Automation** | Workflows, event triggers, scheduled cron jobs |
| **Security** | RBAC management, audit trail |
| **Network** | Connected RHP peers, external A2A agents |
| **Templates** | Prebuilt agent presets and autonomous starters |
| **Skills** | Skill browser, marketplace search |
| **Settings** | API keys, channel tokens, model selection, theme |

Connects via WebSocket with HTTP fallback. Supports dark/light/system themes.

---

## Security

16 independent security layers — defense in depth, no single point of failure.

| # | System | Description |
|---|--------|-------------|
| 1 | **WASM Dual-Metered Sandbox** | Tool code runs in WebAssembly with fuel metering + epoch interruption. Watchdog kills runaway code. |
| 2 | **Merkle Hash-Chain Audit Trail** | Every action cryptographically linked. Tamper with one entry and the chain breaks. |
| 3 | **Taint Tracking** | Information flow labels propagate through execution — secrets tracked from source to sink. |
| 4 | **Ed25519 Signed Manifests** | Agent identity and capabilities are cryptographically signed. |
| 5 | **SSRF Protection** | Blocks private IPs, cloud metadata endpoints, DNS rebinding. |
| 6 | **Secret Zeroization** | `Zeroizing<String>` auto-wipes API keys from memory when no longer needed. |
| 7 | **RHP Mutual Auth** | HMAC-SHA256 nonce-based, constant-time verification for P2P. |
| 8 | **Capability Gates** | Role-based access control — agents declare tools, kernel enforces. |
| 9 | **Security Headers** | CSP, X-Frame-Options, HSTS, X-Content-Type-Options on every response. |
| 10 | **Health Redaction** | Public health check returns minimal info. Full diagnostics require auth. |
| 11 | **Subprocess Sandbox** | `env_clear()` + selective passthrough. Process tree isolation with cross-platform kill. |
| 12 | **Prompt Injection Scanner** | Detects override attempts, data exfiltration patterns, shell injection in skills. |
| 13 | **Loop Guard** | SHA256-based tool call loop detection with circuit breaker. |
| 14 | **Session Repair** | 7-phase message history validation and automatic recovery. |
| 15 | **Path Traversal Prevention** | Canonicalization with symlink escape prevention. |
| 16 | **GCRA Rate Limiter** | Cost-aware token bucket rate limiting with per-IP tracking. |

---

## Deployment

### Systemd

A service file is provided in `deploy/rustyhand.service`:

```bash
sudo cp deploy/rustyhand.service /etc/systemd/system/rustyhand.service
# Edit ExecStart path and user as needed
sudo systemctl daemon-reload
sudo systemctl enable --now rustyhand
```

The service includes security hardening: `NoNewPrivileges`, `ProtectSystem=strict`, `ProtectHome`, `PrivateTmp`, and resource limits.

### Docker

```yaml
# docker-compose.yml
services:
  rustyhand:
    build: .
    ports:
      - "4200:4200"
    volumes:
      - rustyhand-data:/data
    env_file:
      - path: .env
        required: false
    environment:
      - MINIMAX_API_KEY=${MINIMAX_API_KEY:-}
      - GROQ_API_KEY=${GROQ_API_KEY:-}
      - TELEGRAM_BOT_TOKEN=${TELEGRAM_BOT_TOKEN:-}
    restart: unless-stopped

volumes:
  rustyhand-data:
```

### Cross-compilation

Cross-compilation to `aarch64-unknown-linux-gnu` is supported via `Cross.toml`:

```bash
cross build --release --target aarch64-unknown-linux-gnu -p rusty-hand-cli
```

---

## Development

### Prerequisites

- Rust 1.75+ (stable) — `rust-toolchain.toml` auto-selects
- Components: `rustfmt`, `clippy` (included in toolchain)

### Build & verify

```bash
# Compile all crates (use --lib if the daemon binary is locked)
cargo build --workspace --lib

# Run all tests (1,500+)
cargo test --workspace

# Lint — must be 0 warnings
cargo clippy --workspace --all-targets -- -D warnings

# Format check
cargo fmt --all -- --check
```

### Release build

```bash
cargo build --release -p rusty-hand-cli
# Binary: target/release/rustyhand (~32 MB)
# LTO + single codegen unit + stripped symbols + opt-level 3
```

### Project structure

```
rustyhand/
  Cargo.toml                # Workspace manifest (10 member crates)
  Cargo.lock
  rust-toolchain.toml       # Rust stable + rustfmt + clippy
  .env.example              # Environment variable template
  Dockerfile                # Multi-stage build
  docker-compose.yml
  Cross.toml                # Cross-compilation config
  agents/                   # 30 pre-built agent templates (agent.toml each)
  deploy/                   # systemd service, Docker scripts
  scripts/                  # install.sh, install.ps1
  crates/
    rusty-hand-types/       # Core types (config.rs is the master config struct)
    rusty-hand-memory/      # SQLite + vector embeddings
    rusty-hand-runtime/     # Agent loop + LLM drivers + tools + sandbox
    rusty-hand-wire/        # RHP P2P protocol
    rusty-hand-api/         # Axum HTTP server + routes + dashboard
      src/
        server.rs           # Router setup, middleware, AppState
        routes.rs           # All API endpoint handlers (~8500 LOC)
      static/
        index_body.html     # Dashboard SPA (Alpine.js)
        index_head.html     # CSS + fonts
    rusty-hand-kernel/      # Central kernel (~5000 LOC, 130+ fields)
    rusty-hand-cli/         # CLI + TUI binary
    rusty-hand-channels/    # 37 messaging adapters
    rusty-hand-skills/      # Skill system + ClawHub + OpenClaw compat
    rusty-hand-extensions/  # MCP + vault + OAuth2
```

### Key files for contributors

| File | What it does |
|------|-------------|
| `crates/rusty-hand-kernel/src/kernel.rs` | The kernel — 130+ fields, central orchestration |
| `crates/rusty-hand-api/src/routes.rs` | All API handlers (~8500 LOC) |
| `crates/rusty-hand-api/src/server.rs` | Router, middleware, `AppState` struct |
| `crates/rusty-hand-types/src/config.rs` | Master config struct (`KernelConfig`) |
| `crates/rusty-hand-api/static/index_body.html` | Dashboard SPA |
| `crates/rusty-hand-api/src/channel_bridge.rs` | Channel adapter wiring |
| `crates/rusty-hand-runtime/src/drivers/` | LLM drivers (anthropic.rs, gemini.rs, openai.rs) |

### Common gotchas

- `rustyhand.exe` may be locked if the daemon is running — use `--lib` flag or kill daemon first
- New config fields need: struct field + `#[serde(default)]` + `Default` impl entry
- New routes must be registered in `server.rs` router AND implemented in `routes.rs`
- Dashboard tabs need both HTML in `index_body.html` and JS data/methods
- `AgentLoopResult` field is `.response` not `.response_text`
- CLI daemon command is `start` (not `daemon`)

---

## Benchmarks

All data from official documentation and public repositories — February 2026.

| Metric | RustyHand | ZeroClaw | LangGraph | CrewAI | AutoGen | OpenClaw |
|--------|----------|----------|-----------|--------|---------|----------|
| **Cold start** | 180 ms | 10 ms | 2.5 s | 3.0 s | 4.0 s | 5.98 s |
| **Idle memory** | 40 MB | 5 MB | 180 MB | 200 MB | 250 MB | 394 MB |
| **Install size** | 32 MB | 8.8 MB | 150 MB | 100 MB | 200 MB | 500 MB |
| **Security layers** | 16 | 6 | 2 | 1 | 2 | 3 |
| **Channel adapters** | 37 | 15 | 0 | 0 | 0 | 13 |
| **LLM providers** | 27 | 28 | 15 | 10 | 8 | 10 |
| **Language** | Rust | Rust | Python | Python | Python | TypeScript |

---

## License

MIT — use it however you want.

---

## Links

- [Website & Documentation](https://rustyhand.sh)
- [Quick Start Guide](https://rustyhand.sh/docs/getting-started)
- [GitHub](https://github.com/ginkida/rustyhand)
- [Discord](https://discord.gg/sSJqgNnq6X)
- [Twitter / X](https://x.com/rustyhandg)

---

## Acknowledgments

RustyHand is a fork of [OpenFang](https://github.com/RightNow-AI/openfang), originally built by [Jaber](https://x.com/Akashi203) at [RightNow](https://www.rightnowai.co/).

---

<p align="center">
  <strong>Built with Rust. Secured with 16 layers. Agents that actually work for you.</strong>
</p>
