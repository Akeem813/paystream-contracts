// SPDX-License-Identifier: Apache-2.0

#![no_std]

mod events;
pub mod storage;
pub mod types;
mod validate;

#[cfg(test)]
mod test;

use soroban_sdk::{contract, contractimpl, token, Address, BytesN, Env, Vec};
use storage::{
    claimable_amount, clear_pending_admin, consume_admin_nonce, get_admin, get_admin_nonce,
    get_employee_streams, get_employer_streams, get_min_deposit, get_pending_admin,
    get_pending_admin_nonce, index_employee_stream, index_employer_stream, load_stream, next_id,
    save_stream, set_admin, set_min_deposit, set_pending_admin, set_pending_admin_nonce,
};
use types::{
    DataKey, Stream, StreamParams, StreamStatus, ERR_BAD_PENDING_NONCE, ERR_NO_PENDING_ADMIN,
    ERR_NOT_PENDING_ADMIN, ERR_REENTRANT, ERR_STREAM_CANCELLED, ERR_STREAM_EXHAUSTED,
    ERR_ZERO_DEPOSIT,
};
use validate::{validate_create_stream, validate_top_up};

fn get_paused(env: &Env) -> bool {
    env.storage()
        .instance()
        .get(&DataKey::Paused)
        .unwrap_or(false)
}

fn set_paused(env: &Env, paused: bool) {
    env.storage().instance().set(&DataKey::Paused, &paused);
}

#[contract]
pub struct StreamContract;

#[contractimpl]
impl StreamContract {
    /// Initialise the contract with an admin address.
    ///
    /// Must be called once after deployment. The `admin` address gains the
    /// ability to pause/unpause the contract, set the minimum deposit, and
    /// perform upgrades.
    ///
    /// # Parameters
    /// - `admin` — address that becomes the contract admin (requires auth)
    ///
    /// # Errors
    /// - Panics if `admin` auth fails
    pub fn initialize(env: Env, admin: Address) {
        admin.require_auth();
        set_admin(&env, &admin);
    }

    /// Step 1 of two-step admin transfer: current admin proposes a new admin.
    ///
    /// The nominated address must call [`accept_admin`] with the same `nonce`
    /// to complete the transfer. Binding a nonce to the proposal makes the
    /// transfer intent non-replayable: an attacker who observes the proposal
    /// on-chain cannot front-run acceptance without knowing the nonce.
    ///
    /// # Parameters
    /// - `new_admin` — address being nominated as the next admin
    /// - `nonce` — current admin nonce; consumed here for replay protection
    ///   and also stored so `accept_admin` can verify it
    ///
    /// # Errors
    /// - Panics if the current admin auth fails
    /// - E009 if `nonce` does not match the stored admin nonce
    pub fn propose_admin(env: Env, new_admin: Address, nonce: u64) {
        let current = get_admin(&env);
        current.require_auth();
        consume_admin_nonce(&env, nonce);
        set_pending_admin(&env, &new_admin);
        set_pending_admin_nonce(&env, nonce);
    }

    /// Step 2 of two-step admin transfer: proposed admin accepts and becomes admin.
    ///
    /// # Parameters
    /// - `new_admin` — must match the address set by [`propose_admin`] (requires auth)
    /// - `nonce` — must match the nonce stored by [`propose_admin`]
    ///
    /// # Errors
    /// - E010 if there is no pending admin
    /// - E011 if `new_admin` does not match the pending admin
    /// - E024 if `nonce` does not match the nonce stored by propose_admin
    pub fn accept_admin(env: Env, new_admin: Address, nonce: u64) {
        new_admin.require_auth();
        let pending = get_pending_admin(&env).expect(ERR_NO_PENDING_ADMIN);
        assert_eq!(pending, new_admin, "{}", ERR_NOT_PENDING_ADMIN);
        let stored_nonce = get_pending_admin_nonce(&env).expect(ERR_NO_PENDING_ADMIN);
        assert!(nonce == stored_nonce, "{}", ERR_BAD_PENDING_NONCE);
        set_admin(&env, &new_admin);
        clear_pending_admin(&env);
    }

    /// Admin pauses the entire contract — blocks new streams and withdrawals.
    ///
    /// While paused, `create_stream`, `create_streams_batch`, and `withdraw`
    /// will all panic. Admin operations (top-up, cancel, etc.) remain available.
    ///
    /// # Parameters
    /// - `nonce` — current admin nonce; must match the stored value (replay protection)
    ///
    /// # Errors
    /// - Panics if admin auth fails
    /// - E009 if `nonce` does not match the stored nonce
    pub fn pause_contract(env: Env, nonce: u64) {
        let admin = get_admin(&env);
        admin.require_auth();
        consume_admin_nonce(&env, nonce);
        set_paused(&env, true);
        events::contract_paused(&env, true);
    }

