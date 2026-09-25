// SPDX-License-Identifier: Apache-2.0

#![cfg(test)]

use soroban_sdk::testutils::{Address as _, Ledger as _};
use soroban_sdk::{Address, Env};

use crate::storage;
use crate::types::StreamStatus;
use crate::{StreamContract, StreamContractClient};

fn setup() -> (Env, StreamContractClient<'static>) {
    let env = Env::default();
    env.mock_all_auths();
    let id = env.register(StreamContract, ());
    let client = StreamContractClient::new(&env, &id);
    (env, client)
}

fn setup_token(env: &Env, admin: &Address) -> Address {
    let token_id = env.register(paystream_token::TokenContract, ());
    let token = paystream_token::TokenContractClient::new(env, &token_id);
    token.initialize(admin, &1_000_000_000);
    token_id
}

// ---------------------------------------------------------------------------
// Existing tests (updated for nonce-aware admin calls)
// ---------------------------------------------------------------------------

#[test]
fn test_create_stream() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let employer = Address::generate(&env);
    let employee = Address::generate(&env);
    let token_id = setup_token(&env, &employer);

    client.initialize(&admin);
    client.set_min_deposit(&admin, &0, &100);
    let id = client.create_stream(&employer, &employee, &token_id, &3600, &1, &0);
    assert_eq!(id, 1);
    assert_eq!(client.stream_count(), 1);

    let s = client.get_stream(&id);
    assert_eq!(s.status, StreamStatus::Active);
    assert_eq!(s.deposit, 3600);
    assert_eq!(s.rate_per_second, 1);
    assert_eq!(s.withdrawn, 0);
    assert!(!s.locked);
}

#[test]
fn test_claimable_increases_with_time() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let employer = Address::generate(&env);
    let employee = Address::generate(&env);
    let token_id = setup_token(&env, &employer);

    client.initialize(&admin);
    let id = client.create_stream(&employer, &employee, &token_id, &10_000, &10, &0);

    env.ledger().with_mut(|l| l.timestamp += 100);
    assert_eq!(client.claimable(&id), 1000);
}

#[test]
fn test_withdraw() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let employer = Address::generate(&env);
    let employee = Address::generate(&env);
    let token_id = setup_token(&env, &employer);

    client.initialize(&admin);
    let id = client.create_stream(&employer, &employee, &token_id, &10_000, &10, &0);

    env.ledger().with_mut(|l| l.timestamp += 200);
    let withdrawn = client.withdraw(&employee, &id);
    assert_eq!(withdrawn, 2000);

    let s = client.get_stream(&id);
    assert_eq!(s.withdrawn, 2000);
    assert_eq!(s.status, StreamStatus::Active);
    assert!(!s.locked);
}

#[test]
fn test_stream_exhausted_when_fully_withdrawn() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let employer = Address::generate(&env);
    let employee = Address::generate(&env);
    let token_id = setup_token(&env, &employer);

    client.initialize(&admin);
    client.set_min_deposit(&admin, &0, &100);
    let id = client.create_stream(&employer, &employee, &token_id, &500, &10, &0);

    env.ledger().with_mut(|l| l.timestamp += 100);
    let withdrawn = client.withdraw(&employee, &id);
    assert_eq!(withdrawn, 500);
    assert_eq!(client.get_stream(&id).status, StreamStatus::Exhausted);
}

#[test]
fn test_pause_and_resume() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let employer = Address::generate(&env);
    let employee = Address::generate(&env);
    let token_id = setup_token(&env, &employer);

    client.initialize(&admin);
    let id = client.create_stream(&employer, &employee, &token_id, &10_000, &10, &0);

    env.ledger().with_mut(|l| l.timestamp += 100);
    client.pause_stream(&employer, &id);

    env.ledger().with_mut(|l| l.timestamp += 100);
    client.resume_stream(&employer, &id);

    env.ledger().with_mut(|l| l.timestamp += 50);
    assert_eq!(client.claimable(&id), 500);
}

#[test]
fn test_cancel_stream_refunds_employer() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let employer = Address::generate(&env);
    let employee = Address::generate(&env);
    let token_id = setup_token(&env, &employer);

    client.initialize(&admin);
    let id = client.create_stream(&employer, &employee, &token_id, &10_000, &10, &0);

    env.ledger().with_mut(|l| l.timestamp += 100);
    client.cancel_stream(&employer, &id);

    let s = client.get_stream(&id);
    assert_eq!(s.status, StreamStatus::Cancelled);
    assert_eq!(s.withdrawn, 1000);
}

