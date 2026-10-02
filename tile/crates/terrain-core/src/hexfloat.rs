//! Exact text form of `f64` values, in the format of Python's `float.hex()`
//! (`[-]0x1.<13 hex digits>p<±exp>`, `0x0.0p+0` for zero). The golden
//! fixtures use it so that values survive JSON round trips bit for bit.

/// Format a finite `f64` like Python's `float.hex()`. Non-finite values give
/// `nan`, `inf` or `-inf` (which [`parse`] rejects).
pub fn format(v: f64) -> String {
    if v.is_nan() {
        return "nan".into();
    }
    if v.is_infinite() {
        return if v > 0.0 { "inf".into() } else { "-inf".into() };
    }
    let bits = v.to_bits();
    let sign = if bits >> 63 == 1 { "-" } else { "" };
    let exp = ((bits >> 52) & 0x7ff) as i32;
    let mant = bits & ((1u64 << 52) - 1);
    if exp == 0 && mant == 0 {
        return format!("{sign}0x0.0p+0");
    }
    let (lead, e) = if exp == 0 {
        (0, -1022) // subnormal
    } else {
        (1, exp - 1023)
    };
    let e_sign = if e < 0 { '-' } else { '+' };
    format!("{sign}0x{lead}.{mant:013x}p{e_sign}{}", e.unsigned_abs())
}

/// Parse `[-]0x<hex>[.<hex>]p<±exp>`. `None` on anything malformed, or when
/// the mantissa has more digits than an `f64` can hold exactly.
pub fn parse(s: &str) -> Option<f64> {
    let (neg, s) = s.strip_prefix('-').map_or((false, s), |r| (true, r));
    let s = s.strip_prefix("0x")?;
    let (mant, exp) = s.split_once('p')?;
    let exp: i32 = exp.parse().ok()?;
    let (int, frac) = mant.split_once('.').unwrap_or((mant, ""));
    if int.is_empty() || int.len() > 1 || frac.len() > 13 {
        return None;
    }
    // Integer mantissa of `int.frac` scaled by 16^frac.len(): at most 1 + 52
    // bits, so it is exact in u64 and in f64.
    let mut m: u64 = 0;
    for c in int.chars().chain(frac.chars()) {
        m = m * 16 + c.to_digit(16)? as u64;
    }
    let shift = exp.checked_sub(4 * frac.len() as i32)?;
    let v = m as f64 * pow2(shift);
    Some(if neg { -v } else { v })
}

/// `2^e` exactly (as long as the result is a normal or subnormal `f64`).
fn pow2(e: i32) -> f64 {
    // Split so that each factor is a normal power of two: exact products.
    let mut v = 1.0f64;
    let mut e = e;
    while e > 1000 {
        v *= f64::from_bits(((1000 + 1023) as u64) << 52);
        e -= 1000;
    }
    while e < -1000 {
        v *= f64::from_bits(((-1000 + 1023) as u64) << 52);
        e += 1000;
    }
    v * f64::from_bits(((e + 1023) as u64) << 52)
}

#[cfg(test)]
mod tests {
    use super::{format, parse};

    #[test]
    fn matches_python_float_hex() {
        // Strings from Python 3: float.hex(x).
        for (s, v) in [
            ("0x0.0p+0", 0.0),
            ("0x1.0640000000000p+7", 131.125),
            ("0x1.f89108334ec72p+4", 31.535408211155705),
            ("-0x1.0354dc69d6a27p-3", -0.1266267032431909),
            ("0x1.0000000000000p+0", 1.0),
            ("0x1.fffffffffffffp+1023", f64::MAX),
            ("0x1.0000000000000p-1022", f64::MIN_POSITIVE),
        ] {
            assert_eq!(format(v), s);
            assert_eq!(parse(s).map(f64::to_bits), Some(v.to_bits()), "{s}");
        }
        assert_eq!(format(-0.0), "-0x0.0p+0");
        assert_eq!(
            parse("-0x0.0p+0").map(f64::to_bits),
            Some((-0.0f64).to_bits())
        );
    }

    #[test]
    fn round_trips_every_bit_pattern_family() {
        let mut x = 0x9e37_79b9_7f4a_7c15u64;
        for _ in 0..100_000 {
            x ^= x << 13;
            x ^= x >> 7;
            x ^= x << 17;
            let v = f64::from_bits(x);
            if !v.is_finite() {
                continue;
            }
            assert_eq!(
                parse(&format(v)).map(f64::to_bits),
                Some(v.to_bits()),
                "{v:e}"
            );
        }
    }

    #[test]
    fn malformed_is_none() {
        for s in [
            "",
            "0x",
            "1.0p+0",
            "0x1.0",
            "0x1.0pz",
            "0xg.0p+0",
            "0x12.0p+0",
            "nan",
            "inf",
            "0x1.00000000000000p+0",
            "0x1.0p+99999999999",
        ] {
            assert_eq!(parse(s), None, "{s}");
        }
    }
}