    /// Admin unpauses the contract, restoring normal operation.
    ///
    /// # Parameters
    /// - `nonce` — current admin nonce; must match the stored value (replay protection)
    ///
    /// # Errors
    /// - Panics if admin auth fails
    /// - E009 if `nonce` does not match the stored nonce
    pub fn unpause_contract(env: Env, nonce: u64) {
        let admin = get_admin(&env);
        admin.require_auth();
        consume_admin_nonce(&env, nonce);
        set_paused(&env, false);
        events::contract_paused(&env, false);
    }

    /// Set the minimum deposit enforced on `create_stream`.
    ///
    /// Streams created after this call must have `deposit >= amount`.
    /// Existing streams are unaffected.
    ///
    /// # Parameters
    /// - `admin` — must match the stored admin (requires auth)
    /// - `nonce` — current admin nonce (replay protection)
    /// - `amount` — new minimum deposit (must be > 0)
    ///
    /// # Errors
    /// - Panics if `admin` auth fails or does not match stored admin
    /// - E009 if `nonce` is wrong
    /// - E002 if `amount` ≤ 0
    pub fn set_min_deposit(env: Env, admin: Address, nonce: u64, amount: i128) {
        admin.require_auth();
        let stored_admin = get_admin(&env);
        assert_eq!(admin, stored_admin, "not the admin");
        consume_admin_nonce(&env, nonce);
        assert!(amount > 0, "{}", ERR_ZERO_DEPOSIT);
        set_min_deposit(&env, amount);
    }

    /// Employer creates a salary stream and deposits funds into the contract escrow.
    ///
    /// Tokens are transferred from `employer` to the contract immediately.
    /// The employee can call [`withdraw`] at any time to claim earned tokens.
    ///
    /// # Parameters
    /// - `employer` — employer address; funds are pulled from here (requires auth)
    /// - `employee` — employee address; receives streamed tokens
    /// - `token_address` — SEP-41 token contract address
    /// - `deposit` — total tokens to lock in escrow (must be ≥ min deposit)
    /// - `rate_per_second` — tokens streamed per second (1 – 1,000,000,000)
    /// - `stop_time` — hard stop timestamp in seconds; 0 means indefinite
    ///
    /// # Returns
    /// The new stream ID as `u64`.
    ///
    /// # Errors
    /// - Panics if contract is paused
    /// - E002 if `deposit` ≤ 0
    /// - E007 if `deposit` < minimum deposit
    /// - E001 if `rate_per_second` ≤ 0
    /// - E008 if `rate_per_second` > 1,000,000,000
    /// - Panics if `stop_time` is non-zero and in the past
    /// - Panics if `employer` == `employee`
    /// - Panics if the token transfer fails
    pub fn create_stream(
        env: Env,
        employer: Address,
        employee: Address,
        token_address: Address,
        deposit: i128,
        rate_per_second: i128,
        stop_time: u64,
    ) -> u64 {
        employer.require_auth();
        assert!(!get_paused(&env), "contract is paused");

        let now = env.ledger().timestamp();
        let min_deposit = get_min_deposit(&env);
        validate_create_stream(
            deposit,
            min_deposit,
            rate_per_second,
            stop_time,
            now,
            &employer,
            &employee,
        );

        let token_client = token::Client::new(&env, &token_address);
        token_client.balance(&employer); // SEP-41 probe
        token_client.transfer(&employer, &env.current_contract_address(), &deposit);

        let id = next_id(&env);
        let stream = Stream {
            id,
            employer: employer.clone(),
            employee: employee.clone(),
            token: token_address,
            deposit,
            withdrawn: 0,
            rate_per_second,
            start_time: now,
            stop_time,
            last_withdraw_time: now,
            status: StreamStatus::Active,
            locked: false,
        };
        save_stream(&env, &stream);
        index_employer_stream(&env, &employer, id);
        index_employee_stream(&env, &employee, id);
        events::stream_created(&env, id, &employer, &employee, rate_per_second);
        id
    }

