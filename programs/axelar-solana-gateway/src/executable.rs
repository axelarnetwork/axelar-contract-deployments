//! # Axelar Executable
//!
//! If we look at the event data that the EVM Gateway produces when making a [`gateway.callContract()`](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/432449d7b330ec6edf5a8e0746644a253486ca87/contracts/interfaces/IAxelarGateway.sol#L19-L25) call when communicating with an external chain:
//! - `destinationContractAddress` is the identifier of _where_ the Message should land on the destination chain.
//! - `payload` is the payload data used for the contract call. The raw data that the Relayer must send to the destination contract
//! - `payloadHash` is the `keccak256` hash of the sent payload data, used by the recipient to ensure that the data is not tampered with.
//!
//! Relayer for the edge chain **must** know how to call a contract on the given edge chain so that the contract can properly process the Axelar GMP message. The same concept applies to Solana, that every contract that wants to be compatible with the Axelar protocol **must** implement the `axelar-executable` interface. Implementing the `axelar-executable` interface allows the Relayer to send transactions to it.
//!
//! ![high level interactoin](https://github.com/user-attachments/assets/c676c8c6-8867-4560-a0cc-fa476b2b21a7)
//!
//! 1. Relayer will compose a transaction that can interact with the destination program on Solana
//! 2. `axelar-executable` exposes a function that allows the program to parse the Message into a format that it can work with
//! 3. `axelar-executable` exposes a utility function that validates that all the relayer-provided accounts are valid and not malicious. It then performs a CPI call to the Solana Gateway to set the message status to `Executed`.
//!
//!
//! ## Solana specific rundown
//!
//! > [!NOTE]
//! > For better clarity, read the following Solana docs:
//! > - [`Solana Account Model`](https://solana.com/docs/core/accounts)
//! > - [`Solana Transactions and Instructions`](https://solana.com/docs/core/transactions)
//! > - [`Solana CPI`](https://solana.com/docs/core/cpi)
//! > - [`Solana PDAs`](https://solana.com/docs/core/pda)
//!
//! **Accounts**
//!
//! A notable difference between Solana and EVM chains is that every interaction with some Solana programs must define an array of `accounts[]`. Accounts can be looked at as on-chain storage memory slots from Solidity. A contract can only read & write to the accounts provided when the instruction is created on the relayer/user level. The `accounts[]` must also include all the accounts that any internal CPI calls may require. Every storage slot the instruction may touch must be visible to the entity that crafts the transaction before submitting the TX for on-chain execution.
//!
//! **PDAs**
//!
//! Programs can have their accounts, called Program Derived Addresses (PDA for short), which sets the program ID as the owner. A contract can use PDAs to store data up to 10kb, or be a signer to make actoins on the behalf of a program. This can be imagined as having contract storage that only the on-chain program can modify. The PDAs must be deterministically derived.
//!
//! **CPI**
//!
//! Solana programs can call other programs (CPI - cross-program invocation), and a PDA can sign the calls on behalf of a program (this is because the program ID cannot be a signer itself; only its PDAs can be signers). Also, the CPI calls require a list of accounts to be proxied from the top-level call for the CPI call to operate correctly.
//!
//! Key points:
//! - If the proper accounts are not provided for a contract interaction, then **the call will fail**.
//! - The array of accounts **must** be known by the Relayer before calling the destination contract.
//! - The on-chain logic must validate that the provided accounts have been properly derived and are not malicious (e.g. checking if they were derived correctly, checking the expected owners, etc.)
//! - A Solana contract (represented by a `program_id`) cannot be a signer for a CPI call, but a PDA owned by the given `program_id` can be a signer. This is important for when the program makes a `gateway.validate_call()` CPI call
//!
//! ## Providing the `accounts[]` in the payload
//!
//! Solana identifies an instance of the on-chain program by the contract address AND the provided accounts. This means that by having a different array of accounts passed to a program, we may communicate to a completely different instance of the same program.
//!
//! For example, the [SPL token program](https://spl.solana.com/token#creating-a-new-token-type) is a singular program everyone uses to create their on-chain tokens. By having a different token mint account, you are effectively talking to a different token — all while the `program_id` has not even changed.
//!
//! This means that the base interface for GMP messages defined by the Axelar protocol is not expressive enough for communicating with Solana programs - because there’s no place to put the account arrays. Therefore a workaround is needed, where we need to provide the information about the accounts within the scope of the existing `ContractCall` data structure.
//!
//! Solanas `axelar-executable` now expects that the `payload` emitted on the source chain also includes all the `account[]` data for the Relayer to create a proper transaction. If the `account[]` is absent on the GMP call, the Relayer cannot know what accounts to provide when calling the destination contract.
//!
//! ![Axelar Message Payload](https://github.com/user-attachments/assets/29a96677-83f5-4727-befa-3f815e31ad39)
//!
//!
//! The source chain that wants to interact with Solana must encode the messages in a specific format that the Solana destination contract understands, and the Relayer can understand. `Accounts[]` requirement also lets the Relayer deterministically derive the accounts when crafting the transaction.
//!
//! Currently, the `[0]th` byte of the payload indicates the encoding that specifies how the rest of the data is encoded:
//!
//! | value | meaning |
//! |--|--|
//! | 0b00000000 | The rest of the data is Borsh encoded |
//! | 0b00000001 | The rest of the data is ABI encoded |
//!
//! The way how accounts and the payload are encoded is encoding-specific. The `axelar-executable` and the Relayer maintainers can add new encoding support over time. As new chains get added to Axelar, they may not play nicely with ABI or Borsh (or just be expensive to compute). Making the encoding flexible gives us room to support new encodings in the future.
//!
//! ## Examples
//!
//! | Item | Explanation |
//! |--|--|
//! | [`SolanaGatewayPayload.sol`](https://github.com/eigerco/solana-axelar/blob/main/evm-contracts/src/SolanaGatewayPayload.sol) | Can be used to encode the accounts together with the actual payload in Solidity so that the Solana relayer can properly interpret them |
//! | [`AxelarMemo.sol`](https://github.com/eigerco/solana-axelar/blob/033bd17df32920eb6b57a0e6b8d3f82298b0c5ff/evm-contracts/src/AxelarMemo.sol#L46) | An example contract showcasing how the Solidity contract would send a `string` message to Solana, while prefixing some arbitrary accounts  |
//! | [`AxelarSolanaMemo::processor`](https://github.com/eigerco/solana-axelar/blob/033bd17df32920eb6b57a0e6b8d3f82298b0c5ff/solana/programs/axelar-solana-memo-program/src/processor.rs#L33-L36) | Solana program example that implements `axelar-executable`; receives a string message and prints all the accounts it has received |
//! | [end to end unittest](https://github.com/eigerco/solana-axelar/blob/033bd17df32920eb6b57a0e6b8d3f82298b0c5ff/solana/programs/axelar-solana-memo-program/tests/evm-e2e/from_evm_to_solana.rs#L202) | A unit test you can run locally and see for yourself how it works together |
//!
//! ## Differences from EVM
//!
//! | Action | EVM Axelar Executable | Solana Axelar Executable |
//! |--|--|--|
//! | Interface that the contract must implement | EVM contracts must inherit the [AxelarExecutable.sol](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/432449d7b330ec6edf5a8e0746644a253486ca87/contracts/executable/AxelarExecutable.sol) base contract for the Relayer to be able to call it.  | On Solana, there is no concept for inheriting contracts. There is only a single entry point for a Solana program, and the branching logic of how the raw bytes are interpreted is up to the contract logic. On Solana, a contract developer **must** try to parse the incoming bytes using [`parse_axelar_message()`](https://github.com/eigerco/solana-axelar-internal/blob/aee979d2c83d875a9255d73b4eb31eb0f38e544d/solana/crates/axelar-executable/src/lib.rs#L282) before attempting any other action.  |
//! | Relayer calling `destination_contract.execute()` | The Relayer will encode the call with the full payload and call the destination contract; the `AxelarExecutable.sol` interface allows to process the incoming payload | The Relayer will split the raw payload from the `accounts[]` because the tx layout requires them to be split. It will encode the payload in a way that allows the destination contract to parse it using this library |
//! | Calling `validate_message()` | The `AxelarExecutable.sol` contract takes care of the internal cross-contract call to the Gateway to validate that the message has been approved | The Solana contract developer *must* immediately call [`validate_message()`](https://github.com/eigerco/solana-axelar-internal/blob/aee979d2c83d875a9255d73b4eb31eb0f38e544d/solana/crates/axelar-executable/src/lib.rs#L47) form the `axelar-executable` library. This internally will make a CPI call to the Gateway using the messages command id to create a short-lived PDA that will be the signer of the call.  |
//! | Providing the raw payload | As tx arguments | Raw payload is stored on a PDA owned by the Gateway; it must be read from there |
//! | Providing the raw `accounts[]` from the original payload | _No such concept_ | Relayer will parse the payload, extract the provided accounts and append them in the order that they were provided in the original message |
//! | Providing accounts for the raw payload, gateway, etc | _No such concept_ | Relayer will prefix hardcoded accounts for the raw payload PDA |
//! | Validating payload hash | `AxelarExecutable.sol` will take care of hashing the raw payload from tx args and comparing it with the provided payload hash | `axelar_executable::validate_message` will validate all the arguments passed to the instruction (including accounts)  match the ones defined on `MessagePayload PDA`, and ensure that the data hashes match. |
//!
//! You can see the anatomy of an instruction that the Solana Relayer will send to the destination program when sending the raw payload to it:
//!
//! ![Anatomy of an ix](https://github.com/user-attachments/assets/0312abb4-fe7f-45c7-a8ae-1318489da9d2)
//!
//!
//! ## Exceptions of the `accounts[]` rule: ITS & Governance
//!
//! [Interchain Token Service](https://github.com/axelarnetwork/interchain-token-service/blob/main/DESIGN.md#interchain-tokens) and [Governance contract](https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/432449d7b330ec6edf5a8e0746644a253486ca87/test/utils.js#L24-L44) have a legacy ABI interface that must be respected. This means that we cannot enforce arbitrary new encoding for the existing protocols; we only need them to be able to interact with Solana. As a result, the Relayer has special handling when interacting with ITS & Governance contracts; it will decode the `abi` encoded messages, introspect into the message contents and attempt to deterministically derive all the desired accounts for the action the message wants to make. This approach only works when the message layout is known beforehand (meaning that the Relayer can decode it) AND the Relayer has hardcoded custom logic to derive the accounts. This means that this special handling is not possible for the generic case.
//!
//! Also, ITS & Governance use a different entry point on [`axelar_executable::validate_with_gmp_metadata`](https://github.com/eigerco/solana-axelar/blob/033bd17df32920eb6b57a0e6b8d3f82298b0c5ff/solana/crates/axelar-executable/src/lib.rs#L128C8-L128C34). The only difference is that the `accounts[]` validation is no longer done by `axelar_executable`. Instead, it becomes the responsibility of the destination contract itself to validate the provided `accounts[]`.

