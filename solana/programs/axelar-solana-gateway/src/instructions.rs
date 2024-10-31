//! Instruction types

mod proxy_types;

use std::fmt::Debug;

use axelar_rkyv_encoding::hasher::merkle_tree::{MerkleProof, SolanaSyscallHasher};
use axelar_rkyv_encoding::types::{Signature, VerifierSetLeafNode};
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use itertools::Itertools;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use self::proxy_types::{ProxySignature, ProxyVerifierSetLeafNode};
use crate::axelar_auth_weighted::{RotationDelaySecs, SignerSetEpoch};
use crate::state::verifier_set_tracker::VerifierSetHash;

/// Instructions supported by the gateway program.
#[repr(u8)]
#[derive(Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum GatewayInstruction {
    /// Processes incoming batch of ApproveMessage commands from Axelar
    ///
    /// Accounts expected by this instruction:
    /// 0. [] Gateway Root Config PDA account
    /// 1. [WRITE] Gateway ExecuteData PDA account
    /// 2. [] Verifier Setr Tracker PDA account (the one that signed the
    ///    ExecuteData)
    /// 3..N [WRITE] Gateway ApprovedCommand PDA accounts. All commands needs to
    ///         be `ApproveMessages`.
    ApproveMessages,

    /// Rotate signers for the Gateway Root Config PDA account.
    ///
    /// 0. [] Gateway ExecuteData PDA account
    /// 1. [] Verifier Setr Tracker PDA account (the one that signed the
    ///    ExecuteData)
    /// 2. [WRITE, SIGNER] new uninitialized VerifierSetTracker PDA account (the
    ///    one that needs to be initialized)
    /// 3. [WRITE, SIGNER] Funding account for the new VerifierSetTracker PDA
    /// 4. [] System Program account
    /// 5. Optional: [SIGNER] `Operator` that's stored in the gateway config
    ///    PDA.
    // TODO:
    // 1. stop using the VerifierSet merkle root as a double for the Payload merkle root
    // 2. with a verified payload, send a Payload proof with the new VerifierSet merkle root and
    //    update our config.
    RotateSigners,

    /// Represents the `CallContract` Axelar event.
    ///
    /// Accounts expected by this instruction:
    /// 0. [SIGNER] Sender (origin) of the message)
    /// 1. [] Gateway Root Config PDA account
    CallContract {
        /// The name of the target blockchain.
        destination_chain: String,
        /// The address of the target contract in the destination blockchain.
        destination_contract_address: String,
        /// Contract call data.
        payload: Vec<u8>,
    },

    /// Initializes the Gateway configuration PDA account.
    ///
    /// Accounts expected by this instruction:
    /// 0. [WRITE, SIGNER] Funding account
    /// 1. [WRITE] Gateway Root Config PDA account
    /// 2. [] System Program account
    /// 3..N [WRITE] uninitialized VerifierSetTracker PDA accounts
    InitializeConfig(InitializeConfig<(VerifierSetHash, PdaBump)>),

    /// Initializes a verification session for a given Payload root.
    ///
    /// Accounts expected by this instruction:
    /// 0. [WRITE, SIGNER] Funding account
    /// 1. [] Gateway Root Config PDA account
    /// 2. [WRITE] Verification session PDA buffer account
    /// 3. [] System Program account
    InitializePayloadVerificationSession {
        /// The Merkle root for the Payload being verified.
        payload_merkle_root: [u8; 32],
        /// Buffer account PDA bump seed
        bump_seed: u8,
    },

    /// Verifies a signature within a Payload verification session
    ///
    /// Accounts expected by this instruction:
    /// 0. [] Gateway Root Config PDA account
    /// 1. [WRITE] Verification session PDA buffer account
    /// 2. [] Verifier Setr Tracker PDA account (the one that signed the
    ///    Payload's Merkle root)
    VerifySignature {
        /// The Merkle root for the Payload being verified.
        payload_merkle_root: [u8; 32],
        /// Contains all the required information for the
        verifier_set_leaf_node: ProxyVerifierSetLeafNode,
        /// The signer's proof of inclusion in the verifier set Merkle tree.
        verifier_merkle_proof: Vec<u8>,
        /// The signer's digital signature over `payload_merkle_root`
        signature: ProxySignature,
    },
}

/// Configuration parameters for initializing the axelar-solana gateway
#[derive(Debug, Clone, Default, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub struct InitializeConfig<T> {
    /// The domain separator, used as an input for hashing payloads.
    pub domain_separator: [u8; 32],
    /// initial signer sets
    /// The order is important:
    /// - first element == oldest entry
    /// - last element == latest entry
    pub initial_signer_sets: Vec<T>,
    /// the minimum delay required between rotations
    pub minimum_rotation_delay: RotationDelaySecs,
    /// The gateway operator.
    pub operator: Pubkey,
    /// how many n epochs do we consider valid
    pub previous_signers_retention: SignerSetEpoch,
}

