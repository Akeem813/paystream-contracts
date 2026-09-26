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

/// Emitted when `update_rate` changes the stream's `rate_per_second`.
pub fn rate_updated(env: &Env, stream_id: u64, old_rate: i128, new_rate: i128) {
    env.events().publish(
        (symbol_short!("rate_upd"), stream_id),
        (old_rate, new_rate),
    );
}
