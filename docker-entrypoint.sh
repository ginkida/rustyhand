#!/bin/sh
set -e

DATA_DIR="${RUSTY_HAND_HOME:-/data}"

# ── 1. Check permissions on data directory ──────────────────────────
if [ ! -w "$DATA_DIR" ]; then
    echo "ERROR: $DATA_DIR is not writable by uid $(id -u) ($(whoami))."
    OWNER_UID=$(stat -c '%u' "$DATA_DIR" 2>/dev/null || stat -f '%u' "$DATA_DIR" 2>/dev/null || echo '?')
    echo "  Directory is owned by uid $OWNER_UID."
    echo "  Fix: run 'chown $(id -u):$(id -g) $DATA_DIR' on the host,"
    echo "  or start the container with '--user $OWNER_UID'."
    exit 1
fi

# ── 2. Copy default config if missing ───────────────────────────────
if [ ! -f "$DATA_DIR/config.toml" ]; then
    cat > "$DATA_DIR/config.toml" <<'TOML'
# RustyHand — Docker default config
# Override by mounting your own config.toml to /data/config.toml

api_listen = "0.0.0.0:4200"

[default_model]
provider = "minimax"
model = "MiniMax-M2.7"
api_key_env = "MINIMAX_API_KEY"

[memory]
decay_rate = 0.05
TOML
    echo "Created default config at $DATA_DIR/config.toml"
fi

# ── 3. Check that at least one LLM API key is set ──────────────────
HAS_KEY=0
for VAR in MINIMAX_API_KEY GROQ_API_KEY ANTHROPIC_API_KEY OPENAI_API_KEY \
           GEMINI_API_KEY DEEPSEEK_API_KEY OPENROUTER_API_KEY TOGETHER_API_KEY \
           MISTRAL_API_KEY FIREWORKS_API_KEY; do
    eval "val=\$$VAR"
    if [ -n "$val" ]; then
        HAS_KEY=1
        break
    fi
done

if [ "$HAS_KEY" = "0" ]; then
    echo "WARNING: No LLM API key found. Set at least one of:"
    echo "  MINIMAX_API_KEY, GROQ_API_KEY, ANTHROPIC_API_KEY, OPENAI_API_KEY,"
    echo "  GEMINI_API_KEY, DEEPSEEK_API_KEY, OPENROUTER_API_KEY"
    echo "  (agents will fail to respond without a configured provider)"
fi

# ── 4. Exec into rustyhand ──────────────────────────────────────────
exec rustyhand "$@"
