.PHONY: build test fmt fmt-check lint deny clean deploy-local deploy-testnet

build:
	stellar contract build

test:
	cargo build -p paystream-stream --target wasm32v1-none --release
	cargo test

fmt:
	cargo fmt

fmt-check:
	cargo fmt --check

lint:
	cargo clippy --all-targets -- -D warnings

check:
	cargo check --all

deny:
	cargo deny check

clean:
	cargo clean

deploy-local:
	./scripts/deploy-local.sh

deploy-testnet:
	./scripts/deploy-testnet.sh