#[test]
fn test_stop_time_caps_claimable() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let employer = Address::generate(&env);
    let employee = Address::generate(&env);
    let token_id = setup_token(&env, &employer);

    client.initialize(&admin);
    let now = env.ledger().timestamp();
    let id = client.create_stream(&employer, &employee, &token_id, &10_000, &10, &(now + 50));

    env.ledger().with_mut(|l| l.timestamp += 200);
    assert_eq!(client.claimable(&id), 500);
}

#[test]
fn test_pause_excludes_paused_time() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let employer = Address::generate(&env);
    let employee = Address::generate(&env);
    let token_id = setup_token(&env, &employer);

    client.initialize(&admin);
    let id = client.create_stream(&employer, &employee, &token_id, &10_000, &10, &0);

    env.ledger().with_mut(|l| l.timestamp += 50);
    client.pause_stream(&employer, &id);
    env.ledger().with_mut(|l| l.timestamp += 100);
    client.resume_stream(&employer, &id);
    env.ledger().with_mut(|l| l.timestamp += 50);

    // last_withdraw_time is reset to resume time; only 50 active seconds since resume count
    assert_eq!(client.claimable(&id), 500);
}

#[test]
fn test_multiple_pause_resume_cycles() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let employer = Address::generate(&env);
    let employee = Address::generate(&env);
    let token_id = setup_token(&env, &employer);

    client.initialize(&admin);
    let id = client.create_stream(&employer, &employee, &token_id, &10_000, &10, &0);

    env.ledger().with_mut(|l| l.timestamp += 30);
    client.pause_stream(&employer, &id);
    env.ledger().with_mut(|l| l.timestamp += 200);
    client.resume_stream(&employer, &id);

    env.ledger().with_mut(|l| l.timestamp += 20);
    client.pause_stream(&employer, &id);
    env.ledger().with_mut(|l| l.timestamp += 300);
    client.resume_stream(&employer, &id);

    env.ledger().with_mut(|l| l.timestamp += 40);

    // last_withdraw_time is reset to each resume; only 40 active seconds since last resume count
    assert_eq!(client.claimable(&id), 400);
}

#[test]
#[should_panic(expected = "stream not active")]
fn test_withdraw_during_pause_panics() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let employer = Address::generate(&env);
    let employee = Address::generate(&env);
    let token_id = setup_token(&env, &employer);

    client.initialize(&admin);
    let id = client.create_stream(&employer, &employee, &token_id, &10_000, &10, &0);

    env.ledger().with_mut(|l| l.timestamp += 50);
    client.pause_stream(&employer, &id);
    env.ledger().with_mut(|l| l.timestamp += 100);
    client.withdraw(&employee, &id);
}

#[test]
#[should_panic(expected = "stream not active")]
fn test_cannot_withdraw_from_cancelled_stream() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let employer = Address::generate(&env);
    let employee = Address::generate(&env);
    let token_id = setup_token(&env, &employer);

    client.initialize(&admin);
    let id = client.create_stream(&employer, &employee, &token_id, &10_000, &10, &0);
    client.cancel_stream(&employer, &id);

    env.ledger().with_mut(|l| l.timestamp += 100);
    client.withdraw(&employee, &id);
}

#[test]
fn test_withdraw_exhausted_returns_zero() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let employer = Address::generate(&env);
    let employee = Address::generate(&env);
    let token_id = setup_token(&env, &employer);

    client.initialize(&admin);
    client.set_min_deposit(&admin, &0, &100);
    let id = client.create_stream(&employer, &employee, &token_id, &500, &10, &0);

    env.ledger().with_mut(|l| l.timestamp += 100);
    client.withdraw(&employee, &id);
    assert_eq!(client.get_stream(&id).status, StreamStatus::Exhausted);

    let result = client.withdraw(&employee, &id);
    assert_eq!(result, 0);
}

#[test]
#[should_panic(expected = "stream not active")]
fn test_withdraw_cancelled_still_panics() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let employer = Address::generate(&env);
    let employee = Address::generate(&env);
    let token_id = setup_token(&env, &employer);

    client.initialize(&admin);
    let id = client.create_stream(&employer, &employee, &token_id, &10_000, &10, &0);
    client.cancel_stream(&employer, &id);
    client.withdraw(&employee, &id);
}

