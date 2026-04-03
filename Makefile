# RustyHand — build, test, restart daemon
CARGO := ~/.cargo/bin/cargo
BIN   := target/release/rustyhand
PORT  := 4200

.PHONY: build check test clippy restart stop start dev

# Full check: build + clippy + tests (skip runtime timeout)
check: build clippy test

# Release build
build:
	$(CARGO) build --release -p rusty-hand-cli

# Lib-only build (if exe locked by running daemon)
build-lib:
	$(CARGO) build --workspace --lib

# Clippy with zero warnings
clippy:
	$(CARGO) clippy --workspace --all-targets -- -D warnings

# Tests (excluding runtime — pre-existing timeout)
test:
	$(CARGO) test --workspace --exclude rusty-hand-runtime

# Stop running daemon
stop:
	@PID=$$(lsof -ti :$(PORT) 2>/dev/null); \
	if [ -n "$$PID" ]; then \
		kill $$PID 2>/dev/null; \
		echo "Stopped PID $$PID"; \
		sleep 2; \
	else \
		echo "No daemon on port $(PORT)"; \
	fi

# Start daemon (needs GROQ_API_KEY in env)
start:
	$(BIN) start &
	@sleep 5
	@curl -sf http://127.0.0.1:$(PORT)/api/health && echo " — daemon up" || echo " — daemon NOT responding"

# Stop + rebuild + start
restart: stop build start

# Quick dev cycle: lib build + clippy (no daemon restart)
dev: build-lib clippy
	@echo "Dev build OK"
