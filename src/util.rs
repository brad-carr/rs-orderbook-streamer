/// Parses a numeric byte slice into a fixed-point `i64` with a target precision.
///
/// If the input has fewer decimal places than `precision`, it is padded with zeros.
/// If it has more, the extra digits are ignored (truncated).
///
/// # Examples
/// ```
/// let val = parse_i64_with_precision(b"1.2", 8);
/// assert_eq!(val, 120_000_000);
/// ```
pub fn parse_i64_with_precision(bytes: &[u8], precision: i8) -> i64 {
    let mut res = 0i64;
    let mut sign = 1i64;
    let mut decimal_found = false;
    let mut digits_after_decimal = 0;
    let target_scale = precision as i32;

    if bytes.is_empty() { return 0; }

    let start = if bytes[0] == b'-' {
        sign = -1;
        1
    } else {
        0
    };

    for &b in &bytes[start..] {
        if b == b'.' {
            decimal_found = true;
            continue;
        }

        if decimal_found {
            if digits_after_decimal >= target_scale { break; }
            digits_after_decimal += 1;
        }

        res = res * 10 + (b - b'0') as i64;
    }

    // Pad remaining precision with 10^(target - actual)
    if digits_after_decimal < target_scale {
        let diff = target_scale - digits_after_decimal;
        res *= 10i64.pow(diff as u32);
    }

    res * sign
}