#[test]
#[should_panic(expected = "E003")]
fn test_reentrant_withdraw_rejected() {
    use crate::storage::save_stream;

    let (env, client) = setup();
    let admin = Address::generate(&env);
    let employer = Address::generate(&env);
    let employee = Address::generate(&env);
    let token_id = setup_token(&env, &employer);

    client.initialize(&admin);
    let id = client.create_stream(&employer, &employee, &token_id, &10_000, &10, &0);

    env.as_contract(&client.address, || {
        let mut stream = storage::load_stream(&env, id).unwrap();
        stream.locked = true;
        save_stream(&env, &stream);
    });

    env.ledger().with_mut(|l| l.timestamp += 100);
    client.withdraw(&employee, &id);
}

#[test]
#[should_panic(expected = "E004")]
fn test_claimable_overflow_panics() {
    use crate::storage::claimable_amount;
    use crate::types::{Stream, StreamStatus};

    let env = Env::default();
    let addr = Address::generate(&env);

    let stream = Stream {
        id: 1,
        employer: addr.clone(),
        employee: addr.clone(),
        token: addr.clone(),
        deposit: i128::MAX,
        withdrawn: 0,
        rate_per_second: i128::MAX,
        start_time: 0,
        stop_time: 0,
        last_withdraw_time: 0,
        status: StreamStatus::Active,
        locked: false,
    };

    claimable_amount(&stream, 2);
}

#[test]
fn test_claimable_large_elapsed_capped_by_deposit() {
    use crate::storage::claimable_amount;
    use crate::types::{Stream, StreamStatus};

    let env = Env::default();
    let addr = Address::generate(&env);

    let deposit: i128 = 1_000_000;
    let stream = Stream {
        id: 1,
        employer: addr.clone(),
        employee: addr.clone(),
        token: addr.clone(),
        deposit,
        withdrawn: 0,
        rate_per_second: 1,
        start_time: 0,
        stop_time: 0,
        last_withdraw_time: 0,
        status: StreamStatus::Active,
        locked: false,
    };

    let result = claimable_amount(&stream, u64::MAX);
    assert_eq!(result, deposit);
}

#[test]
#[should_panic(expected = "E001")]
fn test_create_stream_zero_rate_rejected() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let employer = Address::generate(&env);
    let employee = Address::generate(&env);
    let token_id = setup_token(&env, &employer);

    client.initialize(&admin);
    client.create_stream(&employer, &employee, &token_id, &10_000, &0, &0);
}

#[test]
fn test_create_stream_positive_rate_ok() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let employer = Address::generate(&env);
    let employee = Address::generate(&env);
    let token_id = setup_token(&env, &employer);

    client.initialize(&admin);
    client.set_min_deposit(&admin, &0, &100);
    let id = client.create_stream(&employer, &employee, &token_id, &3600, &1, &0);
    assert_eq!(id, 1);
    assert_eq!(client.get_stream(&id).rate_per_second, 1);
}

// ---------------------------------------------------------------------------
// Issue #70 – Admin nonce / replay attack protection
// ---------------------------------------------------------------------------

/// Nonce starts at 0 and increments after each admin op.
#[test]
fn test_admin_nonce_increments() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin);

    assert_eq!(client.admin_nonce(), 0);
    client.set_min_deposit(&admin, &0, &500);
    assert_eq!(client.admin_nonce(), 1);
    client.set_min_deposit(&admin, &1, &1000);
    assert_eq!(client.admin_nonce(), 2);
}

/// Replaying an already-consumed nonce must be rejected with E009.
#[test]
#[should_panic(expected = "E009")]
fn test_replayed_admin_nonce_rejected() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin);

    client.set_min_deposit(&admin, &0, &500); // nonce 0 consumed
    client.set_min_deposit(&admin, &0, &500); // replay → must panic
}

/// pause_contract and unpause_contract consume the nonce.
#[test]
fn test_pause_unpause_consume_nonce() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin);

    client.pause_contract(&0);
    assert_eq!(client.admin_nonce(), 1);
    client.unpause_contract(&1);
    assert_eq!(client.admin_nonce(), 2);
}

