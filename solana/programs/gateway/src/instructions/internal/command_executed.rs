use super::*;

fn set(acc: &mut Account<'_, State>, value: bool) {
    acc.value = value;
}
