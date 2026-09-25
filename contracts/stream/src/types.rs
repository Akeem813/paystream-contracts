// SPDX-License-Identifier: Apache-2.0

use soroban_sdk::{contracttype, Address};

/// Status of a salary stream.
#[contracttype]
#[derive(Clone, Debug, PartialEq)]
pub enum StreamStatus {
    Active,
    Paused,
    Cancelled,
    Exhausted,
}

/// A salary stream: employer deposits funds, employee withdraws per-second.
#[contracttype]
#[derive(Clone, Debug)]
pub struct Stream {
    pub id: u64,
    pub employer: Address,
    pub employee: Address,
    pub token: Address,        // SAC token contract address
    pub deposit: i128,         // total deposited amount
    pub withdrawn: i128,       // total already withdrawn
    pub rate_per_second: i128, // tokens streamed per second
    pub start_time: u64,       // ledger timestamp when stream started
    pub stop_time: u64,        // 0 = no end, else hard stop timestamp
    pub last_withdraw_time: u64,
    pub status: StreamStatus,
    /// Reentrancy guard: true while a withdraw cross-contract call is in flight.
    /// Soroban executes contracts atomically within a single transaction, so
    /// cross-contract callbacks cannot interleave with the current frame.
    /// This flag is kept as a defence-in-depth measure and documents the
    /// analysis: no reentrant path exists in the current call graph because
    /// `token::transfer` is a leaf call that cannot call back into this
    /// contract.  If a future upgrade introduces a callback hook the guard
    /// will catch it.
    pub locked: bool,
}

/// Parameters for a single stream in a batch create call.
#[contracttype]
#[derive(Clone, Debug)]
pub struct StreamParams {
    pub employee: Address,
    pub token: Address,
    pub deposit: i128,
    pub rate_per_second: i128,
    pub stop_time: u64,
}

/// Storage keys.
#[contracttype]
pub enum DataKey {
    Stream(u64),
    StreamCount,
    Admin,
    /// Minimum deposit enforced on create_stream.
    MinDeposit,
    /// Monotonically-increasing nonce for admin operations (replay protection).
    AdminNonce,
    /// Contract-wide pause flag.
    Paused,
    /// Pending admin for two-step admin transfer.
    PendingAdmin,
    /// Nonce bound to the pending admin proposal (front-running protection).
    /// Cleared together with PendingAdmin when accept_admin completes.
    PendingAdminNonce,
    /// Index: employer address → Vec<u64> of stream IDs they own.
    EmployerStreams(Address),
    /// Index: employee address → Vec<u64> of stream IDs paying them.
    EmployeeStreams(Address),
}

/// Contract error codes – panic messages reference these names so callers can
/// match on a stable string.
///
/// | Code | Constant            | Meaning                                      |
/// |------|---------------------|----------------------------------------------|
/// | E001 | ERR_ZERO_RATE       | `rate_per_second` must be > 0                |
/// | E002 | ERR_ZERO_DEPOSIT    | `deposit` must be > 0                        |
/// | E003 | ERR_REENTRANT       | Reentrant withdraw detected                  |
/// | E004 | ERR_OVERFLOW        | Arithmetic overflow in claimable calculation |
pub const ERR_ZERO_RATE: &str = "E001: rate_per_second must be greater than zero";
pub const ERR_ZERO_DEPOSIT: &str = "E002: deposit must be positive";
pub const ERR_REENTRANT: &str = "E003: reentrant withdraw detected";
pub const ERR_OVERFLOW: &str = "E004: arithmetic overflow in claimable calculation";
pub const ERR_STREAM_CANCELLED: &str = "E005: cannot top up a cancelled stream";
pub const ERR_STREAM_EXHAUSTED: &str = "E006: cannot top up an exhausted stream";
pub const ERR_BELOW_MIN_DEPOSIT: &str = "E007: deposit below minimum";
pub const ERR_INVALID_RATE: &str = "E008: rate_per_second exceeds maximum";
pub const ERR_BAD_NONCE: &str = "E009: invalid admin nonce";
pub const ERR_NO_PENDING_ADMIN: &str = "E010: no pending admin set";
pub const ERR_NOT_PENDING_ADMIN: &str = "E011: not the pending admin";
pub const ERR_BAD_PENDING_NONCE: &str = "E024: invalid pending admin nonce";