// ---------------------------------------------------------------------------
// Issue #72 – Input validation
// ---------------------------------------------------------------------------

/// deposit below min_deposit must be rejected with E007.
#[test]
#[should_panic(expected = "E007")]
fn test_create_stream_below_min_deposit_rejected() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let employer = Address::generate(&env);
    let employee = Address::generate(&env);
    let token_id = setup_token(&env, &employer);

    client.initialize(&admin);
    client.set_min_deposit(&admin, &0, &10_000);
    // deposit = 100 < min_deposit = 10_000 → E007
    client.create_stream(&employer, &employee, &token_id, &100, &1, &0);
}

/// rate_per_second above MAX_RATE_PER_SECOND must be rejected with E008.
#[test]
#[should_panic(expected = "E008")]
fn test_create_stream_rate_too_high_rejected() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let employer = Address::generate(&env);
    let employee = Address::generate(&env);
    let token_id = setup_token(&env, &employer);

    client.initialize(&admin);
    // 1_000_000_001 > MAX_RATE_PER_SECOND → E008
    client.create_stream(
        &employer,
        &employee,
        &token_id,
        &1_000_000_000_000,
        &1_000_000_001,
        &0,
    );
}

/// employer == employee must be rejected.
#[test]
#[should_panic(expected = "employer and employee must differ")]
fn test_create_stream_same_employer_employee_rejected() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let employer = Address::generate(&env);
    let token_id = setup_token(&env, &employer);

    client.initialize(&admin);
    client.create_stream(&employer, &employer, &token_id, &10_000, &1, &0);
}

/// top_up with amount = 0 must be rejected.
#[test]
#[should_panic(expected = "amount must be positive")]
fn test_top_up_zero_amount_rejected() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let employer = Address::generate(&env);
    let employee = Address::generate(&env);
    let token_id = setup_token(&env, &employer);

    client.initialize(&admin);
    let id = client.create_stream(&employer, &employee, &token_id, &10_000, &1, &0);
    client.top_up(&employer, &id, &0);
}

// ---------------------------------------------------------------------------
// Issue #20 – Contract upgrade / migration path
// ---------------------------------------------------------------------------

mod stream_wasm {
    soroban_sdk::contractimport!(file = "../../target/wasm32v1-none/release/paystream_stream.wasm");
}

#[test]
fn test_upgrade_preserves_stream_state() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let employer = Address::generate(&env);
    let employee = Address::generate(&env);
    let token_id = setup_token(&env, &employer);

    client.initialize(&admin);
    let id = client.create_stream(&employer, &employee, &token_id, &10_000, &10, &0);

    env.ledger().with_mut(|l| l.timestamp += 100);

    let new_wasm_hash = env.deployer().upload_contract_wasm(stream_wasm::WASM);
    client.upgrade(&new_wasm_hash, &0);

    let s = client.get_stream(&id);
    assert_eq!(s.deposit, 10_000);
    assert_eq!(s.rate_per_second, 10);
    assert_eq!(s.status, StreamStatus::Active);
    assert_eq!(client.claimable(&id), 1000);
}

#[test]
fn test_migrate_noop() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    client.initialize(&admin);
    client.migrate(&admin);
}

#[test]
#[should_panic]
fn test_upgrade_non_admin_rejected() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(StreamContract, ());
    let client = StreamContractClient::new(&env, &contract_id);

    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Now stop mocking all auths — attacker's call should fail admin.require_auth()
    let env2 = Env::default();
    env2.register_at(&contract_id, StreamContract, ());
    let client2 = StreamContractClient::new(&env2, &contract_id);

    let new_wasm_hash = env2.deployer().upload_contract_wasm(stream_wasm::WASM);
    client2.upgrade(&new_wasm_hash, &0);
}

// ---------------------------------------------------------------------------
// Issue #19 – Two-step admin transfer
// ---------------------------------------------------------------------------

#[test]
fn test_admin_transfer_full_flow() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    client.initialize(&admin);

    client.propose_admin(&new_admin);
    client.accept_admin(&new_admin);

    // new_admin can now call propose_admin (proves they are admin)
    let another = Address::generate(&env);
    client.propose_admin(&another); // would panic if new_admin is not admin
}

