use crate::{
    OracleValidation, ValidationResult, CLOSE_HOUR, OPEN_HOUR, SECONDS_PER_DAY, SECONDS_PER_HOUR,
};

pub fn transfers_open(unix_timestamp: i64) -> bool {
    let hour = unix_timestamp.rem_euclid(SECONDS_PER_DAY) / SECONDS_PER_HOUR;
    (OPEN_HOUR..CLOSE_HOUR).contains(&hour)
}

/// Seconds elapsed since the most recent open or close boundary.
pub fn seconds_since_boundary(unix_timestamp: i64) -> i64 {
    let second_of_day = unix_timestamp.rem_euclid(SECONDS_PER_DAY);
    let open = OPEN_HOUR * SECONDS_PER_HOUR;
    let close = CLOSE_HOUR * SECONDS_PER_HOUR;
    if second_of_day >= close {
        second_of_day - close
    } else if second_of_day >= open {
        second_of_day - open
    } else {
        second_of_day + SECONDS_PER_DAY - close
    }
}

/// Only Transfer is ever decided here; the adapter is registered for Transfer alone,
/// so the other slots are Pass and never consulted.
pub fn validation_at(unix_timestamp: i64) -> OracleValidation {
    let transfer = if transfers_open(unix_timestamp) {
        ValidationResult::Pass
    } else {
        ValidationResult::Rejected
    };
    OracleValidation::V1 {
        create: ValidationResult::Pass,
        transfer,
        burn: ValidationResult::Pass,
        update: ValidationResult::Pass,
    }
}
