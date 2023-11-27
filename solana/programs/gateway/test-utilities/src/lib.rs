use rand::distributions::{Alphanumeric, Standard};
use rand::Rng;

// Helper function to generate a random byte array
pub fn rand_array<const SIZE: usize>() -> [u8; SIZE] {
    let rng = rand::thread_rng();
    rng.sample_iter(Standard)
        .take(SIZE)
        .collect::<Vec<u8>>()
        .try_into()
        .unwrap()
}

// Function to generate a random alphanumeric string
pub fn rand_str(len: usize) -> String {
    let rng = rand::thread_rng();
    rng.sample_iter(&Alphanumeric)
        .take(len)
        .map(char::from)
        .collect()
}
