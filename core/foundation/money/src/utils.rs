use crate::MoneyError;
use std::cmp::Ordering;

pub(super) fn normalize(mut coefficient: i128, mut scale: u8) -> (i128, u8) {
    while scale > 0 && coefficient % 10 == 0 {
        coefficient /= 10;
        scale -= 1;
    }
    (coefficient, scale)
}

pub(super) fn pow10(power: u8) -> Result<i128, MoneyError> {
    10_i128
        .checked_pow(power.into())
        .ok_or(MoneyError::ArithmeticOverflow)
}

pub(super) fn scale_coeff(value: i128, from: u8, to: u8) -> Result<i128, MoneyError> {
    value
        .checked_mul(pow10(to - from)?)
        .ok_or(MoneyError::ArithmeticOverflow)
}

pub(super) fn compare_magnitude(
    left: u128,
    left_scale: u8,
    right: u128,
    right_scale: u8,
) -> Ordering {
    let left_digits = left.to_string();
    let right_digits = right.to_string();
    let left_order = left_digits.len() as i16 - i16::from(left_scale);
    let right_order = right_digits.len() as i16 - i16::from(right_scale);
    match left_order.cmp(&right_order) {
        Ordering::Equal => {
            let width = left_digits.len().max(right_digits.len());
            let left_aligned = format!("{left_digits:0<width$}");
            let right_aligned = format!("{right_digits:0<width$}");
            left_aligned.cmp(&right_aligned)
        }
        order => order,
    }
}
