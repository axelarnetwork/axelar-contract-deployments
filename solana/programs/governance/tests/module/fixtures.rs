use solana_sdk::signature::Keypair;

pub(crate) type Hash = [u8; 32];

pub(crate) const SOURCE_CHAIN_ADDRESS: &str = "0x742d35Cc6634C0532925a3b844Bc454e4438f44e";
pub(crate) const SOURCE_CHAIN_ADDRESS_KECCAK_HASH: Hash = [
    44, 13, 201, 135, 234, 78, 43, 85, 203, 8, 2, 77, 27, 20, 37, 189, 230, 196, 123, 127, 243, 74,
    121, 192, 77, 135, 45, 39, 42, 127, 115, 28,
];

pub(crate) const SOURCE_CHAIN_NAME: &str = "ethereum";
pub(crate) const SOURCE_CHAIN_NAME_KECCAK_HASH: Hash = [
    84, 17, 17, 36, 139, 69, 183, 168, 220, 63, 85, 121, 246, 48, 231, 76, 176, 20, 86, 234, 106,
    192, 103, 211, 244, 215, 147, 36, 90, 37, 81, 85,
];

pub(crate) const MINIMUM_PROPOSAL_DELAY: u32 = 3600;
pub(crate) const OPERATOR_KEYPAIR_BYTES: [u8; 64] = [
    113, 28, 103, 223, 157, 114, 180, 136, 89, 47, 112, 200, 106, 32, 165, 141, 188, 246, 97, 41,
    200, 53, 66, 28, 174, 147, 175, 150, 49, 150, 60, 233, 154, 130, 153, 59, 69, 122, 88, 207, 16,
    151, 169, 11, 101, 245, 137, 81, 240, 206, 98, 29, 158, 91, 174, 161, 50, 15, 150, 167, 145,
    101, 235, 222,
];

pub(crate) fn operator_keypair() -> Keypair {
    Keypair::from_bytes(&OPERATOR_KEYPAIR_BYTES).unwrap()
}

pub(crate) const PROPOSAL_TARGET_ADDRESS: [u8; 32] = [
    142, 58, 218, 11, 201, 166, 92, 115, 55, 67, 99, 101, 88, 152, 241, 122, 209, 4, 234, 152, 34,
    211, 123, 232, 217, 84, 231, 43, 45, 203, 10, 54,
];
