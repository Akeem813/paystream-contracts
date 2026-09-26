#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0
#
# e2e-test.sh — End-to-end payroll lifecycle test on Stellar Testnet
#
# Exercises the full flow:
#   deploy → initialize → mint tokens → create stream → wait 30s
#   → withdraw → verify balance → cancel remaining stream
#
# Usage:
#   export STELLAR_SOURCE_ACCOUNT=<your-key-name>   # defaults to "default"
#   export STELLAR_ADMIN_ADDRESS=<your-public-key>
#   ./scripts/e2e-test.sh
#
# The script exits non-zero on any failure (set -euo pipefail).

set -euo pipefail

NETWORK="testnet"
SOURCE="${STELLAR_SOURCE_ACCOUNT:-default}"
ADMIN="${STELLAR_ADMIN_ADDRESS:?Error: STELLAR_ADMIN_ADDRESS must be set}"

RATE_PER_SECOND=10
DEPOSIT=100000
WAIT_SECONDS=30

log() { echo "[e2e] $*"; }
fail() { echo "[e2e] FAIL: $*" >&2; exit 1; }

# ---------------------------------------------------------------------------
# Step 1: Build contracts
# ---------------------------------------------------------------------------
log "Building contracts..."
stellar contract build 2>&1 || fail "Build failed"

# ---------------------------------------------------------------------------
# Step 2: Deploy both contracts
# ---------------------------------------------------------------------------
log "Deploying token contract..."
TOKEN_ID=$(stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/paystream_token.wasm \
  --source "$SOURCE" --network "$NETWORK")
[[ -n "$TOKEN_ID" ]] || fail "Token deploy returned empty contract ID"
log "Token contract: $TOKEN_ID"

log "Deploying stream contract..."
STREAM_ID=$(stellar contract deploy \
  --wasm target/wasm32-unknown-unknown/release/paystream_stream.wasm \
  --source "$SOURCE" --network "$NETWORK")
[[ -n "$STREAM_ID" ]] || fail "Stream deploy returned empty contract ID"
log "Stream contract: $STREAM_ID"

# ---------------------------------------------------------------------------
# Step 3: Initialize both contracts
# ---------------------------------------------------------------------------
log "Initializing token contract..."
stellar contract invoke --id "$TOKEN_ID" --source "$SOURCE" --network "$NETWORK" \
  -- initialize --admin "$ADMIN" --initial_supply 1000000000

log "Initializing stream contract..."
stellar contract invoke --id "$STREAM_ID" --source "$SOURCE" --network "$NETWORK" \
  -- initialize --admin "$ADMIN"

# ---------------------------------------------------------------------------
# Step 4: Generate employer and employee keypairs
# ---------------------------------------------------------------------------
log "Generating employer keypair..."
stellar keys generate --network testnet e2e-employer 2>/dev/null || true
EMPLOYER=$(stellar keys address e2e-employer)
log "Employer: $EMPLOYER"

log "Funding employer via Friendbot..."
curl -sf "https://friendbot.stellar.org?addr=${EMPLOYER}" > /dev/null || \
  fail "Friendbot funding failed for employer"

log "Generating employee keypair..."
stellar keys generate --network testnet e2e-employee 2>/dev/null || true
EMPLOYEE=$(stellar keys address e2e-employee)
log "Employee: $EMPLOYEE"

log "Funding employee via Friendbot..."
curl -sf "https://friendbot.stellar.org?addr=${EMPLOYEE}" > /dev/null || \
  fail "Friendbot funding failed for employee"

# ---------------------------------------------------------------------------
# Step 5: Mint tokens to employer
# ---------------------------------------------------------------------------
log "Minting $DEPOSIT tokens to employer..."
stellar contract invoke --id "$TOKEN_ID" --source "$SOURCE" --network "$NETWORK" \
  -- mint --admin "$ADMIN" --to "$EMPLOYER" --amount "$DEPOSIT"

