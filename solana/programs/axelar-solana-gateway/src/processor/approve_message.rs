use std::mem::size_of;
use std::str::FromStr;

use axelar_solana_encoding::hasher::SolanaSyscallHasher;
use axelar_solana_encoding::types::execute_data::MerkleisedMessage;
use axelar_solana_encoding::{rs_merkle, LeafHash};
use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::log::sol_log_data;
use solana_program::pubkey::Pubkey;

use super::Processor;
use crate::error::GatewayError;
use crate::state::incoming_message::{command_id, IncomingMessage, IncomingMessageWrapper};
use crate::state::signature_verification_pda::SignatureVerificationSessionData;
use crate::state::BytemuckedPda;
use crate::{
    assert_valid_incoming_message_pda, assert_valid_signature_verification_pda, event_prefixes,
    get_incoming_message_pda, get_validate_message_signing_pda, seed_prefixes,
};

impl Processor {
    /// Approves an array of messages, signed by the Axelar signers.
    /// reference implementation: https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/2eaf5199ee8ccc5eb1d8353c0dd7592feff0eb5c/contracts/gateway/AxelarAmplifierGateway.sol#L78-L84
    pub fn process_approve_message(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        message: MerkleisedMessage,
        payload_merkle_root: [u8; 32],
    ) -> ProgramResult {
        // Accounts
        let accounts_iter = &mut accounts.iter();
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let funder = next_account_info(accounts_iter)?;
        let verification_session_account = next_account_info(accounts_iter)?;
        let incoming_message_pda = next_account_info(accounts_iter)?;
        let system_program = next_account_info(accounts_iter)?;

        // Check: Gateway Root PDA is initialized.
        // No need to check the bump because that would already be implied by a valid `verification_session_account`
        gateway_root_pda.check_initialized_pda_without_deserialization(program_id)?;

        // Check: Verification session PDA is initialized.
        verification_session_account.check_initialized_pda_without_deserialization(program_id)?;
        let data = verification_session_account.try_borrow_data()?;
        let session = SignatureVerificationSessionData::read(&data)?;
        assert_valid_signature_verification_pda(
            gateway_root_pda.key,
            &payload_merkle_root,
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

        let leaf_hash = message.leaf.hash::<SolanaSyscallHasher>();
        let message_hash = message.leaf.message.hash::<SolanaSyscallHasher>();
        let proof = rs_merkle::MerkleProof::<SolanaSyscallHasher>::from_bytes(&message.proof)
            .map_err(|_err| GatewayError::InvalidMerkleProof)?;

        // Check: leaf node is part of the payload merkle root
        if !proof.verify(
            payload_merkle_root,
            &[message.leaf.position as usize],
            &[leaf_hash],
            message.leaf.set_size as usize,
        ) {
            return Err(GatewayError::LeafNodeNotPartOfMerkleRoot.into());
        }

        // crate a PDA where we write the message metadata contents
        let message = message.leaf.message;
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
        program_utils::init_pda_raw(
            funder,
            incoming_message_pda,
            program_id,
            system_program,
            size_of::<IncomingMessageWrapper>() as u64,
            seeds,
        )?;
        let mut data = incoming_message_pda.try_borrow_mut_data()?;
        let incoming_message_data = IncomingMessageWrapper::read_mut(&mut data)?;

        let destination_address =
            Pubkey::from_str(&message.destination_address).map_err(|_err| {
                solana_program::msg!("Invalid destination address");
                GatewayError::InvalidDestinationAddress
            })?;

        let (_, signing_pda_bump) =
            get_validate_message_signing_pda(destination_address, command_id);

        incoming_message_data.bump = incoming_message_pda_bump;
        incoming_message_data.signing_pda_bump = signing_pda_bump;
        incoming_message_data.message = IncomingMessage::new(message_hash);

        // Emit an event
        sol_log_data(&[
            event_prefixes::MESSAGE_APPROVED,
            &command_id,
            &destination_address.to_bytes(),
            &message.payload_hash,
            cc_id.chain.as_bytes(),
            cc_id.id.as_bytes(),
            message.source_address.as_bytes(),
            message.destination_chain.as_bytes(),
        ]);

        Ok(())
    }
}
