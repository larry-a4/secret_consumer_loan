/// exponential math lib
/// TODO: Generalize this for each cToken asset
pub static scale: u128 = 100_000_000; // 10^8

/// truncate a number according to given mantissa
pub fn truncate(a: u128) -> u128 {
    a / scale
}