    /// Employer creates multiple salary streams atomically in a single transaction.
    ///
    /// All streams succeed or all revert. Cheaper than N individual
    /// `create_stream` calls for N ≥ 2 because Stellar charges one base fee
    /// per transaction.
    ///
    /// # Parameters
    /// - `employer` — employer address (requires auth)
    /// - `params` — list of [`StreamParams`]; must not be empty
    ///
    /// # Returns
    /// `Vec<u64>` of new stream IDs in the same order as `params`.
    ///
    /// # Errors
    /// - Panics if contract is paused
    /// - Panics if `params` is empty
    /// - Same per-stream validations as [`create_stream`]
    pub fn create_streams_batch(
        env: Env,
        employer: Address,
        params: Vec<StreamParams>,
    ) -> Vec<u64> {
        employer.require_auth();
        assert!(!get_paused(&env), "contract is paused");
        assert!(!params.is_empty(), "params must not be empty");

        let now = env.ledger().timestamp();
        let min_deposit = get_min_deposit(&env);
        let mut ids: Vec<u64> = Vec::new(&env);

        for p in params.iter() {
            validate_create_stream(
                p.deposit,
                min_deposit,
                p.rate_per_second,
                p.stop_time,
                now,
                &employer,
                &p.employee,
            );

            let token_client = token::Client::new(&env, &p.token);
            token_client.balance(&employer); // SEP-41 probe
            token_client.transfer(&employer, &env.current_contract_address(), &p.deposit);

            let id = next_id(&env);
            let stream = Stream {
                id,
                employer: employer.clone(),
                employee: p.employee.clone(),
                token: p.token.clone(),
                deposit: p.deposit,
                withdrawn: 0,
                rate_per_second: p.rate_per_second,
                start_time: now,
                stop_time: p.stop_time,
                last_withdraw_time: now,
                status: StreamStatus::Active,
                locked: false,
            };
            save_stream(&env, &stream);
            index_employer_stream(&env, &employer, id);
            index_employee_stream(&env, &p.employee, id);
            events::stream_created(&env, id, &employer, &p.employee, p.rate_per_second);
            ids.push_back(id);
        }

        ids
    }

    /// Employee withdraws all claimable tokens earned so far.
    ///
    /// Claimable amount is `min((now - last_withdraw_time) * rate_per_second, remaining_deposit)`.
    /// Returns 0 without reverting if nothing is claimable yet.
    /// Marks the stream Exhausted when the full deposit has been withdrawn.
    ///
    /// # Parameters
    /// - `employee` — must match the stream's employee (requires auth)
    /// - `stream_id` — ID of the stream to withdraw from
    ///
    /// # Returns
    /// Amount transferred as `i128`; 0 if nothing was claimable.
    ///
    /// # Errors
    /// - Panics if contract is paused
    /// - Panics if stream not found
    /// - Panics if caller is not the stream's employee
    /// - Panics if stream is not Active or Exhausted
    /// - E003 if a reentrant withdraw is detected
    pub fn withdraw(env: Env, employee: Address, stream_id: u64) -> i128 {
        employee.require_auth();
        assert!(!get_paused(&env), "contract is paused");
        let mut stream = load_stream(&env, stream_id).expect("stream not found");
        assert_eq!(stream.employee, employee, "not the employee");
        assert!(
            stream.status == StreamStatus::Active || stream.status == StreamStatus::Exhausted,
            "stream not active"
        );

        let now = env.ledger().timestamp();
        let amount = claimable_amount(&stream, now);
        if amount == 0 {
            return 0;
        }

        assert!(!stream.locked, "{}", ERR_REENTRANT);
        stream.locked = true;
        save_stream(&env, &stream);

        stream.withdrawn = stream
            .withdrawn
            .checked_add(amount)
            .expect("withdrawn overflow");
        stream.last_withdraw_time = now;
        if stream.withdrawn >= stream.deposit {
            stream.status = StreamStatus::Exhausted;
        }

        let token_client = token::Client::new(&env, &stream.token);
        token_client.transfer(&env.current_contract_address(), &employee, &amount);

        stream.locked = false;
        save_stream(&env, &stream);
        events::withdrawn(&env, stream_id, &employee, amount);
        amount
    }

