use axelar_solana_encoding::hasher::SolanaSyscallHasher;
use axelar_solana_encoding::types::execute_data::MerkleisedMessage;
use axelar_solana_encoding::{rs_merkle, LeafHash};
use program_utils::{init_pda_raw, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use super::Processor;
use crate::events::{GatewayEvent, MessageApproved};
use crate::state::incoming_message::{command_id, IncomingMessage, IncomingMessageWrapper};
use crate::state::signature_verification_pda::SignatureVerificationSessionData;
use crate::state::GatewayConfig;
use crate::{assert_valid_incoming_message_pda, seed_prefixes};

impl Processor {
    /// Approves an array of messages, signed by the Axelar signers.
    /// reference implementation: https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/2eaf5199ee8ccc5eb1d8353c0dd7592feff0eb5c/contracts/gateway/AxelarAmplifierGateway.sol#L78-L84
    pub fn process_approve_message(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
        message: MerkleisedMessage,
        payload_merkle_root: [u8; 32],
        incoming_message_pda_bump: u8,
    ) -> ProgramResult {
        // Accounts
        let accounts_iter = &mut accounts.iter();
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let funder = next_account_info(accounts_iter)?;
        let verification_session_account = next_account_info(accounts_iter)?;
        let incoming_message_pda = next_account_info(accounts_iter)?;
        let system_program = next_account_info(accounts_iter)?;

        // Check: the incoming message PDA already approved
        if incoming_message_pda.check_uninitialized_pda().is_err() {
            solana_program::msg!("Message already approved");
            return Ok(());
        }

        // Check: Gateway Root PDA is initialized.
        let _gateway_config =
            gateway_root_pda.check_initialized_pda::<GatewayConfig>(program_id)?;

        // Check: signature verification session is complete
        let mut data = verification_session_account.try_borrow_mut_data()?;
        let data_bytes: &mut [u8; SignatureVerificationSessionData::LEN] =
            (*data).try_into().map_err(|_err| {
                solana_program::msg!("session account data is corrupt");
                ProgramError::InvalidAccountData
            })?;
        let session = bytemuck::cast_mut::<_, SignatureVerificationSessionData>(data_bytes);
        if !session.signature_verification.is_valid() {
            solana_program::msg!("signing session is not complete");
            return Err(ProgramError::InvalidAccountData);
        }

        // Check: Verification PDA can be derived from seeds stored into the account
        // data itself.
        {
            let expected_pda = crate::create_signature_verification_pda(
                gateway_root_pda.key,
                &payload_merkle_root,
                session.bump,
            )?;
            if expected_pda != *verification_session_account.key {
                return Err(ProgramError::InvalidSeeds);
            }
        }

        let leaf_hash = message.leaf.hash::<SolanaSyscallHasher>();
        let proof = rs_merkle::MerkleProof::<SolanaSyscallHasher>::from_bytes(&message.proof)
            .inspect_err(|_err| {
                solana_program::msg!("Could not decode message proof");
            })
            .map_err(|_err| ProgramError::InvalidInstructionData)?;

        // Check: leaf node is part of the payload merkle root
        if !proof.verify(
            payload_merkle_root,
            &[message.leaf.position as usize],
            &[leaf_hash],
            message.leaf.set_size as usize,
        ) {
            solana_program::msg!("Invalid Merkle Proof for the given message");
            return Err(ProgramError::InvalidInstructionData);
        }

        // crate a PDA where we write the message metadata contents
        let message = message.leaf.message;
        let cc_id = message.cc_id;
        let command_id = command_id(&cc_id.chain, &cc_id.id);

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
        // todo assert that the PDA is valid
        init_pda_raw(
            funder,
            incoming_message_pda,
            &crate::id(),
            system_program,
            IncomingMessageWrapper::LEN
                .try_into()
                .expect("usize is valid u64 on sbf"),
            seeds,
        )?;
        let mut data = incoming_message_pda.try_borrow_mut_data()?;
        let data_bytes: &mut [u8; IncomingMessageWrapper::LEN] =
            (*data).try_into().map_err(|_err| {
                solana_program::msg!("session account data is corrupt");
                ProgramError::InvalidAccountData
            })?;
        let incoming_message_data: &mut IncomingMessageWrapper = bytemuck::cast_mut(data_bytes);
        incoming_message_data.bump = incoming_message_pda_bump;
        incoming_message_data.message = IncomingMessage::new(message.payload_hash);

        // Emit event
        GatewayEvent::MessageApproved(MessageApproved {
            command_id,
            source_chain: cc_id.chain,
            message_id: cc_id.id,
            source_address: message.source_address,
            destination_address: message.destination_address,
            payload_hash: message.payload_hash,
        })
        .emit()?;

        Ok(())
    }
}
