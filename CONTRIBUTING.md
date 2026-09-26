# Contributing to PayStream

Thank you for contributing to PayStream — a Soroban smart contract system for real-time salary streaming on Stellar. This guide covers everything you need to go from zero to a merged PR.

---

## Table of Contents

- [Development Setup](#development-setup)
  - [macOS](#macos)
  - [Linux](#linux)
  - [Windows](#windows)
  - [Docker (any OS)](#docker-any-os)
- [Project Structure](#project-structure)
- [Coding Standards](#coding-standards)
- [Commit Conventions](#commit-conventions)
- [Bounty Program & Finding Issues](#bounty-program--finding-issues)
- [Pull Request Process](#pull-request-process)
- [Testing Requirements](#testing-requirements)
- [Code Review Expectations](#code-review-expectations)
- [Glossary](#glossary)
- [License](#license)

---

## Development Setup

### Prerequisites (all platforms)

| Tool | Version | Purpose |
|---|---|---|
| Rust | stable (see `rust-toolchain.toml`) | Contract compilation |
| `wasm32-unknown-unknown` target | bundled via toolchain file | WASM output |
| Stellar CLI | 22.0.0 | Build, deploy, invoke |
| Docker + Compose | any recent | Optional — zero-install alternative |

The `rust-toolchain.toml` at the repo root pins the exact Rust channel and installs `rustfmt` and `clippy` automatically when you run any `cargo` command.

---

### macOS

```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 2. Install Stellar CLI
cargo install --locked stellar-cli --version 22.0.0

# 3. Clone and bootstrap
git clone https://github.com/veracindarella/paystream-contracts.git
cd paystream-contracts
# rust-toolchain.toml handles the target and components automatically

# 4. Verify
make test
```

> Homebrew users can also install Rust via `brew install rust`, but `rustup` is preferred because it respects `rust-toolchain.toml`.

---

### Linux

```bash
# 1. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 2. Install build dependencies (Debian/Ubuntu)
sudo apt-get update && sudo apt-get install -y build-essential pkg-config libssl-dev

# 3. Install Stellar CLI
cargo install --locked stellar-cli --version 22.0.0

# 4. Clone and bootstrap
git clone https://github.com/veracindarella/paystream-contracts.git
cd paystream-contracts

# 5. Verify
make test
```

For Fedora/RHEL replace step 2 with:
```bash
sudo dnf install gcc openssl-devel
```

---

### Windows

Native Windows development is supported via WSL 2. Running inside WSL gives you a full Linux environment and avoids shell-script compatibility issues.

```powershell
# 1. Enable WSL 2 (run in PowerShell as Administrator)
wsl --install
# Restart when prompted, then open a WSL terminal

# Inside WSL:
# 2. Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 3. Install build dependencies
sudo apt-get update && sudo apt-get install -y build-essential pkg-config libssl-dev

# 4. Install Stellar CLI
cargo install --locked stellar-cli --version 22.0.0

# 5. Clone and bootstrap
git clone https://github.com/veracindarella/paystream-contracts.git
cd paystream-contracts

# 6. Verify
make test
```

> If you prefer not to use WSL, the [Docker path](#docker-any-os) below works natively on Windows with Docker Desktop.

---

### Docker (any OS)

No local Rust or Stellar CLI installation required. Docker Compose mounts the repo and caches the Cargo registry between runs.

```bash
# Run the full test suite
docker compose run --rm test

# Build WASM artifacts only
docker compose run --rm build stellar contract build
```

The `cargo-cache` volume persists across runs so subsequent builds are fast. WASM output lands in `target/wasm32-unknown-unknown/release/` on your host machine via the bind mount.

---

## Project Structure

```
.
├── contracts
│   ├── stream/         # Core salary streaming and escrow contract
│   │   └── src/
│   │       ├── lib.rs      # Public entrypoints (create, withdraw, cancel, …)
│   │       ├── storage.rs  # Persistence helpers and claimable calculation
│   │       ├── events.rs   # On-chain event publishing
│   │       ├── types.rs    # Domain models, storage keys, error constants
│   │       └── test.rs     # Contract tests
│   └── token/          # SEP-41 fungible token used in tests
├── scripts/            # Build, deploy, and init shell scripts
├── Cargo.toml          # Workspace manifest
├── Makefile            # Common dev tasks
├── rust-toolchain.toml # Pinned Rust toolchain
└── docker-compose.yml  # Zero-install build/test environment
```

---

## Coding Standards

### General

- All code must compile without warnings: `cargo clippy --all-targets -- -D warnings`
- Formatting is enforced: `cargo fmt --check` must pass
- `#![no_std]` — the standard library is not available in contract code; use `soroban_sdk` primitives
- No floating-point arithmetic — all token amounts use `i128`
- All arithmetic on amounts must use checked or saturating operations (`checked_add`, `checked_mul`, `saturating_sub`). Silent wrapping is a bug

### Contract-specific rules

- Every state-changing function must emit an event via `events.rs` — this includes `initialize`, which must emit a `contract_initialized` event so off-chain indexers can determine when and by whom the contract was initialised
- Authorization must be checked with `address.require_auth()` before any state mutation
- Reentrancy guards (`stream.locked`) must be set before any cross-contract call and released after
- New storage keys belong in the `DataKey` enum in `types.rs`
- New error codes follow the `E###` convention defined in `types.rs` and must be documented in the error table there

### Error codes

| Code | Constant | Meaning |
|---|---|---|
| E001 | `ERR_ZERO_RATE` | `rate_per_second` must be > 0 |
| E002 | `ERR_ZERO_DEPOSIT` | `deposit` must be > 0 |
| E003 | `ERR_REENTRANT` | Reentrant withdraw detected |
| E004 | `ERR_OVERFLOW` | Arithmetic overflow in claimable calculation |
| E005 | `ERR_STREAM_CANCELLED` | Cannot top up a cancelled stream |
| E006 | `ERR_STREAM_EXHAUSTED` | Cannot top up an exhausted stream |

When adding a new error, assign the next available code, add the constant to `types.rs`, and document it in the table above.

### Documentation

- Public functions must have a doc comment (`///`) explaining parameters, return value, and any panic conditions
- Non-obvious logic must have inline comments — especially around reentrancy, overflow handling, and status transitions

---

## Commit Conventions

Follow [Conventional Commits](https://www.conventionalcommits.org/). Every commit message must have the form:

```
<type>(<optional scope>): <short description>

[optional body]

[optional footer(s)]
```

**Allowed types:**

| Type | When to use |
|---|---|
| `feat` | New contract function or observable behaviour |
| `fix` | Bug fix |
| `test` | Adding or updating tests only |
| `docs` | Documentation changes only |
| `refactor` | Code change that neither fixes a bug nor adds a feature |
| `chore` | Build scripts, CI config, dependency bumps |
| `perf` | Performance improvement |

**Examples:**

```
feat(stream): add top-up function to stream contract

fix(storage): cap claimable at remaining deposit to prevent over-withdrawal

test(stream): add stop_time boundary test for claimable calculation

docs: expand CONTRIBUTING with OS-specific setup instructions

chore: bump soroban-sdk to 22.0.0
```

**Rules:**
- Subject line ≤ 72 characters, imperative mood ("add", not "added" or "adds")
- No period at the end of the subject line
- Reference issues in the footer: `Closes #15` or `Fixes #3`
- Breaking changes must include `BREAKING CHANGE:` in the footer

---

## Bounty Program & Finding Issues

Looking for tasks to contribute to?

- **Bounty Program**: PayStream hosts open bounties with defined criteria for merged PRs. See [docs/bounties.md](docs/bounties.md) for active bounty categories, rules, and claiming instructions.
- **Good First Issues**: If you are new to the codebase, check out issues tagged with [`good-first-issue`](https://github.com/veracindarella/paystream-contracts/issues?q=is%3Aissue+is%3Aopen+label%3Agood-first-issue).

---

## Pull Request Process

### Before opening a PR

Run the full local check suite and make sure everything passes:

```bash
make fmt-check   # formatting
make lint        # clippy -D warnings
make test        # cargo test
```

### Branch naming

Branch from `main` using the pattern `<type>/<short-description>`:

```
feat/batch-stream-creation
fix/claimable-overflow
docs/contributing-setup
```

### PR checklist

When you open a PR, the description template will include this checklist. All items must be checked before requesting review:

- [ ] `make test` passes locally
- [ ] `make lint` passes (no clippy warnings)
- [ ] `make fmt-check` passes (no formatting diff)
- [ ] New contract functions have tests in `test.rs`
- [ ] Events are emitted for all state-changing operations
- [ ] New error codes are added to `types.rs` and documented
- [ ] Doc comments added or updated for changed public functions
- [ ] README updated if public behaviour or the function table changed
- [ ] No new `unwrap()` calls without a comment explaining why it is safe

### PR size

Keep PRs focused. A PR that touches a single concern is easier to review and faster to merge. If a feature requires both a contract change and a documentation update, they can live in the same PR — but unrelated changes should be separate.

### Merging

PRs are merged by a maintainer after at least one approving review and a passing CI run. Maintainers may squash commits to keep the history clean; if you want your individual commits preserved, say so in the PR description.

---

## Testing Requirements

### Where tests live

Each contract has a `test.rs` module gated with `#[cfg(test)]`. Tests use the Soroban SDK test utilities (`Env::default()`, `mock_all_auths()`, ledger time manipulation) — no external network required.

### What must be tested

- Every new public contract function needs at least one happy-path test
- Every new error condition (`assert!`, `panic!`) needs a `#[should_panic(expected = "...")]` test that matches the exact error code (e.g. `"E001"`)
- Edge cases that are explicitly called out in comments (overflow, boundary timestamps, status transitions) must have dedicated tests
- Reentrancy guards must be tested by manually setting `stream.locked = true` via `env.as_contract(...)` before calling `withdraw`

### Test structure conventions

```rust
#[test]
fn test_<what_is_being_tested>() {
    // Arrange
    let (env, client) = setup();
    // ... set up actors and state

    // Act
    // ... call the function under test

    // Assert
    assert_eq!(...);
}
```

Use the shared `setup()` and `setup_token()` helpers defined at the top of `test.rs` rather than duplicating boilerplate.

### Running tests

```bash
make test          # run all tests
cargo test <name>  # run a single test by name
```

### Snapshot files

Test snapshots in `test_snapshots/` are generated automatically by the SDK. Commit them alongside the test that produces them. If a snapshot changes unexpectedly, investigate before updating it — an unexpected snapshot diff often indicates a behaviour regression.

---

## Code Review Expectations

### For authors

- Respond to review comments within two business days
- If you disagree with a suggestion, explain your reasoning — don't just dismiss it
- Mark conversations as resolved only after addressing them (or reaching agreement to leave them as-is)
- Keep the PR up to date with `main`; rebase rather than merge to keep history linear

### For reviewers

- Review within two business days of being assigned
- Distinguish between blocking issues and non-blocking suggestions — prefix non-blocking comments with `nit:` or `suggestion:`
- Focus on correctness, security, and maintainability; style issues are handled by `clippy` and `rustfmt` automatically
- When reviewing contract code, pay particular attention to:
  - Authorization checks (`require_auth` present and in the right place)
  - Arithmetic overflow paths
  - Status transition correctness (`Active → Paused → Active`, `Active → Cancelled`, etc.)
  - Event emission for every state change
  - Reentrancy safety around cross-contract token transfers

### Security-sensitive changes

Any change to `withdraw`, `cancel_stream`, token transfer logic, or the reentrancy guard requires sign-off from a maintainer with contract security experience before merge. Tag such PRs with the `security` label.

---

## Glossary

| Term | Meaning |
|---|---|
| Stream | A salary stream from employer to employee |
| Deposit | Funds locked by employer at stream creation |
| Rate | Tokens released per second to the employee (`rate_per_second`) |
| Claimable | Tokens earned but not yet withdrawn |
| Stop time | Optional hard end timestamp; `0` means indefinite |
| Exhausted | Stream status when the full deposit has been withdrawn |
| SEP-41 | Stellar token interface standard (equivalent to ERC-20) |
| SAC | Stellar Asset Contract — a SEP-41 wrapper for native Stellar assets |

---

## License

Apache 2.0 — contributions are licensed under the same terms as the project.
