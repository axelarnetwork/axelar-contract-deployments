use solana_sdk::signature::Signature;

use crate::amplifier_api::Message;

/// Internal transport message sent from the Solana Sentinel to the Axelar
/// Verifier.
pub struct SolanaToAxelarMessage {
    pub message: Message,
    pub signature: Signature,
}
