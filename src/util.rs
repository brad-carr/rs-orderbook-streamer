/// Pre-computed powers of 10 for rapid scaling.
const POWERS_OF_10: [i64; 16] = [
    1, 10, 100, 1_000, 10_000, 100_000, 1_000_000, 10_000_000,
    100_000_000, 1_000_000_000, 10_000_000_000, 100_000_000_000,
    1_000_000_000_000, 10_000_000_000_000, 100_000_000_000_000, 1_000_000_000_000_000
];

/// Parses a number into a fixed-point `i64` and returns the value and the index of the first non-numeric byte.
///
/// If the input has fewer decimal places than `precision`, it is padded with zeros.
/// If it has more, the extra digits are ignored (truncated).
///
/// # Examples
/// ```
/// let (val, next_idx) = parse_i64_with_precision(b"1.2", 0, 8);
/// assert_eq!(val, 120_000_000);
/// assert_eq!(next_idx, 3); // Index of the end of the slice
///
/// let (val2, next_idx2) = parse_i64_with_precision(b"45.67,next_field", 0, 4);
/// assert_eq!(val2, 456_700);
/// assert_eq!(next_idx2, 5); // Stopped at the comma
/// ```
pub fn parse_i64_with_precision(bytes: &[u8], start_idx: usize, target_scale: u32) -> (i64, usize) {
    let mut res = 0i64;
    let mut sign = 1i64;
    let mut idx = start_idx;
    let mut it = bytes[start_idx..].iter();

    // 1. Sign
    if let Some(&b'-') = it.clone().next() {
        sign = -1;
        it.next();
        idx += 1;
    }

    // 2. Parse integer portion
    while let Some(&b) = it.next() {
        match b {
            b'0'..=b'9' => {
                res = res * 10 + (b - b'0') as i64;
                idx += 1;
            }
            b'.' => {
                idx += 1;
                break;
            }
            _ => return (res * POWERS_OF_10[target_scale as usize] * sign, idx),
        }
    }

    // 3. parse fractional portion
    let mut digits_after_decimal = 0u32;
    while let Some(&b) = it.next() {
        match b {
            b'0'..=b'9' => {
                // Keep appending digits up to the target precision;
                // otherwise keep going to advance idx
                if digits_after_decimal < target_scale {
                    res = res * 10 + (b - b'0') as i64;
                    digits_after_decimal += 1;
                }
                idx += 1;
            }
            _ => break, // Delimiter reached
        }
    }

    let final_val = res * POWERS_OF_10[(target_scale - digits_after_decimal) as usize] * sign;
    (final_val, idx)
}