EMPLOYER_BALANCE=$(stellar contract invoke --id "$TOKEN_ID" --source "$SOURCE" \
  --network "$NETWORK" -- balance --owner "$EMPLOYER")
log "Employer token balance: $EMPLOYER_BALANCE"
[[ "$EMPLOYER_BALANCE" -ge "$DEPOSIT" ]] || fail "Employer balance too low after mint"

# ---------------------------------------------------------------------------
# Step 6: Create stream
# ---------------------------------------------------------------------------
log "Creating stream: rate=${RATE_PER_SECOND}/s, deposit=${DEPOSIT}..."
STREAM_RESULT=$(stellar contract invoke --id "$STREAM_ID" --source e2e-employer \
  --network "$NETWORK" \
  -- create_stream \
    --employer "$EMPLOYER" \
    --employee "$EMPLOYEE" \
    --token_address "$TOKEN_ID" \
    --deposit "$DEPOSIT" \
    --rate_per_second "$RATE_PER_SECOND" \
    --stop_time 0)
STREAM_NUM="$STREAM_RESULT"
log "Stream created with ID: $STREAM_NUM"

# ---------------------------------------------------------------------------
# Step 7: Wait 30 seconds for tokens to accrue
# ---------------------------------------------------------------------------
log "Waiting ${WAIT_SECONDS}s for salary to accrue..."
sleep "$WAIT_SECONDS"

CLAIMABLE=$(stellar contract invoke --id "$STREAM_ID" --source "$SOURCE" \
  --network "$NETWORK" -- claimable --stream_id "$STREAM_NUM")
log "Claimable after ${WAIT_SECONDS}s: $CLAIMABLE"
EXPECTED_MIN=$((RATE_PER_SECOND * WAIT_SECONDS))
[[ "$CLAIMABLE" -ge "$EXPECTED_MIN" ]] || \
  fail "Claimable ($CLAIMABLE) less than expected minimum ($EXPECTED_MIN)"

# ---------------------------------------------------------------------------
# Step 8: Withdraw
# ---------------------------------------------------------------------------
log "Employee withdrawing claimable earnings..."
WITHDRAWN=$(stellar contract invoke --id "$STREAM_ID" --source e2e-employee \
  --network "$NETWORK" -- withdraw --employee "$EMPLOYEE" --stream_id "$STREAM_NUM")
log "Withdrawn: $WITHDRAWN"
[[ "$WITHDRAWN" -gt 0 ]] || fail "Withdraw returned 0"

# ---------------------------------------------------------------------------
# Step 9: Verify employee token balance increased
# ---------------------------------------------------------------------------
EMPLOYEE_BALANCE=$(stellar contract invoke --id "$TOKEN_ID" --source "$SOURCE" \
  --network "$NETWORK" -- balance --owner "$EMPLOYEE")
log "Employee token balance after withdrawal: $EMPLOYEE_BALANCE"
[[ "$EMPLOYEE_BALANCE" -gt 0 ]] || fail "Employee balance is 0 after withdrawal"

# ---------------------------------------------------------------------------
# Step 10: Cancel remaining stream
# ---------------------------------------------------------------------------
log "Cancelling remaining stream..."
stellar contract invoke --id "$STREAM_ID" --source e2e-employer \
  --network "$NETWORK" -- cancel_stream --employer "$EMPLOYER" \
  --stream_id "$STREAM_NUM"
log "Stream cancelled."

STREAM_STATE=$(stellar contract invoke --id "$STREAM_ID" --source "$SOURCE" \
  --network "$NETWORK" -- get_stream --stream_id "$STREAM_NUM")
log "Final stream state: $STREAM_STATE"

# ---------------------------------------------------------------------------
# Done
# ---------------------------------------------------------------------------
log ""
log "✅ E2E test PASSED"
log "   Token contract:  $TOKEN_ID"
log "   Stream contract: $STREAM_ID"
log "   Stream ID:       $STREAM_NUM"
log "   Withdrawn:       $WITHDRAWN"
log "   Employee balance: $EMPLOYEE_BALANCE"
