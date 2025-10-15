use axelar_solana_encoding::hasher::SolanaSyscallHasher;
use axelar_solana_encoding::types::execute_data::MerkleisedMessage;
use axelar_solana_encoding::{rs_merkle, LeafHash};
use core::str::FromStr;
use event_cpi_macros::{emit_cpi, event_cpi_accounts};
use program_utils::{
    pda::{BytemuckedPda, ValidPDA},
    validate_system_account_key,
};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use super::Processor;
use crate::error::GatewayError;
use crate::events::MessageApprovedEvent;
use crate::state::incoming_message::{command_id, IncomingMessage, MessageStatus};
use crate::state::signature_verification_pda::SignatureVerificationSessionData;
use crate::state::GatewayConfig;
use crate::{
    assert_valid_gateway_root_pda, assert_valid_incoming_message_pda,
    assert_valid_signature_verification_pda, get_incoming_message_pda,
    get_validate_message_signing_pda, seed_prefixes,
};

impl Processor {
    /// Approves an array of messages, signed by the Axelar signers.
    /// reference implementation: `https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/2eaf5199ee8ccc5eb1d8353c0dd7592feff0eb5c/contracts/gateway/AxelarAmplifierGateway.sol#L78-L84`
    /// # Errors
    ///
    /// Returns an error if:
    /// * Account Validation:
    ///   * Account iteration fails when extracting accounts
    ///   * Gateway Root PDA is not initialized
    ///   * Verification session PDA is not initialized
    ///   * Incoming message PDA is already initialized
    ///
    /// * Data Access and Serialization:
    ///   * Failed to borrow verification session or incoming message account data
    ///   * Verification session or incoming message data has invalid byte length
    ///
    /// * Verification Failures:
    ///   * Signature verification PDA validation fails
    ///   * Signature verification session is not valid
    ///   * Merkle proof is invalid
    ///   * Leaf node is not part of the provided merkle root
    ///
    /// * Message Processing:
    ///   * Failed to initialize PDA for incoming message
    ///   * Destination address is invalid and cannot be converted to a `Pubkey`
    ///
    /// # Panics
    ///
    /// This function will panic if:
    /// * Converting `IncomingMessage::LEN` to u64 overflows.
    #[allow(clippy::too_many_lines)]
    pub fn process_approve_message(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        merkleised_message: MerkleisedMessage,
        payload_merkle_root: [u8; 32],
    ) -> ProgramResult {
        // Accounts
        let accounts_iter = &mut accounts.iter();
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let funder = next_account_info(accounts_iter)?;
        let verification_session_account = next_account_info(accounts_iter)?;
        let incoming_message_pda = next_account_info(accounts_iter)?;
        let system_program = next_account_info(accounts_iter)?;
        event_cpi_accounts!(accounts_iter);

        validate_system_account_key(system_program.key)?;

        // Check: Gateway Root PDA is initialized and valid.
        gateway_root_pda.check_initialized_pda_without_deserialization(&crate::ID)?;
        let gateway_data = gateway_root_pda.try_borrow_data()?;
        let gateway_config =
            GatewayConfig::read(&gateway_data).ok_or(GatewayError::BytemuckDataLenInvalid)?;
        assert_valid_gateway_root_pda(gateway_config.bump, gateway_root_pda.key)?;

        // Check: Verification session PDA is initialized.
        verification_session_account.check_initialized_pda_without_deserialization(program_id)?;
        let data = verification_session_account.try_borrow_data()?;
        let session = SignatureVerificationSessionData::read(&data)
            .ok_or(GatewayError::BytemuckDataLenInvalid)?;
        assert_valid_signature_verification_pda(
            &payload_merkle_root,
            &session.signature_verification.signing_verifier_set_hash,
            session.bump,
            verification_session_account.key,
        )?;

        // Check: the incoming message PDA already approved
        incoming_message_pda
            .check_uninitialized_pda()
            .map_err(|_err| GatewayError::MessageAlreadyInitialised)?;

        // Check: signature verification session is complete
        if !session.signature_verification.is_valid() {
            return Err(GatewayError::SigningSessionNotValid.into());
        }

        // Check: message domain separator matches the gateway's domain separator
        if merkleised_message.leaf.domain_separator != gateway_config.domain_separator {
            return Err(GatewayError::InvalidDomainSeparator.into());
        }

        let leaf_hash = merkleised_message.leaf.hash::<SolanaSyscallHasher>();
        let message_hash = merkleised_message
            .leaf
            .message
            .hash::<SolanaSyscallHasher>();
        let proof =
            rs_merkle::MerkleProof::<SolanaSyscallHasher>::from_bytes(&merkleised_message.proof)
                .map_err(|_err| GatewayError::InvalidMerkleProof)?;

        // Check: leaf node is part of the payload merkle root
        if !proof.verify(
            payload_merkle_root,
            &[merkleised_message.leaf.position.into()],
            &[leaf_hash],
            merkleised_message.leaf.set_size.into(),
        ) {
            return Err(GatewayError::LeafNodeNotPartOfMerkleRoot.into());
        }

        // create a PDA where we write the message metadata contents
        let message = merkleised_message.leaf.message;
        let cc_id = message.cc_id;
        let command_id = command_id(&cc_id.chain, &cc_id.id);

        let (_, incoming_message_pda_bump) = get_incoming_message_pda(&command_id);
        assert_valid_incoming_message_pda(
            &command_id,
            incoming_message_pda_bump,
            incoming_message_pda.key,
        )?;

        let seeds = &[
            seed_prefixes::INCOMING_MESSAGE_SEED,
            &command_id,
            &[incoming_message_pda_bump],
        ];
        program_utils::pda::init_pda_raw(
            funder,
            incoming_message_pda,
            program_id,
            system_program,
            IncomingMessage::LEN.try_into().map_err(|_err| {
                solana_program::msg!("unexpected u64 overflow in struct size");
                ProgramError::ArithmeticOverflow
            })?,
            seeds,
        )?;

        let destination_address =
            Pubkey::from_str(&message.destination_address).map_err(|_err| {
                solana_program::msg!("Invalid destination address");
                GatewayError::InvalidDestinationAddress
            })?;
        let (_, signing_pda_bump) =
            get_validate_message_signing_pda(destination_address, command_id);

        // Persist a new incoming message with "in progress" status in the PDA data.
        let mut data = incoming_message_pda.try_borrow_mut_data()?;
        let incoming_message_data =
            IncomingMessage::read_mut(&mut data).ok_or(GatewayError::BytemuckDataLenInvalid)?;
        *incoming_message_data = IncomingMessage::new(
            incoming_message_pda_bump,
            signing_pda_bump,
            MessageStatus::approved(),
            message_hash,
            message.payload_hash,
        );

        emit_cpi!(MessageApprovedEvent {
            command_id,
            destination_address,
            payload_hash: message.payload_hash,
            source_chain: cc_id.chain,
            cc_id: cc_id.id,
            source_address: message.source_address,
            destination_chain: message.destination_chain,
        });

        Ok(())
    }
}
