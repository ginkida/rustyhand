<p align="center">
  <img src="crates/rusty-hand-api/static/logo.png" width="500" alt="RustyHand Logo" />
</p>

<h1 align="center">RustyHand</h1>
<h3 align="center">The Agent Operating System</h3>

<p align="center">
  Open-source Agent OS built in Rust. 124K LOC. 10 crates. 1,575 tests. Zero clippy warnings.<br/>
  <strong>One binary. Autonomous Telegram agent. Agents that actually work for you.</strong>
</p>

<p align="center">
  <a href="https://github.com/ginkida/rustyhand#quick-start">Quick Start</a> &bull;
  <a href="https://github.com/ginkida/rustyhand#cli-reference">CLI Reference</a> &bull;
  <a href="https://github.com/ginkida/rustyhand#api-endpoints">API Docs</a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/language-Rust-orange?style=flat-square" alt="Rust" />
  <img src="https://img.shields.io/badge/license-MIT-blue?style=flat-square" alt="MIT" />
  <img src="https://img.shields.io/badge/version-0.7.39-green?style=flat-square" alt="v0.7.39" />
  <img src="https://img.shields.io/badge/tests-1,577%20passing-brightgreen?style=flat-square" alt="Tests" />
  <img src="https://img.shields.io/badge/clippy-0%20warnings-brightgreen?style=flat-square" alt="Clippy" />
</p>

---