type PdaBump = u8;
type InitializeConfigTransformation = (
    InitializeConfig<(VerifierSetHash, PdaBump)>,
    Vec<(Pubkey, PdaBump)>,
);

impl InitializeConfig<VerifierSetHash> {
    /// Convert  [`InitializeConfig`] to a type that can be submitted to the
    /// gateway by calculating the PDAs for the initial signers.
    pub fn with_verifier_set_bump(self) -> InitializeConfigTransformation {
        let (pdas, bumps): (Vec<(Pubkey, PdaBump)>, Vec<PdaBump>) = self
            .calculate_verifier_set_pdas()
            .into_iter()
            .map(|(pda, bump)| ((pda, bump), bump))
            .unzip();
        let initial_signer_sets = bumps
            .into_iter()
            .zip_eq(self.initial_signer_sets)
            .map(|(bump, set)| (set, bump))
            .collect_vec();
        (
            InitializeConfig {
                domain_separator: self.domain_separator,
                initial_signer_sets,
                minimum_rotation_delay: self.minimum_rotation_delay,
                operator: self.operator,
                previous_signers_retention: self.previous_signers_retention,
            },
            pdas,
        )
    }

    /// Calculate the PDAs and PDA bumps for the initial verifiers
    pub fn calculate_verifier_set_pdas(&self) -> Vec<(Pubkey, PdaBump)> {
        self.initial_signer_sets
            .iter()
            .map(|init_verifier_set_hash| {
                let (pda, derived_bump) =
                    crate::get_verifier_set_tracker_pda(&crate::id(), *init_verifier_set_hash);
                (pda, derived_bump)
            })
            .collect_vec()
    }
}

