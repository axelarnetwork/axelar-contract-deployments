use axelar_rkyv_encoding::test_fixtures::signing_key::{random_keypair, TestSigningKey};
use axelar_rkyv_encoding::types::{PublicKey, U128};

#[derive(Clone)]
pub struct TestSigner {
    pub public_key: PublicKey,
    pub secret_key: TestSigningKey,
    pub weight: U128,
}

pub fn create_signer_with_weight(weight: u128) -> TestSigner {
    let (secret_key, public_key) = random_keypair();

    TestSigner {
        public_key,
        secret_key,
        weight: weight.into(),
    }
}