use crate::error::GatewayError;
use crate::state::incoming_message::{command_id, IncomingMessage};
use crate::state::message_payload::ImmutMessagePayload;
use crate::{get_validate_message_signing_pda, BytemuckedPda};
use axelar_solana_encoding::types::messages::Message;
use core::str::FromStr;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::msg;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

mod axelar_payload;
pub use axelar_payload::{
    AxelarMessagePayload, AxelarMessagePayloadHash, EncodingScheme, PayloadError, SolanaAccountRepr,
};

/// Axelar executable command prefix
pub const AXELAR_EXECUTE: &[u8; 16] = b"axelar-execute__";

/// The index of the first account that is expected to be passed to the
/// destination program.
pub const PROGRAM_ACCOUNTS_START_INDEX: usize = 4;

/// Perform CPI call to the Axelar Gateway to ensure that the given message is
/// approved.
///
/// The check will ensure that the provided accounts are indeed the ones that
/// were originated on the source chain.
///
/// Expected accounts:
/// 0. `gateway_incoming_message` - `GatewayApprovedMessage` PDA
/// 1. `gateway_message_payload` - `MessagePayload` PDA
/// 2. `signing_pda` - Signing PDA that's associated with the provided
///    `program_id`
/// 3. `gateway_program_id` - Gateway Prorgam ID
/// N. accounts required by the `DataPayload` constructor
///
/// # Errors
/// - if not enough accounts were provided
/// - if the payload hashes do not match
/// - if CPI call to the gateway failed
pub fn validate_message(accounts: &[AccountInfo<'_>], message: &Message) -> ProgramResult {
    let (relayer_prepended_accs, origin_chain_provided_accs) =
        accounts.split_at(PROGRAM_ACCOUNTS_START_INDEX);
    let accounts_iter = &mut relayer_prepended_accs.iter();

    let incoming_message_payload_hash;
    let signing_pda_bump = {
        // scope to drop the account borrow after reading the data we want
        let incoming_message_pda = next_account_info(accounts_iter)?;

        // Check: Incoming Message account is owned by the Gateway
        if incoming_message_pda.owner != &crate::ID {
            return Err(ProgramError::InvalidAccountOwner);
        }

        let incoming_message_data = incoming_message_pda.try_borrow_data()?;
        let incoming_message = IncomingMessage::read(&incoming_message_data)
            .ok_or(GatewayError::BytemuckDataLenInvalid)?;
        incoming_message_payload_hash = incoming_message.payload_hash;
        incoming_message.signing_pda_bump
    };

    // Check: Message Payload account is owned by the Gateway
    let message_payload_account = next_account_info(accounts_iter)?;
    if message_payload_account.owner != &crate::ID {
        return Err(ProgramError::InvalidAccountOwner);
    }

    // Read the raw payload from the MessagePayload PDA account
    let message_payload_account_data = message_payload_account.try_borrow_data()?;
    let message_payload: ImmutMessagePayload<'_> = (**message_payload_account_data).try_into()?;

    // Check: MessagePayload PDA is finalized
    if !message_payload.committed() {
        return Err(ProgramError::InvalidAccountData);
    }

    // Check: MessagePayload's payload hash matches IncomingMessage's
    if *message_payload.payload_hash != incoming_message_payload_hash {
        return Err(ProgramError::InvalidAccountData);
    }

    // Decode the raw payload
    let axelar_payload = AxelarMessagePayload::decode(message_payload.raw_payload)?;

    // Check: parsed accounts matches the original chain provided accounts
    if !axelar_payload
        .solana_accounts()
        .eq(origin_chain_provided_accs)
    {
        return Err(ProgramError::InvalidAccountData);
    }

    validate_message_internal(
        accounts,
        message,
        message_payload.payload_hash,
        signing_pda_bump,
    )
}

/// Perform CPI (Cross-Program Invocation) call to the Axelar Gateway to
/// ensure that the given command (containing a GMP message) is approved
///
/// This is useful for contracts that have custom legacy implementations by
/// Axelar on other chains, and therefore they cannot provide the accounts in
/// the GMP message. Therefore, the validation of the accounts becomes the
/// responsibility of the destination program.
///
/// Expected accounts:
/// 0. `gateway_incoming_message` - `GatewayApprovedMessage` PDA
/// 1. `gateway_message_payload` - `MessagePayload` PDA
/// 2. `signing_pda` - Signing PDA that's associated with the provided
///    `program_id`
/// 3. `gateway_program_id` - Gateway Prorgam ID
/// N. accounts required by the inner instruction (part of the payload).
///
/// # Errors
/// - if not enough accounts were provided
/// - if the payload hashes do not match
/// - if CPI call to the gateway failed
pub fn validate_with_gmp_metadata(
    accounts: &[AccountInfo<'_>],
    message: &Message,
) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let signing_pda_bump = {
        // scope to release the account after reading the data we want
        let incoming_message_pda = next_account_info(accounts_iter)?;
        let incoming_message_data = incoming_message_pda.try_borrow_data()?;
        let incoming_message = IncomingMessage::read(&incoming_message_data)
            .ok_or(GatewayError::BytemuckDataLenInvalid)?;
        incoming_message.signing_pda_bump
    };

    // Check: Message Payload account is owned by the Gateway
    let message_payload_account = next_account_info(accounts_iter)?;
    if message_payload_account.owner != &crate::ID {
        return Err(ProgramError::InvalidAccountOwner);
    }

    // Read the raw payload from the MessagePayload PDA account
    let message_payload_account_data = message_payload_account.try_borrow_data()?;
    let message_payload: ImmutMessagePayload<'_> = (**message_payload_account_data).try_into()?;

    // Check: MessagePayload PDA is finalized
    if !message_payload.committed() {
        return Err(ProgramError::InvalidAccountData);
    }

    let axelar_raw_payload = message_payload.raw_payload;
    let payload_hash = solana_program::keccak::hash(axelar_raw_payload).to_bytes();

    if payload_hash != *message_payload.payload_hash {
        return Err(ProgramError::InvalidAccountData);
    }

    validate_message_internal(
        accounts,
        message,
        message_payload.payload_hash,
        signing_pda_bump,
    )
}

fn validate_message_internal(
    accounts: &[AccountInfo<'_>],
    message: &Message,
    payload_hash: &[u8; 32],
    signing_pda_derived_bump: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let gateway_incoming_message = next_account_info(account_info_iter)?;
    let _message_payload_pda = next_account_info(account_info_iter)?; // skip this one, we don't need it
    let signing_pda = next_account_info(account_info_iter)?;
    let _gateway_program_id = next_account_info(account_info_iter)?;

    // Build the actual Message we are going to use
    let command_id = command_id(&message.cc_id.chain, &message.cc_id.id);

    // Check: Original message's payload_hash is equivalent to provided payload's
    // hash
    if &message.payload_hash != payload_hash {
        msg!("Invalid payload hash");
        return Err(ProgramError::InvalidInstructionData);
    }

    invoke_signed(
        &crate::instructions::validate_message(
            gateway_incoming_message.key,
            signing_pda.key,
            message.clone(),
        )?,
        &[gateway_incoming_message.clone(), signing_pda.clone()],
        &[&[&command_id, &[signing_pda_derived_bump]]],
    )?;

    Ok(())
}

/// # Create a generic `Execute` instruction
///
/// Intended to be used by the relayer when it is about to call the
/// destination program.
///
/// It will prepend the accounts array with these predefined accounts
/// 0. `gateway_incoming_message` - `GatewayApprovedMessage` PDA
/// 1. `gateway_message_payload` - `MessagePayload` PDA
/// 2. `signing_pda` - Signing PDA that's associated with the provided
///    `program_id`
/// 3. `gateway_root_pda` - Gateway Root PDA
/// 4. `gateway_program_id` - Gateway Prorgam ID
/// N... - The accounts provided in the `axelar_message_payload`
///
/// # Errors
/// - if the destination address is not a vald base58 encoded ed25519 pubkey
/// - if the `axelar_message_payload` could not be decoded
/// - if we cannot encode the `AxelarExecutablePayload`
pub fn construct_axelar_executable_ix(
    message: &Message,
    // The payload of the incoming message, contains encoded accounts and the actual payload
    axelar_message_payload: &[u8],
    // The PDA for the gateway approved message, this *must* be initialized
    // beforehand
    gateway_incoming_message: Pubkey,
    gateway_message_payload: Pubkey,
) -> Result<Instruction, ProgramError> {
    let passed_in_accounts = AxelarMessagePayload::decode(axelar_message_payload)?.account_meta();

    let destination_address = Pubkey::from_str(&message.destination_address)
        .map_err(|_er| ProgramError::InvalidAccountData)?;

    let command_id = command_id(&message.cc_id.chain, &message.cc_id.id);
    let (signing_pda, _) = get_validate_message_signing_pda(destination_address, command_id);

    let mut accounts = vec![
        // The expected accounts for the `ValidateMessage` ix
        AccountMeta::new(gateway_incoming_message, false),
        AccountMeta::new_readonly(gateway_message_payload, false),
        AccountMeta::new_readonly(signing_pda, false),
        AccountMeta::new_readonly(crate::id(), false),
    ];
    accounts.extend(passed_in_accounts);

    let data = serialize_message(message)?;

    Ok(Instruction {
        program_id: destination_address,
        accounts,
        data,
    })
}

/// We prefix a byte slice with the literal contents of `AXELAR_EXECUTE` followed
/// by the borsh-serialized Message.
///
/// This two-step approach is needed because borsh demonstrated to exaust a Solana
/// program's memory when trying to deserialize the alternative form (Tag, Message)
/// for an absent tag.
fn serialize_message(message: &Message) -> Result<Vec<u8>, ProgramError> {
    // In our tests, randomly generated messages have, in average, 175 bytes, so 256
    // should be sufficient to avoid reallocations.
    let mut buffer = Vec::with_capacity(256);
    buffer.extend_from_slice(AXELAR_EXECUTE);
    borsh::to_writer(&mut buffer, &message)
        .map_err(|borsh_error| ProgramError::BorshIoError(borsh_error.to_string()))?;
    Ok(buffer)
}

/// Tries to parse input into an Axelar's message.
///
/// # Errors
/// Will return a `ProgramError::BorshIoError` if parsing fails.
#[allow(clippy::indexing_slicing)]
#[must_use]
pub fn parse_axelar_message(input: &[u8]) -> Option<Result<Message, ProgramError>> {
    // This pre-parsing check is required, otherwise borsh will exhaust the available
    // memory trying to find a possibly missing `AXELAR_EXECUTE` prefix.
    if !input.starts_with(AXELAR_EXECUTE) {
        return None;
    }

    // Slicing: we already checked that slice's lower bound above.
    match borsh::from_slice(&input[AXELAR_EXECUTE.len()..])
        .map_err(|borsh_error| ProgramError::BorshIoError(borsh_error.to_string()))
    {
        Ok(message) => Some(Ok(message)),
        Err(err) => Some(Err(err)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axelar_solana_gateway_test_fixtures::gateway::random_message;

    #[test]
    fn test_instruction_serialization() {
        let message = random_message();
        let serialized = serialize_message(&message).unwrap();
        let deserialized = parse_axelar_message(&serialized).unwrap().unwrap();
        assert_eq!(message, deserialized);
    }
}