/// Creates a [`GatewayInstruction::ApproveMessages`] instruction.
pub fn approve_messages(
    execute_data_account: Pubkey,
    gateway_root_pda: Pubkey,
    command_accounts: &[Pubkey],
    verifier_set_tracker_pda: Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&GatewayInstruction::ApproveMessages)?;

    let mut accounts = vec![
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(execute_data_account, false),
        AccountMeta::new_readonly(verifier_set_tracker_pda, false),
    ];

    // Message accounts needs to be writable so we can set them as processed.
    accounts.extend(
        command_accounts
            .iter()
            .map(|key| AccountMeta::new(*key, false)),
    );

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`GatewayInstruction::RotateSigners`] instruction.
pub fn rotate_signers(
    execute_data_account: Pubkey,
    gateway_root_pda: Pubkey,
    operator: Option<Pubkey>,
    current_verifier_set_tracker_pda: Pubkey,
    new_verifier_set_tracker_pda: Pubkey,
    payer: Pubkey,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&GatewayInstruction::RotateSigners)?;

    let mut accounts = vec![
        AccountMeta::new(gateway_root_pda, false),
        AccountMeta::new_readonly(execute_data_account, false),
        AccountMeta::new_readonly(current_verifier_set_tracker_pda, false),
        AccountMeta::new(new_verifier_set_tracker_pda, false),
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    if let Some(operator) = operator {
        accounts.push(AccountMeta::new(operator, true));
    }

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`CallContract`] instruction.
pub fn call_contract(
    gateway_root_pda: Pubkey,
    sender: Pubkey,
    destination_chain: String,
    destination_contract_address: String,
    payload: Vec<u8>,
) -> Result<Instruction, ProgramError> {
    let data = to_vec(&GatewayInstruction::CallContract {
        destination_chain,
        destination_contract_address,
        payload,
    })?;

    let accounts = vec![
        AccountMeta::new_readonly(sender, true),
        AccountMeta::new_readonly(gateway_root_pda, false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`GatewayInstruction::InitializeConfig`] instruction.
pub fn initialize_config(
    payer: Pubkey,
    config: InitializeConfig<VerifierSetHash>,
    gateway_config_pda: Pubkey,
) -> Result<Instruction, ProgramError> {
    let mut accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new(gateway_config_pda, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];
    let (config, with_verifier_set_bump) = config.with_verifier_set_bump();
    with_verifier_set_bump.into_iter().for_each(|(pda, _)| {
        accounts.push(AccountMeta {
            pubkey: pda,
            is_signer: false,
            is_writable: true,
        })
    });

    let data = to_vec(&GatewayInstruction::InitializeConfig(config))?;
    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`GatewayInstruction::InitializePayloadVerificationSession`]
/// instruction.
pub fn initialize_payload_verification_session(
    payer: Pubkey,
    gateway_config_pda: Pubkey,
    payload_merkle_root: [u8; 32],
) -> Result<Instruction, ProgramError> {
    let (verification_session_pda, bump_seed) =
        crate::get_signature_verification_pda(&gateway_config_pda, &payload_merkle_root);

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(gateway_config_pda, false),
        AccountMeta::new(verification_session_pda, false),
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
    ];

    let data = to_vec(&GatewayInstruction::InitializePayloadVerificationSession {
        payload_merkle_root,
        bump_seed,
    })?;

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates a [`GatewayInstruction::VerifySignature`] instruction.
pub fn verify_signature(
    gateway_config_pda: Pubkey,
    verifier_set_tracker_pda: Pubkey,
    payload_merkle_root: [u8; 32],
    verifier_set_leaf_node: VerifierSetLeafNode<SolanaSyscallHasher>,
    verifier_merkle_proof: MerkleProof<SolanaSyscallHasher>,
    signature: Signature,
) -> Result<Instruction, ProgramError> {
    let (verification_session_pda, _bump) =
        crate::get_signature_verification_pda(&gateway_config_pda, &payload_merkle_root);

    let accounts = vec![
        AccountMeta::new_readonly(gateway_config_pda, false),
        AccountMeta::new(verification_session_pda, false),
        AccountMeta::new_readonly(verifier_set_tracker_pda, false),
    ];

    let data = to_vec(&GatewayInstruction::VerifySignature {
        payload_merkle_root,
        verifier_set_leaf_node: verifier_set_leaf_node.into(),
        verifier_merkle_proof: verifier_merkle_proof.to_bytes(),
        signature: signature.into(),
    })?;

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

#[cfg(test)]
pub mod tests {

    use borsh::from_slice;
    use solana_sdk::signature::Keypair;
    use solana_sdk::signer::Signer;

    use super::*;
    use crate::state::GatewayConfig;

    #[test]
    fn round_trip_queue() {
        let original = GatewayInstruction::ApproveMessages {};
        let serialized = to_vec(&original).unwrap();
        let deserialized = from_slice::<GatewayInstruction>(&serialized).unwrap();
        assert_eq!(deserialized, original);
    }

    #[test]
    fn round_trip_queue_function() {
        let execute_data_account = Keypair::new().pubkey();
        let _payer = Keypair::new().pubkey();
        let (gateway_root_pda, _) = GatewayConfig::pda();
        let approved_message_accounts = vec![Keypair::new().pubkey()];
        let verifier_set_tracker_pda = Pubkey::new_unique();
        let instruction = approve_messages(
            execute_data_account,
            gateway_root_pda,
            &approved_message_accounts,
            verifier_set_tracker_pda,
        )
        .expect("valid instruction construction");
        let deserialized = from_slice(&instruction.data).expect("deserialized valid instruction");
        assert!(matches!(
            deserialized,
            GatewayInstruction::ApproveMessages {}
        ));
    }

    #[test]
    fn round_trip_call_contract() {
        let destination_chain = "ethereum".to_owned();
        let destination_contract_address = "2F43DDFf564Fb260dbD783D55fc6E4c70Be18862".to_owned();
        let payload = vec![5; 100];

        let instruction = GatewayInstruction::CallContract {
            destination_chain: destination_chain.to_owned(),
            destination_contract_address,
            payload,
        };

        let serialized = to_vec(&instruction).expect("call contract to be serialized");
        let deserialized = from_slice(&serialized).expect("call contract to be deserialized");

        assert_eq!(instruction, deserialized);
    }

    #[test]
    fn round_trip_call_contract_function() {
        let sender = Keypair::new().pubkey();
        let destination_chain = "ethereum".to_owned();
        let destination_contract_address = "2F43DDFf564Fb260dbD783D55fc6E4c70Be18862".to_owned();
        let payload = vec![5; 100];

        let instruction = call_contract(
            crate::id(),
            sender,
            destination_chain.clone(),
            destination_contract_address.clone(),
            payload.clone(),
        )
        .expect("valid instruction construction");

        let deserialized = from_slice(&instruction.data).expect("deserialize valid instruction");

        match deserialized {
            GatewayInstruction::CallContract {
                destination_chain: deserialized_destination_chain,
                destination_contract_address: deserialized_destination_contract_address,
                payload: deserialized_payload,
            } => {
                assert_eq!(destination_chain, deserialized_destination_chain);
                assert_eq!(
                    destination_contract_address,
                    deserialized_destination_contract_address
                );
                assert_eq!(payload.as_slice(), deserialized_payload.as_slice());
            }
            _ => panic!("Wrong instruction"),
        };
    }
}
