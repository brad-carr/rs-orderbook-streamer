/// Pre-computed powers of 10 for rapid scaling.
const POWERS_OF_10: [i64; 16] = [
    1, 10, 100, 1_000, 10_000, 100_000, 1_000_000, 10_000_000,
    100_000_000, 1_000_000_000, 10_000_000_000, 100_000_000_000,
    1_000_000_000_000, 10_000_000_000_000, 100_000_000_000_000, 1_000_000_000_000_000
];

#[derive(Debug, PartialEq)]
pub enum ParseError {
    EmptyInput,
    InvalidFirstChar,
    NoDigits,
    InvalidTerminator,
}

/// Parses a number into a fixed-point `i64` and returns the value and the index of the first non-numeric byte.
///
/// If the input has fewer decimal places than `precision`, it is padded with zeros.
/// If it has more, the extra digits are ignored (truncated).
///
/// # Examples
/// ```
/// use rs_orderbook_streamer::util::{parse_i64_with_precision, ParseError};
///
/// let (val, next_idx) = parse_i64_with_precision(b"1.2", 0, 8).unwrap();
/// assert_eq!(val, 120_000_000);
/// assert_eq!(next_idx, 3); // Index of the end of the slice
///
/// let (val2, next_idx2) = parse_i64_with_precision(b"45.67,next_field", 0, 4).unwrap();
/// assert_eq!(val2, 456_700);
/// assert_eq!(next_idx2, 5); // Stopped at the comma
/// ```
pub fn parse_i64_with_precision(bytes: &[u8], start_idx: usize, target_scale: u32) -> Result<(i64, usize), ParseError> {
    if start_idx >= bytes.len() {
        return Err(ParseError::EmptyInput);
    }

    let mut idx = start_idx;
    let mut sign = 1i64;

    // 1. Sign or First Digit
    match bytes[idx] {
        b'-' => {
            sign = -1;
            idx += 1;
            if idx >= bytes.len() {
                return Err(ParseError::NoDigits);
            }
        }
        b'0'..=b'9' | b'.' => {
            // OK, proceed to parsing
        }
        _ => return Err(ParseError::InvalidFirstChar),
    }

    let mut res = 0i64;
    let mut digits_seen = false;

    // 2. Parse integer portion
    while idx < bytes.len() {
        let b = bytes[idx];
        match b {
            b'0'..=b'9' => {
                res = res * 10 + (b - b'0') as i64;
                digits_seen = true;
                idx += 1;
            }
            b'.' => {
                idx += 1;
                // We allow a trailing dot (e.g. "1." or "1.a"), treating it as "1.0"
                break;
            }
            b'-' => {
                // A dash inside the number is invalid
                return Err(ParseError::InvalidTerminator);
            }
            _ => {
                // Terminator reached
                if !digits_seen {
                    // e.g. "-" followed by non-digit
                    return Err(ParseError::NoDigits);
                }
                return Ok((res * POWERS_OF_10[target_scale as usize] * sign, idx));
            }
        }
    }

    // If we finished loop without hitting '.' or terminator
    if idx == bytes.len() {
        if !digits_seen {
            return Err(ParseError::NoDigits);
        }
        return Ok((res * POWERS_OF_10[target_scale as usize] * sign, idx));
    }

    // 3. Parse fractional portion
    // We are here because we hit '.'
    let mut digits_after_decimal = 0u32;
    while idx < bytes.len() {
        let b = bytes[idx];
        match b {
            b'0'..=b'9' => {
                if digits_after_decimal < target_scale {
                    res = res * 10 + (b - b'0') as i64;
                    digits_after_decimal += 1;
                }
                digits_seen = true;
                idx += 1;
            }
            b'-' | b'.' => {
                 // A dash or second dot inside the fractional part is invalid
                 return Err(ParseError::InvalidTerminator);
            }
            _ => break, // Terminator reached
        }
    }

    if !digits_seen {
        return Err(ParseError::NoDigits);
    }

    let final_val = res * POWERS_OF_10[(target_scale - digits_after_decimal) as usize] * sign;
    Ok((final_val, idx))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_integers() {
        assert_eq!(parse_i64_with_precision(b"123", 0, 2), Ok((12300, 3)));
        assert_eq!(parse_i64_with_precision(b"-123", 0, 2), Ok((-12300, 4)));
        assert_eq!(parse_i64_with_precision(b"0", 0, 2), Ok((0, 1)));
    }

    #[test]
    fn test_valid_decimals() {
        assert_eq!(parse_i64_with_precision(b"1.23", 0, 2), Ok((123, 4)));
        assert_eq!(parse_i64_with_precision(b"-1.23", 0, 2), Ok((-123, 5)));
        assert_eq!(parse_i64_with_precision(b"1.2", 0, 2), Ok((120, 3)));
        assert_eq!(parse_i64_with_precision(b"1.234", 0, 2), Ok((123, 5))); // Truncate
        assert_eq!(parse_i64_with_precision(b"1.a", 0, 2), Ok((100, 2))); // 1. treated as 1.0, stops at 'a'
        assert_eq!(parse_i64_with_precision(b"1.", 0, 2), Ok((100, 2))); // 1. treated as 1.0, stops at EOF
        assert_eq!(parse_i64_with_precision(b"-.5", 0, 2), Ok((-50, 3))); // -.5 treated as -0.5
        assert_eq!(parse_i64_with_precision(b".5", 0, 2), Ok((50, 2))); // .5 treated as 0.5
    }

    #[test]
    fn test_terminators() {
        assert_eq!(parse_i64_with_precision(b"123,456", 0, 0), Ok((123, 3)));
        assert_eq!(parse_i64_with_precision(b"1.23,456", 0, 2), Ok((123, 4)));
    }

    #[test]
    fn test_errors() {
        assert_eq!(parse_i64_with_precision(b"", 0, 2), Err(ParseError::EmptyInput));
        assert_eq!(parse_i64_with_precision(b"abc", 0, 2), Err(ParseError::InvalidFirstChar));
        assert_eq!(parse_i64_with_precision(b"-", 0, 2), Err(ParseError::NoDigits));
        assert_eq!(parse_i64_with_precision(b".", 0, 2), Err(ParseError::NoDigits)); // . has no digits
        assert_eq!(parse_i64_with_precision(b"-.", 0, 2), Err(ParseError::NoDigits)); // -. has no digits
        assert_eq!(parse_i64_with_precision(b"1.2.3", 0, 2), Err(ParseError::InvalidTerminator));
        assert_eq!(parse_i64_with_precision(b"1-2", 0, 2), Err(ParseError::InvalidTerminator));
        assert_eq!(parse_i64_with_precision(b"1.2-3", 0, 2), Err(ParseError::InvalidTerminator));
    }
}