> **v0.7.33–v0.7.39 — Demo Mode + persistence + product polish (May 2026)**
>
> Clone → `cargo run --release start` → open the dashboard → talk to an agent.
> **No API key required.** When no provider key is found in the environment,
> RustyHand falls back to a deterministic mock driver and seeds four sample
> resources so every major dashboard page is interactive on first visit:
>
> - **`rusty`** welcome agent (chat-ready)
> - **`demo-pipeline`** workflow (2-step sample, click to run)
> - sample agent-spawn trigger
> - **`demo-daily-ping`** cron job (registered, disabled by default)
>
> A welcome modal on first visit lists all four with one-click navigation.
> The CLI startup banner and Docker entrypoint both announce demo mode
> as a feature instead of a missing-key warning. Set `ANTHROPIC_API_KEY`
> (or any of 26 other supported providers' env vars) and restart for real
> LLM responses, or `RUSTYHAND_DISABLE_DEMO_MODE=1` to force a hard fail.
>
> Also new since v0.7.27:
> - **Audit log persists** — Merkle hash chain at `~/.rustyhand/data/audit.jsonl`,
>   replayed and validated on boot.
> - **Triggers and workflows persist** — webhook triggers and pipeline
>   definitions survive daemon restart, including subtle state like
>   `max_fires` auto-disable and trigger fire-counts.
> - **31 API response-shape contract tests** — every endpoint the dashboard
>   or CLI reads is pinned to its JSON shape, so a server-side rename fails
>   CI loudly instead of producing a silently-empty widget.
> - **6 mock-driver e2e tests** — full HTTP → kernel → driver → result
>   pipeline runs in CI without burning real LLM credits, covering agent
>   message round-trip, workflow run, all three CronAction variants, and
>   the demo-mode auto-spawn flow.
> - **wasmtime 42 → 44** for [RUSTSEC-2026-0114](https://rustsec.org/advisories/RUSTSEC-2026-0114).

---

## Origin

This project is based on [OpenFang](https://github.com/RightNow-AI/openfang) by RightNow-AI, modified and extended for custom use cases.

## Table of Contents

- [What is RustyHand?](#what-is-rustyhand)
- [Installation](#installation)
- [Quick Start](#quick-start)
- [Telegram Setup](#telegram-setup)
- [Configuration](#configuration)
- [CLI Reference](#cli-reference)
- [Autonomous Templates](#autonomous-templates)
- [40 Pre-built Agent Templates](#40-pre-built-agent-templates)
- [Channel Adapters](#channel-adapters)
- [7 LLM Providers](#7-llm-providers)
- [Architecture](#architecture)
- [API Endpoints](#api-endpoints)
- [Dashboard](#dashboard)
- [Security](#security)
- [Deployment](#deployment)
- [Docker Environment Variables](#docker-environment-variables)
- [Development](#development)
- [Benchmarks](#benchmarks)
- [MCP Integration (for AI Agents)](#mcp-integration-for-ai-agents)
- [How It Works — Data Flow](#how-it-works--data-flow)
- [License](#license)

---

## What is RustyHand?

RustyHand is an **open-source Agent Operating System** — not a chatbot framework, not a Python wrapper around an LLM. It is a full operating system for autonomous agents, built from scratch in Rust.

Traditional agent frameworks wait for you to type something. RustyHand runs **autonomous agents that work for you** — on schedules, 24/7, building knowledge graphs, monitoring targets, generating leads, managing social media, and reporting results directly to your **Telegram chat**.

### Telegram-First Autonomous Agent

Telegram is the primary interface for RustyHand agents. Your agent can:

| Capability | How it works |
|------------|-------------|
| **See photos** | Auto-describes images via vision API |
| **Hear voice** | Auto-transcribes voice messages via Whisper |
| **Receive files** | Downloads documents, forwards to agent |
| **Send files/photos/voice** | Sends generated content back to chat |
| **Ask permission** | Inline keyboard buttons (Approve/Reject) pushed automatically |
| **Show progress** | Real-time tool-use updates: "⚙️ web_search..." → "✅ Done" |
| **Report autonomously** | Background tasks push results to your chat without prompting |
| **61 built-in tools** | Shell, web/news search, browser (wait, JS exec, scroll, download), RAG, knowledge graph |
| **Markdown formatting** | Bold, italic, code blocks render natively in Telegram |
| **Reply threading** | Responses reply to the user's message for clean conversation flow |
| **Sticker/GIF/Location** | Agent understands stickers, animations, and shared locations |

The entire system compiles to a **single ~32MB binary**. One install, one command, your agents are live.

---

## Installation

### One-liner (Linux / macOS / WSL)

```bash
curl -fsSL https://raw.githubusercontent.com/ginkida/rustyhand/main/scripts/install.sh | sh
```

Environment variables:
- `RUSTY_HAND_INSTALL_DIR` — custom install path (default: `~/.rustyhand/bin`)
- `RUSTY_HAND_VERSION` — pin a specific version tag

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/ginkida/rustyhand/main/scripts/install.ps1 | iex
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

Or run directly with env vars (no config.toml needed):

```bash
docker run -p 4200:4200 \
  -e ANTHROPIC_API_KEY=your-key \
  -e RUSTYHAND_API_KEY=my-secret-bearer-token \
  -v rustyhand-data:/data \
  ghcr.io/ginkida/rustyhand:latest
```

All configuration can be set via `RUSTYHAND_*` environment variables — see [Docker Environment Variables](#docker-environment-variables).

---

## Quick Start

### Option Zero: Try it in 30 seconds, no API key

The fastest possible first run, no credentials, no configuration:

```bash
git clone https://github.com/ginkida/rustyhand
cd rustyhand
cargo run --release -- start
# In another tab: open http://localhost:4200
```

The dashboard banner will read **"DEMO MODE — running on the deterministic
mock driver."** Spawn an agent, send a message, watch the agent loop run,
session grow, audit log fill up. Every reply is `[mock] <your message>` —
unmistakably demo, but the full pipeline (sessions, persistence, workflows,
cron jobs) is real. Set `ANTHROPIC_API_KEY` (or any of 26 other supported
providers' env vars) and restart for real LLM responses.

### Option A: Docker (fastest)

```bash
docker run -d --name rustyhand \
  -p 4200:4200 \
  -e ANTHROPIC_API_KEY=your-key \
  -v rustyhand-data:/data \
  ghcr.io/ginkida/rustyhand:latest

# Dashboard: http://localhost:4200
# API:       http://localhost:4200/api/health
```

To secure the API with a bearer token:

```bash
docker run -d --name rustyhand \
  -p 4200:4200 \
  -e ANTHROPIC_API_KEY=your-key \
  -e RUSTYHAND_API_KEY=my-secret-token \
  -v rustyhand-data:/data \
  ghcr.io/ginkida/rustyhand:latest

# Now all API calls require: -H "Authorization: Bearer my-secret-token"
```

See [Docker Environment Variables](#docker-environment-variables) for all options.

### Option B: From binary

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

# 5. Send a one-shot message
rustyhand message researcher "What are the emerging trends in AI agent frameworks?"

# 6. Launch the interactive TUI dashboard
rustyhand tui

# 7. Run diagnostics
rustyhand doctor
```

---

## Telegram Setup

Telegram is the **primary channel** for interacting with RustyHand agents. Setup takes 2 minutes:

### 1. Create a Telegram Bot

1. Message [@BotFather](https://t.me/BotFather) on Telegram
2. Send `/newbot`, follow prompts, get your bot token
3. Set the token: `export TELEGRAM_BOT_TOKEN=123456:ABC-DEF...`

### 2. Configure RustyHand

```toml
# ~/.rustyhand/config.toml
[channels.telegram]
bot_token_env = "TELEGRAM_BOT_TOKEN"
allowed_users = []    # Empty = allow anyone. Set [123456] for specific user IDs.
```

### 3. Start and chat

```bash
rustyhand start
# Open Telegram, message your bot
# /agents — list agents
# /agent assistant — select an agent
# Send text, photos, voice messages — the agent handles all of them
```

### What your agent can do in Telegram

```
You:     [send a voice message]
Agent:   [auto-transcribes via Whisper, processes your request]

You:     [send a photo]
Agent:   [auto-describes the image, responds based on what it sees]

You:     "Search for Rust 2024 edition changes"
Agent:   ⚙️ web_search...
         ✅ web_search
         Here are the key changes in Rust 2024...

Agent:   ⚠️ Agent "coder" wants to execute:
         `shell_exec: rm -rf /tmp/cache`
         [✅ Approve] [❌ Reject]    ⏱️ 60s

You:     [click ✅ Approve]
Agent:   Done! Cache cleared.
```

### Autonomous mode

Agents with `schedule_mode = "continuous"` or `"periodic"` run in the background and **push results to your Telegram chat automatically** — no prompting needed.

```toml
# agent.toml
[schedule]
mode = "periodic"
cron = "0 9 * * *"    # Every day at 9 AM
```

The agent wakes up, performs its task, and sends the result to the last Telegram chat it was used in.

---

## Configuration

RustyHand can be configured in two ways:
- **Config file** (`~/.rustyhand/config.toml`) — for binary installs
- **Environment variables** (`RUSTYHAND_*`) — for Docker, see [Docker Environment Variables](#docker-environment-variables)

### API Authentication

When `api_key` is set, all endpoints (except `/api/health`) require a Bearer token:

```bash
# In config.toml:
api_key = "my-secret-token"

# Or via env var (Docker):
RUSTYHAND_API_KEY=my-secret-token

# Clients must include the header:
curl -H "Authorization: Bearer my-secret-token" http://localhost:4200/api/agents
```

Without `api_key`, the API is open (fine for local development).

### Config file

Location: `~/.rustyhand/config.toml`

```toml
# API server settings
api_key = "your-bearer-token"          # Recommended for non-localhost access
api_listen = "127.0.0.1:4200"          # HTTP bind address

[default_model]
provider = "anthropic"                 # anthropic, kimi, deepseek, zhipu, minimax, openrouter, ollama
model = "claude-sonnet-4-20250514"     # Model identifier
api_key_env = "ANTHROPIC_API_KEY"      # Env var holding the API key
# base_url = "https://api.anthropic.com"  # Optional: override endpoint

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
# LLM providers — set ANY key and RustyHand auto-detects the provider.
# Priority order: Anthropic → Kimi → DeepSeek → Zhipu → MiniMax → OpenRouter.
ANTHROPIC_API_KEY=sk-ant-...       # Claude Opus / Sonnet / Haiku (default)
KIMI_API_KEY=sk-kimi-...            # Kimi Code — Anthropic-compat, 256K ctx
DEEPSEEK_API_KEY=sk-...             # DeepSeek V4 Flash / V4 Pro (V3/R1 legacy, deprecated 2026-07-24)
ZHIPU_API_KEY=...                   # Zhipu GLM-4.6
MINIMAX_API_KEY=eyJ...              # MiniMax M1 / M2.7 (1M context)
OPENROUTER_API_KEY=sk-or-...        # Universal gateway (GPT/Gemini/Grok/etc.)

# Local LLM — no key needed, just run `ollama serve`
# (Base URL defaults to http://localhost:11434/v1 — override only if needed)

# Embedding-only upstreams (independent of LLM provider)
VOYAGE_API_KEY=pa-...                # Voyage AI (voyage-3-lite, code, legal, ...)
# OPENAI_API_KEY can also be used for text-embedding-3-* — not for LLM completion.

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
rustyhand config set default_model.provider kimi   # Switch provider
rustyhand config set-key kimi                      # Interactively save API key
rustyhand config test-key kimi                     # Verify connectivity
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

## 40 Pre-built Agent Templates

Spawn any template with `rustyhand agent new <name>`:

| Template | Description |
|----------|-------------|
| `analyst` | Data analysis and reporting |
| `api-monitor` | API endpoint monitoring |
| `architect` | System design and architecture |
| `assistant` | General-purpose assistant |
| `capability-builder` | **Meta-agent** — writes new skills at runtime via privileged `skill_install` tool |
| `ci-monitor` | CI/CD pipeline monitoring |
| `code-reviewer` | Code review and feedback |
| `coder` | Software development |
| `coordinator` | **Meta-agent** — delegates work across other agents via `agent_send` |
| `customer-support` | Customer support |
| `dag-monitor` | DAG/workflow monitoring |
| `data-scientist` | Data science and ML |
| `db-reporter` | Database reporting |
| `debugger` | Bug investigation |
| `devops-lead` | DevOps and infrastructure |
| `diagnostic` | **Meta-agent** — read-only observability (self-history, metrics, audit log) |
| `doc-writer` | Documentation |
| `email-assistant` | Email drafting and management |
| `health-tracker` | Health and fitness tracking |
| `hello-world` | Starter agent for new users |
| `home-automation` | Smart home control |
| `legal-assistant` | Legal document review |
| `log-analyzer` | Log analysis and alerting |
| `meeting-assistant` | Meeting notes and follow-ups |
| `ops` | Operations management |
| `orchestrator` | Multi-agent orchestration |
| `personal-finance` | Financial tracking |
| `planner` | Project planning |
| `recruiter` | Recruiting and screening |
| `researcher` | Research and analysis |
| `sales-assistant` | Sales support |
| `security-auditor` | Security analysis |
| `slack-notifier` | Slack notification automation |
| `social-media` | Social media management |
| `test-engineer` | Testing and QA |
| `translator` | Multi-language translation |
| `travel-planner` | Travel planning |
| `tutor` | Education and tutoring |
| `weekly-digest` | Weekly summary reports |
| `writer` | Content writing |

### Agent manifest format (`agent.toml`)

```toml
name = "hello-world"
version = "0.1.0"
description = "A friendly greeting agent"
author = "rusty-hand"
module = "builtin:chat"

[model]
provider = "anthropic"
model = "claude-sonnet-4-20250514"
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

## Channel Adapters

RustyHand ships three messaging adapters — the ones whose APIs work without
a public webhook URL, which is what most users actually run:

- **Telegram** — long-polling Bot API (`@BotFather` token).
- **Discord** — Gateway WebSocket (Developer Portal bot token).
- **Slack** — Socket Mode (`xapp-` app token + `xoxb-` bot token).

Each adapter supports per-channel model overrides, DM/group policies, rate
limiting, and output formatting.

> v0.7.4 and earlier shipped 38 adapters (Matrix, WhatsApp, Signal, Teams,
> IRC, ...). They were dropped in v0.7.5 — most were webhook-only and broken
> in typical localhost / home-Docker setups, and many were sprint fillers
> without real usage. Pin to v0.7.4 if you need them, or open an issue and
> we'll discuss a route.

### Channel policies

Configure each channel under the `[channels.*]` table in `config.toml`:

```toml
[channels.telegram]
bot_token_env = "TELEGRAM_BOT_TOKEN"
allowed_users = [123456789]            # Restrict to specific users
default_agent = "assistant"            # Route inbound messages here

[channels.telegram.overrides]
dm_policy = "Respond"                  # Respond | AllowedOnly | Ignore
group_policy = "MentionOnly"           # All | MentionOnly | CommandsOnly | Ignore
output_format = "TelegramHtml"         # Markdown | TelegramHtml | SlackMrkdwn | PlainText
```

### Zero-config Telegram on Docker

Since v0.7.10, the Docker entrypoint generates `default_agent = "assistant"`
under each `[channels.*]` section automatically (override with the
`RUSTYHAND_{TELEGRAM,DISCORD,SLACK}_DEFAULT_AGENT` env var, or set it to
an empty string to leave it blank). The bundled `assistant` manifest uses
`provider = "anthropic"`, so a fresh container with `ANTHROPIC_API_KEY`
plus a bot token replies to the first message without any extra config or
`rustyhand init` step.

If the router can't resolve a target at message time (no
`default_agent`, no bindings, no direct routes), the bridge tries to
auto-route to a running agent first, then to spawn one of the bundled
meta-agents (`assistant` → `coordinator` → `coder`). The user only sees
a config-pointing error message when every fallback fails.

---

## 7 LLM Providers

RustyHand v0.7.0 ships with a deliberately lean set of 7 providers, driven by 2 wire protocols (Anthropic Messages API + OpenAI-compatible Chat Completions). Anthropic and **Kimi Code** are the two first-class coding providers:

| Provider | Env var | Role |
|---|---|---|
| **Anthropic** (default) | `ANTHROPIC_API_KEY` | Claude Opus/Sonnet/Haiku — best-in-class tool use + extended thinking |
| **Kimi (Moonshot)** | `KIMI_API_KEY` | Kimi Code — Anthropic-compatible, 256K ctx, vision, reasoning |
| **DeepSeek** | `DEEPSEEK_API_KEY` | V4 Flash + V4 Pro — fast & reasoning (V3/R1 deprecated 2026-07-24). Also exposes an Anthropic-compatible endpoint at `/anthropic` |
| **Zhipu GLM** | `ZHIPU_API_KEY` | GLM-4.6 — Chinese frontier |
| **MiniMax** | `MINIMAX_API_KEY` | M1/M2 — 1M context |
| **OpenRouter** | `OPENROUTER_API_KEY` | Universal gateway — any model via one key |
| **Ollama** | (no key) | Local on `localhost:11434` |

Default auto-detect order: Anthropic → Kimi → DeepSeek → Zhipu → MiniMax → OpenRouter. Set whichever key you have; RustyHand picks the first one found.

Prefer Kimi? Set `KIMI_API_KEY` and auto-detect will route to `kimi-for-coding` on the Kimi Code endpoint (`api.kimi.com/coding`). Change anytime via `rustyhand config set default_model.provider <name>`.

> v0.6.x shipped 27 providers (OpenAI, Gemini, Groq, xAI, Copilot, Mistral, Together, Fireworks, Perplexity, Cohere, AI21, Cerebras, SambaNova, HuggingFace, Replicate, vLLM, LM Studio, Moonshot, Qwen, Qianfan, Bedrock). They were removed in v0.7.0 — use `openrouter` to reach any of those models through one gateway.

Features:
- Intelligent routing with task complexity scoring
- Automatic fallback between providers
- Per-model pricing and cost tracking
- Per-agent budget limits

### Embedding providers

Vector embeddings power semantic memory recall. The catalog is independent of
the LLM provider list — `OPENAI_API_KEY` is still usable for text-embedding-3-*
even though OpenAI is not a first-class LLM provider in v0.7.0.

Auto-detected at boot (first available wins):

| Provider | Models | Key required |
|----------|--------|-------------|
| **Voyage AI** | voyage-3, voyage-3-lite, voyage-code-3, voyage-finance-2, voyage-law-2, voyage-multilingual-2 | `VOYAGE_API_KEY` |
| **OpenAI** (embedding-only) | text-embedding-3-small, text-embedding-3-large, text-embedding-ada-002 | `OPENAI_API_KEY` |
| **Ollama** | nomic-embed-text, all-MiniLM-L6-v2, mxbai-embed-large | No |
| **Any OpenAI-compat endpoint** | whatever the server exposes | provider-specific |

Configure explicitly in `config.toml`:

```toml
[memory]
embedding_provider = "voyage"              # or "openai", "ollama", "<custom>"
embedding_api_key_env = "VOYAGE_API_KEY"
```

Or let RustyHand auto-detect: it probes Voyage → OpenAI → Ollama at boot and uses the first available provider. Falls back to text search (SQLite LIKE) when no embedding driver is found.

```bash
rustyhand models list                 # Browse all models
rustyhand models list --provider kimi # Filter by provider
rustyhand models set claude-sonnet    # Set default model
```

---

## Architecture

10 Rust crates with a modular kernel design:

```
rusty-hand-types       Core types, traits, config, taint tracking, Ed25519 manifest signing
    |
    +-- rusty-hand-memory      SQLite persistence, vector embeddings (Voyage/OpenAI/Ollama), session compaction
    +-- rusty-hand-wire        RHP P2P protocol (JSON-RPC over TCP, HMAC-SHA256 auth)
    +-- rusty-hand-channels    Telegram + Discord + Slack adapters with rate limiting
    +-- rusty-hand-skills      Skill system + ClawHub marketplace
    +-- rusty-hand-extensions  25 MCP integrations, AES-256-GCM credential vault, OAuth2
    |
    +-- rusty-hand-runtime     Agent loop, 2 LLM drivers (Anthropic + OpenAI-compat), 53+ tools, WASM sandbox, MCP, A2A
    |
    +-- rusty-hand-kernel      Orchestration: lifecycle, scheduling, metering, RBAC, workflows
    |
    +-- rusty-hand-api         Axum HTTP daemon, 120+ endpoints, WebSocket, SSE, OpenAI-compat
    |
    +-- rusty-hand-cli         CLI binary + TUI dashboard (ratatui)
```

### Key internals

| Concept | Implementation |
|---------|---------------|
| **Agent loop** | `rusty-hand-runtime` — iterative LLM call → tool execution → response cycle |
| **Kernel** | `RustyHandKernel` struct (40+ fields) — central orchestration for all subsystems |
| **AppState** | Bridges kernel to HTTP routes via `Arc<RustyHandKernel>` in Axum state |
| **Sandbox** | WASM (wasmtime) with fuel metering + epoch interruption + watchdog thread |
| **Memory** | SQLite + vector embeddings (Voyage AI, OpenAI, Ollama) for semantic search + knowledge graph |
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

Full REST/WS/SSE endpoints cover agents, memory, workflows, channels, models, skills, sessions, approvals, triggers, crons, security, and more (120+ total).

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

### Docker Environment Variables

The Docker entrypoint generates `config.toml` from environment variables automatically — no config file needed. If you mount your own `config.toml`, env vars are ignored.

Set `RUSTYHAND_FORCE_ENV_CONFIG=1` to always regenerate config from env vars (overrides mounted file).

#### Core

| Env var | Default | Description |
|---------|---------|-------------|
| `RUSTYHAND_API_KEY` | *(none)* | **Bearer auth token.** When set, all API endpoints require `Authorization: Bearer <token>` header. Strongly recommended for non-local access. |
| `RUSTYHAND_API_LISTEN` | `0.0.0.0:4200` | HTTP bind address |
| `RUSTYHAND_LOG_LEVEL` | `info` | Log level: `trace`, `debug`, `info`, `warn`, `error` |

#### LLM Provider

| Env var | Default | Description |
|---------|---------|-------------|
| `RUSTYHAND_PROVIDER` | `anthropic` | LLM provider: `anthropic`, `kimi`, `deepseek`, `zhipu`, `minimax`, `openrouter`, `ollama` |
| `RUSTYHAND_MODEL` | `claude-sonnet-4-20250514` | Model identifier |
| `RUSTYHAND_MODEL_KEY_ENV` | `ANTHROPIC_API_KEY` | Which env var holds the LLM API key |
| `RUSTYHAND_MODEL_BASE_URL` | *(auto)* | Override provider API endpoint |
| `RUSTYHAND_FALLBACK_PROVIDER` | *(none)* | Fallback provider if primary fails |
| `RUSTYHAND_FALLBACK_MODEL` | *(none)* | Fallback model |
| `RUSTYHAND_FALLBACK_KEY_ENV` | *(none)* | Env var for fallback API key |

#### LLM API Keys (pass through to agents)

| Env var | Provider |
|---------|----------|
| `ANTHROPIC_API_KEY` | Anthropic Claude (default) |
| `KIMI_API_KEY` | Kimi Code (Moonshot) |
| `DEEPSEEK_API_KEY` | DeepSeek V4 Flash / V4 Pro (V3/R1 legacy, deprecated 2026-07-24) |
| `ZHIPU_API_KEY` | Zhipu GLM-4.6 |
| `MINIMAX_API_KEY` | MiniMax M1 / M2 |
| `OPENROUTER_API_KEY` | OpenRouter gateway (any upstream model) |

#### Budget

| Env var | Default | Description |
|---------|---------|-------------|
| `RUSTYHAND_BUDGET_HOURLY` | `0.0` | Max spend per hour in USD (0 = unlimited) |
| `RUSTYHAND_BUDGET_DAILY` | `0.0` | Max spend per day in USD |
| `RUSTYHAND_BUDGET_MONTHLY` | `0.0` | Max spend per month in USD |

#### Memory & Embeddings

| Env var | Default | Description |
|---------|---------|-------------|
| `RUSTYHAND_MEMORY_DECAY` | `0.05` | Memory confidence decay rate |
| `RUSTYHAND_EMBEDDING_PROVIDER` | *(auto)* | Embedding provider: `voyage`, `openai`, `ollama` |
| `RUSTYHAND_EMBEDDING_KEY_ENV` | *(auto)* | Env var for embedding API key |
| `VOYAGE_API_KEY` | *(none)* | Voyage AI embeddings key |

#### Channels

| Env var | Description |
|---------|-------------|
| `TELEGRAM_BOT_TOKEN` | Telegram bot — auto-enables `[telegram]` section |
| `DISCORD_BOT_TOKEN` | Discord bot — auto-enables `[discord]` section |
| `SLACK_BOT_TOKEN` | Slack bot — auto-enables `[slack]` section |
| `SLACK_APP_TOKEN` | Slack app-level token (for Socket Mode) |
| `RUSTYHAND_TELEGRAM_USERS` | Comma-separated Telegram user IDs to allow (e.g. `123456789,987654321`). Brackets and quotes tolerated. Negative IDs (channels) supported. Unset = open to ANY Telegram user — **strongly recommended to set**. |

#### Other

| Env var | Description |
|---------|-------------|
| `RUSTYHAND_EXEC_MODE` | Shell exec policy: `deny`, `allowlist`, `full` |
| `RUSTYHAND_A2A_ENABLED` | Enable A2A protocol: `true` / `1` |
| `RUSTYHAND_USAGE_FOOTER` | Response footer: `Off`, `Tokens`, `Cost`, `Full` |
| `RUSTYHAND_FORCE_ENV_CONFIG` | Set to `1` to always regenerate config from env vars |

#### Example: full Docker run

```bash
docker run -d --name rustyhand \
  -p 4200:4200 \
  -e RUSTYHAND_API_KEY=my-secret-token \
  -e RUSTYHAND_PROVIDER=anthropic \
  -e RUSTYHAND_MODEL=claude-sonnet-4-20250514 \
  -e RUSTYHAND_MODEL_KEY_ENV=ANTHROPIC_API_KEY \
  -e ANTHROPIC_API_KEY=sk-ant-... \
  -e RUSTYHAND_BUDGET_DAILY=5.0 \
  -e TELEGRAM_BOT_TOKEN=123456:ABC... \
  -v rustyhand-data:/data \
  ghcr.io/ginkida/rustyhand:latest
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

# Run all tests (1,481 as of v0.7.10)
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
  agents/                   # 37 pre-built agent templates (agent.toml each)
  deploy/                   # systemd service, Docker scripts
  scripts/                  # install.sh, install.ps1
  crates/
    rusty-hand-types/       # Core types (config.rs is the master config struct)
    rusty-hand-memory/      # SQLite + vector embeddings (Voyage AI, OpenAI, Ollama)
    rusty-hand-runtime/     # Agent loop + LLM drivers + tools + sandbox
    rusty-hand-wire/        # RHP P2P protocol
    rusty-hand-api/         # Axum HTTP server + routes + dashboard
      src/
        server.rs           # Router setup, middleware, AppState
        routes.rs           # All API endpoint handlers (~7600 LOC)
      static/
        index_body.html     # Dashboard SPA (Alpine.js)
        index_head.html     # CSS + fonts
    rusty-hand-kernel/      # Central kernel (~5300 LOC, 40+ fields)
    rusty-hand-cli/         # CLI + TUI binary
    rusty-hand-channels/    # Telegram + Discord + Slack adapters
    rusty-hand-skills/      # Skill system + ClawHub + OpenClaw compat
    rusty-hand-extensions/  # MCP + vault + OAuth2
```

### Key files for contributors

| File | What it does |
|------|-------------|
| `crates/rusty-hand-kernel/src/kernel.rs` | The kernel — 40+ fields, central orchestration |
| `crates/rusty-hand-api/src/routes.rs` | All API handlers (~7600 LOC) |
| `crates/rusty-hand-api/src/server.rs` | Router, middleware, `AppState` struct |
| `crates/rusty-hand-types/src/config.rs` | Master config struct (`KernelConfig`) |
| `crates/rusty-hand-api/static/index_body.html` | Dashboard SPA |
| `crates/rusty-hand-api/src/channel_bridge.rs` | Channel adapter wiring |
| `crates/rusty-hand-runtime/src/drivers/` | LLM drivers (anthropic.rs for Anthropic + Kimi; openai.rs for DeepSeek/Zhipu/MiniMax/OpenRouter/Ollama) |

### Common gotchas

- `rustyhand.exe` may be locked if the daemon is running — use `--lib` flag or kill daemon first
- New config fields need: struct field + `#[serde(default)]` + `Default` impl entry
- New routes must be registered in `server.rs` router AND implemented in `routes.rs`
- Dashboard tabs need both HTML in `index_body.html` and JS data/methods
- `AgentLoopResult` field is `.response` not `.response_text`
- CLI daemon command is `start` (not `daemon`)

---

## Benchmarks

All data from official documentation and public repositories — April 2026.

| Metric | RustyHand | ZeroClaw | LangGraph | CrewAI | AutoGen | OpenClaw |
|--------|----------|----------|-----------|--------|---------|----------|
| **Cold start** | 180 ms | 10 ms | 2.5 s | 3.0 s | 4.0 s | 5.98 s |
| **Idle memory** | 40 MB | 5 MB | 180 MB | 200 MB | 250 MB | 394 MB |
| **Install size** | 32 MB | 8.8 MB | 150 MB | 100 MB | 200 MB | 500 MB |
| **Security layers** | 16 | 6 | 2 | 1 | 2 | 3 |
| **Channel adapters** | 3 | 15 | 0 | 0 | 0 | 13 |
| **LLM providers** | 7 (+ OpenRouter gateway) | 28 | 15 | 10 | 8 | 10 |
| **Language** | Rust | Rust | Python | Python | Python | TypeScript |

---

## MCP Integration (for AI Agents)

RustyHand exposes itself as an **MCP server** over stdio, giving any MCP-compatible AI agent (Claude Desktop, Cursor, Windsurf, Claude Code, etc.) full control over the agent OS.

### Setup

Add to your MCP client config (e.g. `claude_desktop_config.json`):

```json
{
  "mcpServers": {
    "rustyhand": {
      "command": "rustyhand",
      "args": ["mcp"]
    }
  }
}
```

That's it. The AI agent now has 30+ tools to manage the entire system.

### Available MCP Tools

| Tool | What it does |
|------|-------------|
| **System** | |
| `rustyhand_system_health` | Check daemon health and DB connectivity |
| `rustyhand_system_status` | Uptime, agent count, default provider/model |
| `rustyhand_config_get` | Read current config (secrets redacted) |
| `rustyhand_config_set` | Set config field by dotted path (e.g. `default_model.provider`) |
| `rustyhand_config_reload` | Hot-reload config from `~/.rustyhand/config.toml` |
| **Agents** | |
| `rustyhand_agent_list` | List all agents (ID, name, state, model) |
| `rustyhand_agent_get` | Full agent details by ID |
| `rustyhand_agent_spawn` | Spawn new agent from TOML manifest |
| `rustyhand_agent_kill` | Stop and remove agent |
| `rustyhand_agent_message` | Send message, get LLM-powered response |
| `rustyhand_agent_session` | Get conversation history |
| `rustyhand_agent_set_model` | Change agent's LLM model at runtime |
| `rustyhand_agent_session_reset` | Clear conversation history |
| **Models & Providers** | |
| `rustyhand_provider_list` | All 7 providers with auth status |
| `rustyhand_model_list` | Available models (tier, context window, cost) |
| **Budget** | |
| `rustyhand_budget_status` | Global spend vs limits (hourly/daily/monthly) |
| `rustyhand_budget_agents` | Per-agent cost ranking |
| **Workflows** | |
| `rustyhand_workflow_list` | List workflow definitions |
| `rustyhand_workflow_run` | Execute workflow with input |
| `rustyhand_workflow_runs` | List past workflow runs |
| **Cron** | |
| `rustyhand_cron_list` | List all scheduled jobs |
| `rustyhand_cron_create` | Schedule recurring agent task (cron expression) |
| `rustyhand_cron_delete` | Remove scheduled job |
| **Approvals** | |
| `rustyhand_approval_list` | List pending/approved/rejected requests |
| `rustyhand_approval_approve` | Approve a pending execution |
| `rustyhand_approval_reject` | Reject a pending execution |
| **Other** | |
| `rustyhand_channel_list` | Channel adapters and their status |
| `rustyhand_template_list` | Available agent templates |
| `rustyhand_tool_list` | All built-in tools available to agents |

Plus: every running agent automatically becomes an MCP tool named `rusty_hand_agent_{name}`, so the AI agent can delegate tasks to specialized RustyHand agents.

### Example: AI agent workflow via MCP

An AI agent (Claude, GPT, etc.) can autonomously:

1. `rustyhand_agent_spawn` — create a researcher agent with web access
2. `rustyhand_agent_message` — ask it to research a topic
3. `rustyhand_agent_spawn` — create a writer agent
4. `rustyhand_agent_message` — pass research results to the writer
5. `rustyhand_cron_create` — schedule the researcher to update weekly
6. `rustyhand_budget_status` — check how much it all cost

### Why an AI agent should use RustyHand

- **Delegate work** — spawn specialized agents instead of doing everything yourself
- **Persistent memory** — agents remember context across sessions (vector search + knowledge graph)
- **3 channels** — reach users on Telegram, Discord, and Slack without building integrations
- **Budget control** — set spending limits so agents can't run up costs
- **7 LLM providers** — Anthropic, Kimi, DeepSeek, Zhipu, MiniMax, OpenRouter, Ollama — plus OpenRouter gateway to any other model
- **Autonomous scheduling** — cron jobs run agents on schedule, no human needed
- **60 bundled skills** — instant expertise in Kubernetes, AWS, PostgreSQL, Git, Python, etc.
- **Approval gates** — dangerous actions require human approval before executing
- **Audit trail** — every action is logged in a Merkle hash chain

## REST API for Programmatic Access

When `api_key` is configured, add `-H "Authorization: Bearer <token>"` to all requests (except `/api/health`).

```bash
# Health check (always public)
curl http://localhost:4200/api/health

# List agents
curl http://localhost:4200/api/agents

# Spawn an agent
curl -X POST http://localhost:4200/api/agents \
  -H "Content-Type: application/json" \
  -d '{"manifest_toml": "name = \"my-agent\"\nmodule = \"builtin:chat\"\n[model]\nprovider = \"kimi\"\nmodel = \"kimi-for-coding\"\napi_key_env = \"KIMI_API_KEY\"\nsystem_prompt = \"You are a helpful assistant.\""}'

# Send a message (triggers LLM call, returns full response)
curl -X POST http://localhost:4200/api/agents/{id}/message \
  -H "Content-Type: application/json" \
  -d '{"message": "Hello, what can you do?"}'

# Stream a response (SSE)
curl -N -X POST http://localhost:4200/api/agents/{id}/message/stream \
  -H "Content-Type: application/json" \
  -d '{"message": "Write a haiku about Rust"}'

# OpenAI-compatible endpoint (drop-in replacement for any OpenAI client)
curl -X POST http://localhost:4200/v1/chat/completions \
  -H "Content-Type: application/json" \
  -d '{"model": "coder", "messages": [{"role": "user", "content": "Fix this bug"}]}'

# Budget status
curl http://localhost:4200/api/budget

# Memory — store and recall
curl -X PUT http://localhost:4200/api/memory/agents/{id}/kv/project_name \
  -H "Content-Type: application/json" \
  -d '{"value": "rustyhand"}'
curl http://localhost:4200/api/memory/agents/{id}/kv/project_name

# With auth enabled:
curl -H "Authorization: Bearer my-secret-token" http://localhost:4200/api/agents
```

SDKs for Python and JavaScript are included in `sdk/python/` and `sdk/javascript/`.

---

## How It Works — Data Flow

```
User message (CLI / API / Telegram / Discord / ...)
    |
    v
[Channel Adapter] --- converts platform message to unified ChannelMessage
    |
    v
[Kernel Router] --- resolves target agent via bindings/broadcast rules
    |
    v
[Agent Registry] --- looks up AgentManifest + Session
    |
    v
[Agent Loop] (rusty-hand-runtime/src/agent_loop.rs)
    |
    +-- 1. Recall memories (vector similarity via Voyage/OpenAI/Ollama, or text LIKE)
    +-- 2. Build system prompt (SOUL.md + USER.md + TOOLS.md + MEMORY.md + recalled context)
    +-- 3. Call LLM (driver: Anthropic or OpenAI-compat)
    |       |-- retry on rate limit (3x, exponential backoff)
    |       |-- fallback to next provider on failure
    |       |-- model routing by complexity (simple/medium/complex)
    +-- 4. If tool_use → execute tool → append result → goto 3 (max 50 iterations)
    |       |-- built-in: file_read, file_write, shell_exec, web_search, web_fetch,
    |       |             memory_store, memory_recall, agent_send, agent_spawn, browser_*
    |       |-- MCP tools: GitHub, Notion, Slack, PostgreSQL, ... (25+ integrations)
    |       |-- skills: 60 prompt-only + Python/WASM/Node.js executable skills
    +-- 5. Extract response text + reply directives
    |
    v
[Metering] --- record token usage + cost, check budget limits
    |
    v
[Session Save] --- persist messages to SQLite, append daily memory log
    |
    v
[Channel Adapter] --- format response for platform, send back
    |
    v
User receives response
```

### Key Types

| Type | File | Purpose |
|------|------|---------|
| `KernelConfig` | `crates/rusty-hand-types/src/config.rs` | Master config (50+ fields, all with `#[serde(default)]`) |
| `AgentManifest` | `crates/rusty-hand-types/src/agent.rs` | Agent definition (model, tools, capabilities, resources) |
| `RustyHandKernel` | `crates/rusty-hand-kernel/src/kernel.rs` | Central orchestrator (40+ subsystem fields) |
| `LlmDriver` | `crates/rusty-hand-runtime/src/llm_driver.rs` | Trait: `complete()` + `complete_stream()` |
| `KernelHandle` | `crates/rusty-hand-runtime/src/kernel_handle.rs` | Trait: inter-agent ops (spawn, send, kill, memory, tasks) |
| `AppState` | `crates/rusty-hand-api/src/routes.rs` | Axum state: `Arc<RustyHandKernel>` + bridge manager |
| `AgentLoopResult` | `crates/rusty-hand-runtime/src/agent_loop.rs` | Result: `.response`, `.total_usage`, `.cost_usd`, `.silent` |
| `MemorySubstrate` | `crates/rusty-hand-memory/src/substrate.rs` | Unified memory API (structured + semantic + knowledge graph) |
| `Event` | `crates/rusty-hand-types/src/event.rs` | Event bus payload (Message, ToolResult, Lifecycle, System) |
| `ToolDefinition` | `crates/rusty-hand-types/src/tool.rs` | Tool schema for LLM (name, description, JSON Schema input) |

### Extending RustyHand

**Add a new LLM provider:**
1. Add base URL constant to `crates/rusty-hand-types/src/model_catalog.rs`
2. Add match arm to `crates/rusty-hand-runtime/src/drivers/mod.rs` `provider_defaults()`
3. Most providers use the OpenAI-compatible driver — no new driver code needed

**Add a new API endpoint:**
1. Add handler function in `crates/rusty-hand-api/src/routes.rs`
2. Register route in `crates/rusty-hand-api/src/server.rs` `build_router()`
3. Add request/response types in `crates/rusty-hand-api/src/types.rs` if needed

**Add a new config field:**
1. Add field with `#[serde(default)]` to struct in `crates/rusty-hand-types/src/config.rs`
2. Add default value to the `Default` impl
3. Add to custom `Debug` impl (redact secrets)

**Add a new built-in tool:**
1. Add `ToolDefinition` to `builtin_tool_definitions()` in `crates/rusty-hand-runtime/src/tool_runner.rs`
2. Add execution handler in the same file's `execute_tool()` match

**Add a new channel adapter:**
1. Create `crates/rusty-hand-channels/src/<name>.rs`
2. Add `pub mod <name>` to `crates/rusty-hand-channels/src/lib.rs`
3. Wire into `crates/rusty-hand-api/src/channel_bridge.rs`

---

## License

MIT — use it however you want.

---

## Links

- [GitHub](https://github.com/ginkida/rustyhand)
- [Quick Start](https://github.com/ginkida/rustyhand#quick-start)
- [API Reference](https://github.com/ginkida/rustyhand#api-endpoints)
- [Python SDK](https://github.com/ginkida/rustyhand/tree/main/sdk/python)
- [JavaScript SDK](https://github.com/ginkida/rustyhand/tree/main/sdk/javascript)

---

## Acknowledgments

RustyHand is a fork of [OpenFang](https://github.com/RightNow-AI/openfang), originally built by [Jaber](https://x.com/Akashi203) at [RightNow](https://www.rightnowai.co/).

---

<p align="center">
  <strong>Built with Rust. Secured with 16 layers. Agents that actually work for you.</strong>
</p>
