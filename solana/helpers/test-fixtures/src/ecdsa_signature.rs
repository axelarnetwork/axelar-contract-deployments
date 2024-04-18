use libsecp256k1::{sign, Message, PublicKey, RecoveryId, SecretKey, Signature};
use solana_program::keccak::hash;

#[derive(Debug)]
pub struct TestSignature {
    pub signature: Signature,
    pub recovery_id: RecoveryId,
    pub secret_key: SecretKey,
    pub public_key: PublicKey,
}

pub fn create_random_signature(message_to_sign: &[u8]) -> TestSignature {
    let secret_key = SecretKey::random(&mut rand_core::OsRng);
    let message_hash = hash(message_to_sign).to_bytes();
    let message = Message::parse(&message_hash);
    let (signature, recovery_id) = sign(&message, &secret_key);
    let public_key = PublicKey::from_secret_key(&secret_key);

    TestSignature {
        signature,
        recovery_id,
        secret_key,
        public_key,
    }
}

#[cfg(test)]
mod tests {
    use libsecp256k1::{recover, verify};

    use super::*;

    #[test]
    fn signature() -> anyhow::Result<()> {
        let message_to_sign = b"Hello, World!";
        let TestSignature {
            signature,
            recovery_id,
            secret_key,
            public_key,
        } = create_random_signature(message_to_sign);

        // Check: Message signing
        let message_hash = solana_program::keccak::hash(message_to_sign).to_bytes();
        let message = Message::parse(&message_hash);
        assert!(verify(&message, &signature, &public_key));

        // Check: Public key can be recovered

        let recovered = recover(&message, &signature, &recovery_id)?;
        assert_eq!(recovered, public_key);

        // Check: signature was produced by signer
        let (expected_signature, expected_recovery_id) = sign(&message, &secret_key);
        assert_eq!(expected_signature, signature);
        assert_eq!(expected_recovery_id, recovery_id);

        Ok(())
    }
}