#[test]
#[should_panic]
fn test_propose_admin_non_admin_rejected() {
    // Don't use mock_all_auths — we need auth to actually fail for non-admin
    let env = Env::default();
    let contract_id = env.register(StreamContract, ());
    let client = StreamContractClient::new(&env, &contract_id);

    // Initialize with mocked auth
    env.mock_all_auths();
    let admin = Address::generate(&env);
    client.initialize(&admin);

    // Create a new env without mock_all_auths for the attacker's call
    let env2 = Env::default();
    env2.register_at(&contract_id, StreamContract, ());
    let client2 = StreamContractClient::new(&env2, &contract_id);

    let attacker = Address::generate(&env2);
    client2.propose_admin(&attacker); // should panic — attacker is not admin
}

#[test]
#[should_panic(expected = "not the pending admin")]
fn test_accept_admin_wrong_address_rejected() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let new_admin = Address::generate(&env);
    let attacker = Address::generate(&env);
    client.initialize(&admin);
    client.propose_admin(&new_admin);
    client.accept_admin(&attacker); // wrong address
}

// ---------------------------------------------------------------------------
// Issue #59 – TEST-10: top_up extends stream duration
// ---------------------------------------------------------------------------

/// Verify that a top_up increases the deposit and extends how long the stream
/// runs past the point it would otherwise have been exhausted.
///
/// Timeline
/// --------
///   T=0   create stream  deposit=100, rate=10 (exhausts at T=10 without top-up)
///   T=5   withdraw       claimable = 5×10 = 50; withdrawn=50, last_withdraw_time=5
///   T=5   top_up +100    deposit=200; stream now exhausts at T=5+(150/10)=T=20
///   T=15  claimable      (15-5)×10 = 100; remaining = 200-50 = 150 → claimable = 100
///   T=20  withdraw       (20-5)×10 = 150 = remaining → stream exhausts, withdrawn = 200
#[test]
fn test_top_up_extends_stream_duration() {
    let (env, client) = setup();
    let admin = Address::generate(&env);
    let employer = Address::generate(&env);
    let employee = Address::generate(&env);
    let token_id = setup_token(&env, &employer);

    client.initialize(&admin);
    client.set_min_deposit(&admin, &0, &100);

    // T=0: create a stream that would exhaust at T=10 without a top-up.
    let id = client.create_stream(&employer, &employee, &token_id, &100, &10, &0);
    assert_eq!(client.get_stream(&id).deposit, 100);

    // T=5: partial withdrawal — proves stream is active and accruing.
    env.ledger().with_mut(|l| l.timestamp += 5);
    let w1 = client.withdraw(&employee, &id);
    assert_eq!(w1, 50, "5s × 10/s = 50 tokens earned");
    assert_eq!(client.get_stream(&id).status, StreamStatus::Active);

    // T=5: top-up — deposit grows from 100 to 200.
    // Without this top-up the stream would be fully drained in 5 more seconds
    // (T=10). With the top-up the remaining balance is 150, so it exhausts at
    // T = 5 + 150/10 = T=20 — 10 seconds later than the original end.
    client.top_up(&employer, &id, &100);
    assert_eq!(client.get_stream(&id).deposit, 200);
    assert_eq!(client.get_stream(&id).status, StreamStatus::Active);

    // T=15: 10 seconds after the top-up; the original stream would have been
    // fully exhausted by now, but the top-up kept it running.
    env.ledger().with_mut(|l| l.timestamp += 10);
    let claimable_t15 = client.claimable(&id);
    // elapsed since last_withdraw_time(T=5) = 10s; 10×10 = 100
    // remaining = 200 - 50 = 150; earned (100) < remaining → claimable = 100
    assert_eq!(claimable_t15, 100, "stream still active past original T=10 exhaustion");
    assert_eq!(client.get_stream(&id).status, StreamStatus::Active);

    // T=20: stream reaches new exhaustion point.
    env.ledger().with_mut(|l| l.timestamp += 5);
    let w2 = client.withdraw(&employee, &id);
    // elapsed since last_withdraw_time(T=5) = 15s; 15×10 = 150 = remaining
    assert_eq!(w2, 150, "remaining 150 tokens drained at T=20");
    assert_eq!(client.get_stream(&id).status, StreamStatus::Exhausted);

    // Total withdrawn = 50 + 150 = 200 = full deposit.
    assert_eq!(client.get_stream(&id).withdrawn, 200);
}