    /// Employer tops up an active stream with additional funds.
    ///
    /// Increases `deposit` by `amount`. The stream's rate and timeline are
    /// unchanged; the extra funds simply extend how long the stream can run.
    ///
    /// # Parameters
    /// - `employer` — must match the stream's employer (requires auth)
    /// - `stream_id` — ID of the stream to top up
    /// - `amount` — additional tokens to deposit (must be > 0)
    ///
    /// # Errors
    /// - Panics if stream not found
    /// - Panics if caller is not the stream's employer
    /// - E005 if stream is Cancelled
    /// - E006 if stream is Exhausted
    /// - Panics if `amount` ≤ 0
    /// - Panics if the token transfer fails
    pub fn top_up(env: Env, employer: Address, stream_id: u64, amount: i128) {
        employer.require_auth();
        validate_top_up(amount);
        let mut stream = load_stream(&env, stream_id).expect("stream not found");
        assert_eq!(stream.employer, employer, "not the employer");
        assert!(
            stream.status != StreamStatus::Cancelled,
            "{}",
            ERR_STREAM_CANCELLED
        );
        assert!(
            stream.status != StreamStatus::Exhausted,
            "{}",
            ERR_STREAM_EXHAUSTED
        );

        let token_client = token::Client::new(&env, &stream.token);
        token_client.transfer(&employer, &env.current_contract_address(), &amount);

        stream.deposit = stream
            .deposit
            .checked_add(amount)
            .expect("deposit overflow");
        save_stream(&env, &stream);
        events::topped_up(&env, stream_id, &employer, amount);
    }

    /// Employer pauses an active stream, stopping token accrual.
    ///
    /// The employee cannot withdraw while the stream is paused. Call
    /// [`resume_stream`] to restart accrual; paused time is excluded from
    /// the claimable calculation.
    ///
    /// # Parameters
    /// - `employer` — must match the stream's employer (requires auth)
    /// - `stream_id` — ID of the stream to pause
    ///
    /// # Errors
    /// - Panics if stream not found
    /// - Panics if caller is not the stream's employer
    /// - Panics if stream is not Active
    pub fn pause_stream(env: Env, employer: Address, stream_id: u64) {
        employer.require_auth();
        let mut stream = load_stream(&env, stream_id).expect("stream not found");
        assert_eq!(stream.employer, employer, "not the employer");
        assert_eq!(stream.status, StreamStatus::Active, "stream not active");
        stream.status = StreamStatus::Paused;
        save_stream(&env, &stream);
        events::stream_status_changed(&env, stream_id, &StreamStatus::Paused);
    }

    /// Employer resumes a paused stream, restarting token accrual.
    ///
    /// `last_withdraw_time` is reset to the current ledger timestamp so that
    /// the paused interval is excluded from future claimable calculations.
    ///
    /// # Parameters
    /// - `employer` — must match the stream's employer (requires auth)
    /// - `stream_id` — ID of the stream to resume
    ///
    /// # Errors
    /// - Panics if stream not found
    /// - Panics if caller is not the stream's employer
    /// - Panics if stream is not Paused
    pub fn resume_stream(env: Env, employer: Address, stream_id: u64) {
        employer.require_auth();
        let mut stream = load_stream(&env, stream_id).expect("stream not found");
        assert_eq!(stream.employer, employer, "not the employer");
        assert_eq!(stream.status, StreamStatus::Paused, "stream not paused");
        stream.last_withdraw_time = env.ledger().timestamp();
        stream.status = StreamStatus::Active;
        save_stream(&env, &stream);
        events::stream_status_changed(&env, stream_id, &StreamStatus::Active);
    }

    /// Employer cancels a stream and reclaims unstreamed funds.
    ///
    /// The employee receives all tokens earned up to the cancellation time.
    /// The employer is refunded the remaining deposit. Works on both Active
    /// and Paused streams.
    ///
    /// # Parameters
    /// - `employer` — must match the stream's employer (requires auth)
    /// - `stream_id` — ID of the stream to cancel
    ///
    /// # Errors
    /// - Panics if stream not found
    /// - Panics if caller is not the stream's employer
    /// - Panics if stream is already Cancelled or Exhausted
    pub fn cancel_stream(env: Env, employer: Address, stream_id: u64) {
        employer.require_auth();
        let mut stream = load_stream(&env, stream_id).expect("stream not found");
        assert_eq!(stream.employer, employer, "not the employer");
        assert!(
            stream.status == StreamStatus::Active || stream.status == StreamStatus::Paused,
            "stream already ended"
        );

        let now = env.ledger().timestamp();
        let claimable = claimable_amount(&stream, now);
        let token_client = token::Client::new(&env, &stream.token);

        if claimable > 0 {
            token_client.transfer(
                &env.current_contract_address(),
                &stream.employee,
                &claimable,
            );
            stream.withdrawn = stream
                .withdrawn
                .checked_add(claimable)
                .expect("withdrawn overflow");
        }

        let refund = stream
            .deposit
            .checked_sub(stream.withdrawn)
            .unwrap_or(0)
            .max(0);
        if refund > 0 {
            token_client.transfer(&env.current_contract_address(), &employer, &refund);
        }

        stream.status = StreamStatus::Cancelled;
        save_stream(&env, &stream);
        events::stream_status_changed(&env, stream_id, &StreamStatus::Cancelled);
    }

