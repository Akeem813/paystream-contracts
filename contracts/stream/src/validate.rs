// SPDX-License-Identifier: Apache-2.0

use crate::types::{ERR_BELOW_MIN_DEPOSIT, ERR_INVALID_RATE, ERR_ZERO_DEPOSIT, ERR_ZERO_RATE};
use soroban_sdk::Address;

/// Maximum allowed rate_per_second (1 billion tokens/s — prevents overflow in
/// claimable_amount for any realistic elapsed time up to ~292 years).
pub const MAX_RATE_PER_SECOND: i128 = 1_000_000_000_i128;

/// Validate stream creation parameters.
///
/// # Panics
/// - E002 if `deposit` ≤ 0
/// - E007 if `deposit` < `min_deposit`
/// - E001 if `rate_per_second` ≤ 0
/// - E008 if `rate_per_second` > MAX_RATE_PER_SECOND
/// - if `stop_time` is in the past (when non-zero)
/// - if `employer` == `employee`
pub fn validate_create_stream(
    deposit: i128,
    min_deposit: i128,
    rate_per_second: i128,
    stop_time: u64,
    now: u64,
    employer: &Address,
    employee: &Address,
) {
    assert!(deposit > 0, "{}", ERR_ZERO_DEPOSIT);
    assert!(deposit >= min_deposit, "{}", ERR_BELOW_MIN_DEPOSIT);
    assert!(rate_per_second > 0, "{}", ERR_ZERO_RATE);
    assert!(
        rate_per_second <= MAX_RATE_PER_SECOND,
        "{}",
        ERR_INVALID_RATE
    );
    if stop_time > 0 {
        assert!(stop_time > now, "stop_time must be in the future");
    }
    assert!(employer != employee, "employer and employee must differ");
}

/// Validate a top-up amount.
pub fn validate_top_up(amount: i128) {
    assert!(amount > 0, "amount must be positive");
}
