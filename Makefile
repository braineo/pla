.PHONY: format
format:
	cargo fmt --all

.PHONY: lint
lint:
	cargo clippy --all-targets --all-features --fix --allow-dirty -- -D warnings

.PHONY: test
test:
	cargo test
