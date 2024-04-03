use std::iter::repeat_with;

pub fn string(n: usize) -> String {
    repeat_with(fastrand::alphanumeric).take(n).collect()
}

pub fn bytes(n: usize) -> Vec<u8> {
    repeat_with(|| fastrand::u8(..)).take(n).collect()
}

pub fn array32() -> [u8; 32] {
    bytes(32).try_into().unwrap()
}

pub fn rand_u128() -> u128 {
    fastrand::u128(..)
}
