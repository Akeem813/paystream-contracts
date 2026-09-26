# PayStream Contribution Bounty Program

## Overview

The PayStream Bounty Program provides a structured framework for open-source contributors to contribute to decentralized salary streaming on the Stellar network. Contributors can identify open tasks, submit pull requests, and earn recognition and bounty rewards for merged code.

---

## Bounty Criteria & Claiming Rules

To claim any bounty in the PayStream repository, every submission must meet the following mandatory conditions:

1. **Pull Request Merged**: The pull request must be reviewed, approved, and merged into the upstream `main` branch.
2. **All Tests Pass**: 
   - `cargo test` passes completely with zero failures.
   - Any new contract function or state modification includes unit tests in `test.rs` covering happy paths and edge cases.
   - Error assertions must test explicit error codes.
3. **Documentation Updated**:
   - Relevant documentation in `docs/` (`docs/api-reference.md`, `docs/bounties.md`, etc.) must reflect changes.
   - Public contract methods must include docstrings documenting parameters, return values, and panic conditions.
   - When public behavior or contract interfaces change, `README.md` must be updated.
4. **Code Quality & Linting**:
   - `cargo clippy --all-targets -- -D warnings` passes without warnings.
   - `cargo fmt --check` passes with zero formatting diffs.
   - Code maintains `#![no_std]` compliance in contract modules.
   - Commits follow Conventional Commits formatting (`feat:`, `fix:`, `docs:`, `test:`, `chore:`).

---

## Bounty Tiers & Effort Levels

Tasks in the issue tracker are categorized by effort and complexity:

| Tier | Label | Description | Prerequisites |
|---|---|---|---|
| Starter | `good-first-issue`, `small` | Self-contained tasks, documentation additions, query getters, isolated tests | Basic Rust syntax |
| Intermediate | `medium`, `medium-effort` | Storage layout adjustments, event emission, CLI scripts, integration tests | Soroban SDK familiarity |
| Advanced | `high`, `large` | Core state machine logic, token mechanics, fuzzing targets, security mitigations | Deep Soroban and smart contract security knowledge |

---

## Beginner-Friendly Open Bounties (`good-first-issue`)

The following issues from the issue tracker are designated as starter bounties for new contributors:

| Issue | Title | Category | Effort | Description |
|---|---|---|---|---|
| [#87](https://github.com/veracindarella/paystream-contracts/issues/87) | DOC-03: Add glossary of Soroban and PayStream terms | Documentation | Small | Create a glossary explaining terms such as Stream, Accrual, Rate, and SEP-41 |
| [#90](https://github.com/veracindarella/paystream-contracts/issues/90) | DOC-06: Add FAQ section to README | Documentation | Small | Document common questions for integrators and contract consumers |
| [#95](https://github.com/veracindarella/paystream-contracts/issues/95) | DOC-11: Add error code reference table to API docs | Documentation | Small | Document all contract error codes (`E001`-`E006`+) with descriptions |
| [#7](https://github.com/veracindarella/paystream-contracts/issues/7) | SC-07: Emit contract_initialized event in initialize | Smart Contract | Small | Publish an event upon contract initialization in `lib.rs` |
| [#16](https://github.com/veracindarella/paystream-contracts/issues/16) | SC-16: Add get_min_deposit public query function | Smart Contract | Small | Expose a read-only getter for minimum stream deposit threshold |
| [#17](https://github.com/veracindarella/paystream-contracts/issues/17) | SC-17: Add is_paused public query function | Smart Contract | Small | Add a view function returning boolean paused status of a stream |
| [#20](https://github.com/veracindarella/paystream-contracts/issues/20) | SC-20: Add admin getter public function | Smart Contract | Small | Expose a read-only getter returning the current contract admin address |
| [#57](https://github.com/veracindarella/paystream-contracts/issues/57) | TEST-08: Add test for claimable_at with stop_time boundary conditions | Testing | Small | Add test coverage for claimable calculation around stream stop timestamps |
| [#77](https://github.com/veracindarella/paystream-contracts/issues/77) | OPS-08: Add Makefile target for one-command dev environment setup | DevOps | Small | Add a `setup` target in `Makefile` to install target and dependencies |
| [#79](https://github.com/veracindarella/paystream-contracts/issues/79) | OPS-10: Add pre-commit hooks for fmt and clippy | DevOps | Small | Implement a git pre-commit hook enforcing formatting and clippy |
| [#108](https://github.com/veracindarella/paystream-contracts/issues/108) | TOK-09: Add allowance query function | Token Contract | Small | Expose read-only allowance getter matching SEP-41 standard |

---

## Active Bounty Categories from Issue Tracker

Contributors can explore open issues across the following specialized tracks:

### 1. Smart Contracts (`smart-contract`)
- Stream state machine improvements (auto-transitions, pause/resume guards)
- Rate adjustment functions (`rate_per_second` updates)
- Pagination and query helpers for stream indexes (`stream_ids_paginated`)
- Stream transfer and reassignment functionality

### 2. Token Integration (`token-contract`)
- Full SEP-41 compliance enhancements (name, symbol, decimals, allowances)
- Two-step admin transfers and re-initialization guards
- Supply overflow protections and token event emissions

### 3. Testing & Verification (`testing`)
- Property-based testing with `proptest` for stream lifecycle transitions
- Fuzzing targets for deposit and withdrawal arithmetic
- Snapshot tests for batch operations (`create_streams_batch`)
- Test coverage reporting integration in CI

### 4. Security & Hardening (`security`)
- Reentrancy guard validation
- Key rotation runbooks and admin timelock mechanisms
- Error code audit across panic assertions
- Emergency circuit-breakers and drain controls

### 5. DevOps & Operations (`devops`, `ci-cd`)
- Release automation and CHANGELOG generators
- GitHub Actions version pinning and toolchain matrices
- Stellar CLI contract optimization checks in CI
- Local sandbox development containers

### 6. Documentation (`documentation`)
- Architecture diagrams and flowcharts
- Worked examples for claimable accrual arithmetic
- Operations runbooks and deployment guides

---

## Contribution Workflow

Follow these steps to submit work for a bounty:

```
[1. Select Issue] ──> [2. Signal Intent] ──> [3. Fork & Branch]
         │
         ▼
[4. Implement & Test] ──> [5. Open Draft PR] ──> [6. Review & Merge]
```

### Step 1: Select an Issue
Browse the [GitHub Issues](https://github.com/veracindarella/paystream-contracts/issues) tracker. Look for the `good-first-issue` label if you are a first-time contributor.

### Step 2: Signal Intent
Comment on the issue expressing your plan to work on it. Maintainers will confirm assignment to avoid duplicated effort. Work on one active issue at a time.

### Step 3: Fork and Branch
Fork the repository and create a feature branch:
```bash
git checkout -b feat/your-feature-name
```

### Step 4: Implement and Test Locally
Ensure all checks pass before opening a PR:
```bash
make fmt-check
make lint
make test
```

### Step 5: Open a Draft Pull Request
Open a Draft PR against `veracindarella/paystream-contracts:main`. Reference the issue number in the PR description (e.g., `Closes #119`).

### Step 6: Review, Merge, and Bounty Fulfillment
Once checks are green, mark the PR "Ready for Review". Address maintainer feedback promptly. Upon merge, the bounty acceptance criteria are fulfilled.
