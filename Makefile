.PHONY: format
format:
	cargo fmt --all -- --check

.PHONY: lint
lint:
	cargo clippy --all-targets --all-features --fix --allow-dirty -- -D warnings
