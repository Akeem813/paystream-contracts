# Testnet Deployment

This document lists the current PayStream testnet contract addresses, explains how to get test tokens, and describes how to interact with the contracts on Stellar Testnet.

> **Note:** Contract IDs change after every redeployment. This file is updated as part of the release process. If the IDs below are stale, redeploy using the scripts in `scripts/` and update this file.

---

## Current Contract IDs

| Contract | ID |
|---|---|
| PayStream Token | `CDZQHVQHQMHIGJGSQIJVXGPGNZJQDQV4BBKMG7MSIDTJTTBBQYAEUPB` |
| PayStream Stream | `CBXKDJUQYQDQKSQT6AKZRQWXDXTQHVXHBQQJQ3WKDXJHPNMRGYAQMRS` |

Network: **Stellar Testnet** (`https://horizon-testnet.stellar.org`)
RPC URL: `https://soroban-testnet.stellar.org`
Last deployed: **2026-09-25**

---

## Prerequisites

- [Rust](https://rustup.rs/) (latest stable) with `wasm32-unknown-unknown` target
- [Stellar CLI](https://developers.stellar.org/docs/tools/developer-tools/cli/stellar-cli)

```bash
rustup target add wasm32-unknown-unknown
stellar network add testnet \
  --rpc-url https://soroban-testnet.stellar.org \
  --network-passphrase "Test SDF Network ; September 2015"
```

---

## Getting a Testnet Account and Test XLM

1. Generate a keypair:
   ```bash
   stellar keys generate --global my-testnet-key --network testnet
   stellar keys address my-testnet-key
   ```

2. Fund it via the Friendbot faucet:
   ```bash
   # Replace <YOUR_PUBLIC_KEY> with the address from the previous step
   curl "https://friendbot.stellar.org?addr=<YOUR_PUBLIC_KEY>"
   ```
   Or use the web faucet: <https://laboratory.stellar.org/#account-creator?network=test>

3. Verify the balance:
   ```bash
   stellar account show <YOUR_PUBLIC_KEY> --network testnet
   ```

---

## Getting Test Tokens (PayStream Token)

The PayStream Token contract is pre-initialised with an admin-controlled supply. To mint test tokens to your account, ask the admin or run:

```bash
stellar contract invoke \
  --id <TOKEN_CONTRACT_ID> \
  --source <ADMIN_KEY> \
  --network testnet \
  -- mint --admin <ADMIN_ADDRESS> --to <YOUR_ADDRESS> --amount 1000000000
```

Check your balance:
```bash
stellar contract invoke \
  --id <TOKEN_CONTRACT_ID> \
  --source <YOUR_KEY> \
  --network testnet \
  -- balance --owner <YOUR_ADDRESS>
```

---

## Deploying Contracts (maintainers)

```bash
./scripts/build.sh
./scripts/deploy-testnet.sh
```

The script prints the new contract IDs. Export them and initialise:

```bash
export STELLAR_ADMIN_ADDRESS=<YOUR_PUBLIC_KEY>
export TOKEN_CONTRACT_ID=<FROM_DEPLOY>
export STREAM_CONTRACT_ID=<FROM_DEPLOY>
./scripts/init-testnet.sh
```

Update the **Current Contract IDs** table above and commit the change.

---

## Quick Interaction Examples

### Create a stream

```bash
stellar contract invoke \
  --id <STREAM_CONTRACT_ID> \
  --source <EMPLOYER_KEY> \
  --network testnet \
  -- create_stream \
    --employer <EMPLOYER_ADDRESS> \
    --employee <EMPLOYEE_ADDRESS> \
    --token_address <TOKEN_CONTRACT_ID> \
    --deposit 1000000 \
    --rate_per_second 100 \
    --stop_time 0
```

### Check claimable amount

```bash
stellar contract invoke \
  --id <STREAM_CONTRACT_ID> \
  --source <ANY_KEY> \
  --network testnet \
  -- claimable --stream_id 1
```

### Withdraw earnings

```bash
stellar contract invoke \
  --id <STREAM_CONTRACT_ID> \
  --source <EMPLOYEE_KEY> \
  --network testnet \
  -- withdraw --employee <EMPLOYEE_ADDRESS> --stream_id 1
```

---

## Useful Links

- Stellar Testnet Horizon: <https://horizon-testnet.stellar.org>
- Stellar Laboratory: <https://laboratory.stellar.org>
- Friendbot faucet: <https://friendbot.stellar.org>
- Soroban Testnet RPC: <https://soroban-testnet.stellar.org>
