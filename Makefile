.PHONY: build test integration-test fmt fmt-check lint deny clean deploy-local deploy-testnet

build:
	stellar contract build

test:
	cargo build -p paystream-stream --target wasm32v1-none --release
	cargo test

integration-test:
	docker run -d --rm --name paystream-sandbox -p 8000:8000 stellar/quickstart:latest --local --enable-soroban-rpc
	./tests/integration/run.sh; status=$$?; docker stop paystream-sandbox >/dev/null; exit $$status

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
