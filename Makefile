.PHONY: build test clean install help lint fmt fmt-check ci

BINARY  := target/release/tokemon

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

lint: ## Run clippy lints
	cargo clippy -- -W clippy::pedantic -A clippy::module_name_repetitions

fmt: ## Format code
	cargo fmt

fmt-check: ## Check formatting
	cargo fmt -- --check

ci: fmt-check lint test ## Run all CI checks
