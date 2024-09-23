// NOTE: there are issues with using `multisig-prover` as a dependency (build
// breaks). Thats why the types are re-defined here
use axelar_wasm_std::nonempty::Uint64;
use axelar_wasm_std::MajorityThreshold;
use cosmwasm_schema::cw_serde;
use multisig::key::KeyType;
use router_api::{CrossChainId, Message};

#[cw_serde]
pub(crate) enum MultisigProverExecuteMsg {
    // Start building a proof that includes specified messages
    // Queries the gateway for actual message contents
    ConstructProof(Vec<CrossChainId>),
    UpdateVerifierSet,

    ConfirmVerifierSet,
    // Updates the signing threshold. The threshold currently in use does not change.
    // The verifier set must be updated and confirmed for the change to take effect.
    UpdateSigningThreshold {
        new_signing_threshold: MajorityThreshold,
    },
    UpdateAdmin {
        new_admin_address: String,
    },
}

#[cw_serde]
pub(crate) enum QueryMsg {
    Proof {
        multisig_session_id: Uint64,
    },

    /// Returns a `VerifierSetResponse` with the current verifier set id and the
    /// verifier set itself.
    CurrentVerifierSet,

    NextVerifierSet,
}

#[cw_serde]
pub(crate) struct GetProofResponse {
    pub(crate) multisig_session_id: Uint64,
    pub(crate) message_ids: Vec<CrossChainId>,
    pub(crate) payload: Payload,
    pub(crate) status: ProofStatus,
}

#[cw_serde]
pub(crate) struct VerifierSetResponse {
    pub(crate) id: String,
    pub(crate) verifier_set: multisig::verifier_set::VerifierSet,
}

#[cw_serde]
pub(crate) enum Payload {
    Messages(Vec<Message>),
}

#[cw_serde]
pub(crate) enum ProofStatus {
    Pending,
    Completed { execute_data: String }, // encoded data and proof sent to destination gateway
}

#[cw_serde]
pub(crate) struct InstantiateMsg {
    /// Address that can execute all messages that either have unrestricted or
    /// admin permission level, such as Updateverifier set. Should be set to
    /// a trusted address that can react to unexpected interruptions to the
    /// contract's operation.
    pub(crate) admin_address: String,
    /// Address that can call all messages of unrestricted, admin and governance
    /// permission level, such as UpdateSigningThreshold. This address can
    /// execute messages that bypasses verification checks to rescue the
    /// contract if it got into an otherwise unrecoverable state due to external
    /// forces. On mainnet, it should match the address of the Cosmos
    /// governance module.
    pub(crate) governance_address: String,
    /// Address of the gateway on axelar associated with the destination chain.
    /// For example, if this prover is creating proofs to be relayed to
    /// Ethereum, this is the address of the gateway on Axelar for Ethereum.
    pub(crate) gateway_address: String,
    /// Address of the multisig contract on axelar.
    pub(crate) multisig_address: String,
    /// Address of the coordinator contract on axelar.
    pub(crate) coordinator_address: String,
    /// Address of the service registry contract on axelar.
    pub(crate) service_registry_address: String,
    /// Address of the voting verifier contract on axelar associated with the
    /// destination chain. For example, if this prover is creating proofs to
    /// be relayed to Ethereum, this is the address of the voting verifier for
    /// Ethereum.
    pub(crate) voting_verifier_address: String,
    /// Threshold of weighted signatures required for signing to be considered
    /// complete
    pub(crate) signing_threshold: MajorityThreshold,
    /// Name of service in the service registry for which verifiers are
    /// registered.
    pub(crate) service_name: String,
    /// Name of chain for which this prover contract creates proofs.
    pub(crate) chain_name: String,
    /// Maximum tolerable difference between currently active verifier set and
    /// registered verifier set. The verifier set registered in the service
    /// registry must be different by more than this number of verifiers
    /// before calling UpdateVerifierSet. For example, if this is set to 1,
    /// UpdateVerifierSet will fail unless the registered verifier set and
    /// active verifier set differ by more than 1.
    pub(crate) verifier_set_diff_threshold: u32,
    /// Type of encoding to use for signed payload. Blockchains can encode their
    /// execution payloads in various ways (ABI, BCS, etc). This defines the
    /// specific encoding type to use for this prover, which should correspond
    /// to the encoding type used by the gateway deployed on the destination
    /// chain.
    pub(crate) encoder: String,
    /// Public key type verifiers use for signing payload. Different blockchains
    /// support different cryptographic signature algorithms (ECDSA, Ed25519,
    /// etc). This defines the specific signature algorithm to use for this
    /// prover, which should correspond to the signature algorithm used by the
    /// gateway deployed on the destination chain. The multisig contract
    /// supports multiple public keys per verifier (each a different type of
    /// key), and this parameter controls which registered public key to use
    /// for signing for each verifier registered to the destination chain.
    pub(crate) key_type: KeyType,
    /// An opaque value created to distinguish distinct chains that the external
    /// gateway should be initialized with. Value must be a String in hex
    /// format without `0x`, e.g.
    /// "598ba04d225cec385d1ce3cf3c9a076af803aa5c614bc0e0d176f04ac8d28f55".
    pub(crate) domain_separator: String,
}
