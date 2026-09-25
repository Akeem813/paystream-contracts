// SPDX-License-Identifier: Apache-2.0

use crate::types::StreamStatus;
use soroban_sdk::{symbol_short, Address, Env};

pub fn stream_created(env: &Env, id: u64, employer: &Address, employee: &Address, rate: i128) {
    env.events().publish(
        (symbol_short!("created"), id),
        (employer.clone(), employee.clone(), rate),
    );
}

pub fn withdrawn(env: &Env, id: u64, employee: &Address, amount: i128) {
    env.events()
        .publish((symbol_short!("withdraw"), id), (employee.clone(), amount));
}

pub fn stream_status_changed(env: &Env, id: u64, status: &StreamStatus) {
    env.events()
        .publish((symbol_short!("status"), id), status.clone());
}

pub fn topped_up(env: &Env, id: u64, employer: &Address, amount: i128) {
    env.events()
        .publish((symbol_short!("topup"), id), (employer.clone(), amount));
}

pub fn contract_paused(env: &Env, paused: bool) {
    env.events().publish((symbol_short!("paused"),), paused);
}

/// Enriched cancellation event that includes the exact cash-flow amounts.
///
/// Emitted by `cancel_stream` instead of the generic `stream_status_changed`
/// event so that off-chain indexers can track:
/// - `claimable_paid` — tokens sent to the employee at cancellation time
/// - `refund_paid`    — tokens returned to the employer
pub fn stream_cancelled(
    env: &Env,
    stream_id: u64,
    employer: &Address,
    employee: &Address,
    claimable_paid: i128,
    refund_paid: i128,
) {
    env.events().publish(
        (symbol_short!("cancelled"), stream_id),
        (
            employer.clone(),
            employee.clone(),
            claimable_paid,
            refund_paid,
        ),
    );
}
