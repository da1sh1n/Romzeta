// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2026 da1sh1n
// This file is part of Romzeta, licensed under the GNU General Public License
// v3.0 or later. Romzeta comes with ABSOLUTELY NO WARRANTY. See the LICENSE file
// or <https://www.gnu.org/licenses/> for details.

//! Formats the current UTC time: `YYYY-MM-DD HH:MM:SSZ` and `YYYY-MM-DD`.
//! Computed from the system clock with integer arithmetic.

// ########## UTC DATE AND TIME ##########

use std::time::{SystemTime, UNIX_EPOCH};

// ========== Formatted ==========

/// The current UTC time as `YYYY-MM-DD HH:MM:SSZ`, for a log line.
/// The trailing `Z` stops a reader taking the line for their own wall clock.
pub fn timestamp() -> String {
    let secs = nowSeconds();
    let (year, month, day) = civilFromDays((secs / 86_400) as i64);
    let tod = secs % 86_400; // seconds elapsed so far today
    format!(
        "{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02}Z",
        tod / 3600,
        (tod % 3600) / 60,
        tod % 60
    )
}

/// Today's UTC date as `YYYY-MM-DD`, for the trusted comment `xtask` signs into
/// a binary. Provenance for a human; nothing parses it.
pub fn today() -> String {
    let (year, month, day) = civilFromDays((nowSeconds() / 86_400) as i64);
    format!("{year:04}-{month:02}-{day:02}")
}

// ========== The Calendar ==========

/// Seconds since the Unix epoch, or 0 if the machine clock is set before 1970 —
/// which is a fine answer for a log line and not worth an error path.
fn nowSeconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Turns days since the Unix epoch into `(year, month, day)`.
///
/// `pub` for the tests in `tests/time.rs`: the dates worth proving — the
/// epoch, the March pivot, a leap day, a date before 1970 — cannot be reached
/// through `today()`, which only ever asks about now.
///
/// Howard Hinnant's `civil_from_days`: it shifts the year to begin in March so
/// the leap day lands at the very end, which collapses the month-length table
/// into plain arithmetic. Correct for every date the epoch can express.
pub fn civilFromDays(days: i64) -> (i64, u32, u32) {
    // Re-base onto 0000-03-01, the start of a 400-year cycle.
    let z = days + 719_468;
    // Which 400-year cycle, and how far into it. `div_euclid`/`rem_euclid`
    // floor towards negative infinity, which plain `/` and `%` do not do for
    // negative numbers — that is what makes pre-1970 dates come out right.
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097); // day of era, [0, 146096]
    // Year of era, correcting back out the leap days already counted in `doe`.
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365; // [0, 399]
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year, [0, 365]
    // The March-based month. 153/5 is exact because the March-to-January month
    // lengths repeat in a 5-month, 153-day pattern.
    let mp = (5 * doy + 2) / 153; // [0, 11]
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32; // [1, 31]
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32; // back to [1, 12]
    // January and February sit at the end of the shifted year, so they belong
    // to the next calendar one.
    (if m <= 2 { y + 1 } else { y }, m, d)
}
