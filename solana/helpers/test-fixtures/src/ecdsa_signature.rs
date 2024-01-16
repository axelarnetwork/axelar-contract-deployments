use k256::ecdsa::{RecoveryId, Signature, SigningKey, VerifyingKey};
use sha3::{Digest, Keccak256};
use signature::Verifier;

#[derive(Debug)]
pub struct TestSignature {
    pub signature: Signature,
    pub recovery_id: RecoveryId,
    pub signing_key: SigningKey,
    pub verifying_key: VerifyingKey,
}

pub fn create_random_signature(message: &[u8]) -> TestSignature {
    let signing_key = SigningKey::random(&mut rand_core::OsRng);
    let digest = Keccak256::new_with_prefix(message);
    let (signature, recovery_id) = signing_key
        .sign_digest_recoverable(digest.clone())
        .expect("failed to sign message");
    let verifying_key = VerifyingKey::recover_from_msg(message, &signature, recovery_id)
        .expect("failed to recover public key");

    TestSignature {
        signature,
        recovery_id,
        signing_key,
        verifying_key,
    }
}

#[test]
fn signature() -> anyhow::Result<()> {
    let message = b"Hello, World!";
    let TestSignature {
        signature,
        recovery_id,
        signing_key,
        verifying_key,
    } = create_random_signature(message);

    // Check: Message signing
    assert!(verifying_key.verify(message, &signature).is_ok());

    // Check: Public key can be recovered
    let recovered = VerifyingKey::recover_from_msg(message, &signature, recovery_id)?;
    assert_eq!(recovered, verifying_key);

    // Check: signature was produced by signer
    let digest = Keccak256::new_with_prefix(message);
    let (expected_signature, expected_recovery_id) =
        signing_key.sign_digest_recoverable(digest.clone())?;
    assert_eq!(expected_signature, signature);
    assert_eq!(expected_recovery_id, recovery_id);

    Ok(())
}