    /// Read the full state of a stream by ID.
    ///
    /// # Parameters
    /// - `stream_id` — ID of the stream to read
    ///
    /// # Returns
    /// The [`Stream`] struct.
    ///
    /// # Errors
    /// - Panics if stream not found
    pub fn get_stream(env: Env, stream_id: u64) -> Stream {
        load_stream(&env, stream_id).expect("stream not found")
    }

    /// Query how many tokens the employee can withdraw right now.
    ///
    /// Returns 0 for Cancelled or Exhausted streams.
    ///
    /// # Parameters
    /// - `stream_id` — ID of the stream to query
    ///
    /// # Returns
    /// Claimable token amount as `i128`.
    ///
    /// # Errors
    /// - Panics if stream not found
    pub fn claimable(env: Env, stream_id: u64) -> i128 {
        let stream = load_stream(&env, stream_id).expect("stream not found");
        claimable_amount(&stream, env.ledger().timestamp())
    }

    /// Query how many tokens would be claimable at an arbitrary timestamp.
    ///
    /// Useful for off-chain projections without advancing ledger time.
    ///
    /// # Parameters
    /// - `stream_id` — ID of the stream to query
    /// - `timestamp` — hypothetical ledger timestamp (seconds)
    ///
    /// # Returns
    /// Claimable amount at `timestamp` as `i128`.
    ///
    /// # Errors
    /// - Panics if stream not found
    pub fn claimable_at(env: Env, stream_id: u64, timestamp: u64) -> i128 {
        let stream = load_stream(&env, stream_id).expect("stream not found");
        claimable_amount(&stream, timestamp)
    }

    /// Admin upgrades the contract WASM in-place.
    ///
    /// The new WASM must be uploaded to the network before calling this.
    /// After upgrading, call [`migrate`] to confirm the new WASM is operational.
    ///
    /// # Parameters
    /// - `new_wasm_hash` — 32-byte hash of the uploaded WASM blob
    /// - `nonce` — current admin nonce (replay protection)
    ///
    /// # Errors
    /// - Panics if admin auth fails
    /// - E009 if `nonce` is wrong
    pub fn upgrade(env: Env, new_wasm_hash: BytesN<32>, nonce: u64) {
        let admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("admin not set");
        admin.require_auth();
        consume_admin_nonce(&env, nonce);
        env.deployer().update_current_contract_wasm(new_wasm_hash);
    }

    /// No-op migration hook called by the admin after an upgrade.
    ///
    /// Confirms the new WASM is operational and the admin key is still valid.
    /// Future upgrades may add state migration logic here.
    ///
    /// # Parameters
    /// - `admin` — must match the stored admin (requires auth)
    ///
    /// # Errors
    /// - Panics if `admin` auth fails or does not match stored admin
    pub fn migrate(env: Env, admin: Address) {
        admin.require_auth();
        let stored_admin: Address = env
            .storage()
            .instance()
            .get(&DataKey::Admin)
            .expect("admin not set");
        assert_eq!(admin, stored_admin, "not the admin");
    }

    /// Return the total number of streams ever created.
    ///
    /// IDs are assigned sequentially starting at 1, so this also equals the
    /// highest stream ID in existence.
    ///
    /// # Returns
    /// Stream count as `u64`.
    pub fn stream_count(env: Env) -> u64 {
        env.storage()
            .instance()
            .get(&DataKey::StreamCount)
            .unwrap_or(0)
    }

    /// Return the current admin nonce.
    ///
    /// Use this to build the `nonce` argument for the next admin transaction
    /// (`pause_contract`, `unpause_contract`, `set_min_deposit`, `upgrade`).
    ///
    /// # Returns
    /// Current nonce as `u64`.
    pub fn admin_nonce(env: Env) -> u64 {
        get_admin_nonce(&env)
    }

    /// Return all stream IDs owned by `employer`.
    ///
    /// # Parameters
    /// - `employer` — employer address to query
    ///
    /// # Returns
    /// `Vec<u64>` of stream IDs; empty if the address has no streams.
    pub fn streams_by_employer(env: Env, employer: Address) -> Vec<u64> {
        get_employer_streams(&env, &employer)
    }

    /// Return all stream IDs paying `employee`.
    ///
    /// # Parameters
    /// - `employee` — employee address to query
    ///
    /// # Returns
    /// `Vec<u64>` of stream IDs; empty if the address receives no streams.
    pub fn streams_by_employee(env: Env, employee: Address) -> Vec<u64> {
        get_employee_streams(&env, &employee)
    }
}
