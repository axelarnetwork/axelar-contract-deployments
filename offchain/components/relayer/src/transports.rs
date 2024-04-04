use crate::amplifier_api::Message;
use solana_sdk::signature::Signature;

/// Internal transport message sent from the Solana Sentinel to the Axelar Verifier.
pub struct SolanaToAxelarMessage {
    pub message: Message,
    pub signature: Signature,
}
