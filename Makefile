.PHONY: build test run clean docker-build docker-run install help

BINARY  := target/release/tokemon
IMAGE   := tokemon
VERSION := $(shell grep '^version' Cargo.toml | head -1 | cut -d'"' -f2)

help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | \
		awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-16s\033[0m %s\n", $$1, $$2}'

build: ## Build release binary
	cargo build --release

test: ## Run all tests
	cargo test

install: build ## Install to ~/.cargo/bin
	cp $(BINARY) $(HOME)/.cargo/bin/tokemon

clean: ## Remove build artifacts
	cargo clean

docker-build: ## Build Docker image (multi-stage, produces minimal runtime image)
	docker build -t $(IMAGE):$(VERSION) -t $(IMAGE):latest .

docker-run: ## Run via Docker with provider data mounted (pass ARGS="...")
	@docker run --rm \
		-v $(HOME)/.claude:/root/.claude:ro \
		-v $(HOME)/.cache/tokemon:/root/.cache/tokemon:ro \
		$(IMAGE):latest $(ARGS)

lint: ## Run clippy lints
	cargo clippy -- -W clippy::pedantic -A clippy::module_name_repetitions

fmt: ## Format code
	cargo fmt

fmt-check: ## Check formatting
	cargo fmt -- --check

ci: fmt-check lint test ## Run all CI checks
