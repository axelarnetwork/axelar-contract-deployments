//! Instructions supported by the multicall program.

use std::borrow::Cow;

use axelar_message_primitives::{DataPayload, DestinationProgramId};
use axelar_solana_encoding::types::messages::Message;
use axelar_solana_gateway::state::incoming_message::command_id;
use bitflags::bitflags;
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use interchain_token_transfer_gmp::{
    DeployInterchainToken, DeployTokenManager, GMPPayload, InterchainTransfer, SendToHub,
};
use role_management::instructions::RoleManagementInstruction;
use solana_program::bpf_loader_upgradeable;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::{system_program, sysvar};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use typed_builder::TypedBuilder;

use crate::state::{self, flow_limit};
use crate::Roles;

pub mod interchain_token;
pub mod minter;
pub mod operator;
pub mod token_manager;

bitflags! {
    /// Bitmask for the optional accounts passed in some of the instructions.
    #[derive(Debug, PartialEq, Eq)]
    pub struct OptionalAccountsFlags: u8 {
        /// The minter account is being passed.
        const MINTER = 0b0000_0001;

        /// The minter roles account is being passed.
        const MINTER_ROLES = 0b0000_0010;

        /// The operator account is being passed.
        const OPERATOR = 0b0000_0100;

        /// The operator roles account is being passed.
        const OPERATOR_ROLES = 0b0000_1000;
    }
}

impl PartialEq<u8> for OptionalAccountsFlags {
    fn eq(&self, other: &u8) -> bool {
        self.bits().eq(other)
    }
}

impl PartialEq<OptionalAccountsFlags> for u8 {
    fn eq(&self, other: &OptionalAccountsFlags) -> bool {
        self.eq(&other.bits())
    }
}

impl BorshSerialize for OptionalAccountsFlags {
    fn serialize<W: std::io::prelude::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.bits().serialize(writer)
    }
}

impl BorshDeserialize for OptionalAccountsFlags {
    fn deserialize_reader<R: std::io::prelude::Read>(reader: &mut R) -> std::io::Result<Self> {
        let byte = u8::deserialize_reader(reader)?;
        Ok(Self::from_bits_truncate(byte))
    }
}

/// Instructions supported by the multicall program.
#[derive(Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum InterchainTokenServiceInstruction {
    /// Initializes the interchain token service program.
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. [writable,signer] The address of payer / sender
    /// 1. [] gateway root pda
    /// 2. [writable] ITS root pda
    /// 3. [] system program id
    /// 4. [] The account that will become the operator of the ITS
    /// 5. [writable] The address of the PDA account that will store the roles
    ///    of the operator account.
    Initialize,

    /// Pauses or unpauses the interchain token service.
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. [writable,signer] The ITS owner.
    /// 1. [] The program data account.
    /// 2. [writable] ITS root pda.
    SetPauseStatus {
        /// The new pause status.
        paused: bool,
    },

    /// Deploys an interchain token.
    ///
    /// 0. [writable,signer] The address of payer / sender
    /// 1. [] Gateway root pda
    /// 2. [] System program id if local deployment OR Gateway program if remote
    ///    deployment
    /// 3. [] ITS root pda
    /// 4. [writable] Token Manager PDA (if local deployment)
    /// 5. [writable] The mint account to be created (if local deployment)
    /// 6. [writable] The Token Manager ATA (if local deployment)
    /// 7. [] Token program id (if local deployment)
    /// 8. [] Associated token program id (if local deployment)
    /// 9. [] Rent sysvar (if local deployment)
    /// 10. [] The minter account (if local deployment)
    DeployInterchainToken {
        /// The deploy params containing token metadata as well as other
        /// required inputs.
        params: DeployInterchainTokenInputs,
    },

    /// Deploys a token manager.
    ///
    /// 0. [writable,signer] The address of payer / sender
    /// 1. [] Gateway root pda
    /// 2. [] System program id if local deployment OR Gateway program if remote
    ///    deployment
    /// 3. [] ITS root pda
    /// 4. [writable] Token Manager PDA (if local deployment)
    /// 5. [writable] The mint account to be created (if local deployment)
    /// 6. [writable] The Token Manager ATA (if local deployment)
    /// 7. [] Token program id (if local deployment)
    /// 8. [] Associated token program id (if local deployment)
    /// 9. [] Rent sysvar (if local deployment)
    /// 10. [] The minter account (if local deployment)
    DeployTokenManager {
        /// The deploy params containing token metadata as well as other
        /// required inputs.
        params: DeployTokenManagerInputs,

        /// The optional accounts mask for the instruction.
        optional_accounts_mask: OptionalAccountsFlags,
    },

    /// Transfers interchain tokens.
    ///
    /// 0. [maybe signer] The address of the authority signing the transfer. In
    ///    case it's the `TokenManager`, it shouldn't be set as signer as the
    ///    signing happens on chain.
    /// 1. [] Gateway root pda
    /// 2. [] Gateway program id
    /// 3. [] ITS root pda
    /// 4. [writable] Interchain token PDA
    /// 5. [writable] The account where the tokens are being transferred from
    /// 5. [writable] The mint account
    /// 6. [writable] The Token Manager PDA
    /// 6. [writable] The Token Manager ATA
    /// 7. [] Token program id
    InterchainTransfer {
        /// The transfer parameters.
        params: InterchainTransferInputs,
    },

    /// Transfers tokens to a contract on the destination chain and call the give instruction on
    /// it. This instruction is is the same as [`InterchainTransfer`], but will fail if call data
    /// is empty.
    ///
    /// 0. [maybe signer] The address of the authority signing the transfer. In
    ///    case it's the `TokenManager`, it shouldn't be set as signer as the
    ///    signing happens on chain.
    /// 1. [] Gateway root pda
    /// 2. [] Gateway program id
    /// 3. [] ITS root pda
    /// 4. [writable] Interchain token PDA
    /// 5. [writable] The account where the tokens are being transferred from
    /// 5. [writable] The mint account
    /// 6. [writable] The Token Manager PDA
    /// 6. [writable] The Token Manager ATA
    /// 7. [] Token program id
    CallContractWithInterchainToken {
        /// The instruction inputs.
        params: CallContractWithInterchainTokenInputs,
    },

    /// Transfers tokens to a contract on the destination chain and call the give instruction on
    /// it. This instruction is is the same as [`InterchainTransfer`], but will fail if call data
    /// is empty.
    ///
    /// 0. [maybe signer] The address of the authority signing the transfer. In
    ///    case it's the `TokenManager`, it shouldn't be set as signer as the
    ///    signing happens on chain.
    /// 1. [] Gateway root pda
    /// 2. [] Gateway program id
    /// 3. [] ITS root pda
    /// 4. [writable] Interchain token PDA
    /// 5. [writable] The account where the tokens are being transferred from
    /// 5. [writable] The mint account
    /// 6. [writable] The Token Manager PDA
    /// 6. [writable] The Token Manager ATA
    /// 7. [] Token program id
    CallContractWithInterchainTokenOffchainData {
        /// The instruction inputs.
        params: CallContractWithInterchainTokenInputs,
    },

    /// A GMP Interchain Token Service instruction.
    ///
    /// 0. [writable,signer] The address of payer / sender
    /// 1. [] gateway root pda
    /// 2. [] ITS root pda
    ///
    /// 3..N Accounts depend on the inner ITS instruction.
    ItsGmpPayload {
        /// The GMP metadata
        message: Message,

        /// The optional accounts mask for the instruction.
        optional_accounts_mask: OptionalAccountsFlags,
    },

    /// Sets the flow limit for an interchain token.
    SetFlowLimit {
        /// The new flow limit.
        flow_limit: u64,
    },

    /// ITS operator role management instructions.
    ///
    /// 0. [] Gateway root pda
    /// 1..N [`operator::OperatorInstruction`] accounts, where the resource is
    /// the ITS root PDA.
    OperatorInstruction(operator::Instruction),

    /// Instructions operating on deployed [`TokenManager`] instances.
    TokenManagerInstruction(token_manager::Instruction),

    /// Instructions operating in Interchain Tokens.
    InterchainTokenInstruction(interchain_token::Instruction),
}

/// Parameters for `[InterchainTokenServiceInstruction::DeployInterchainToken]`.
///
/// To construct this type, use its builder API.
///
/// # Example
///
/// ```ignore
/// use axelar_solana_its::instructions::DeployInterchainTokenInputs;
///
/// let params = DeployInterchainTokenInputs::builder()
///    .payer(payer_pubkey)
///    .salt(salt)
///    .name("MyToken".to_owned())
///    .symbol("MT".to_owned())
///    .decimals(18)
///    .minter(payer_pubkey)
///    .gas_value(100)
///    .build();
/// ```
#[derive(Debug, PartialEq, Eq, Clone, TypedBuilder, BorshSerialize, BorshDeserialize)]
pub struct DeployInterchainTokenInputs {
    /// The payer account for this transaction
    pub(crate) payer: Pubkey,

    /// The salt used to derive the tokenId associated with the token
    pub(crate) salt: [u8; 32],

    /// The program id of the gas service program.
    pub(crate) gas_service: Pubkey,

    /// The PDA of the account holding the gas service config.
    pub(crate) gas_config_pda: Pubkey,

    /// The chain where the `InterchainToken` should be deployed.
    /// Deploys to (crate) Solana if `None`.
    #[builder(default, setter(strip_option))]
    pub(crate) destination_chain: Option<String>,

    /// Token name
    pub(crate) name: String,

    /// Token symbol
    pub(crate) symbol: String,

    /// Token decimals
    pub(crate) decimals: u8,

    /// The minter account
    pub(crate) minter: Vec<u8>,

    /// The gas value to be paid for the deploy transaction
    pub(crate) gas_value: u64,

    /// Signing PDA bump
    #[builder(default, setter(strip_option))]
    pub(crate) signing_pda_bump: Option<u8>,
}

/// Parameters for `[InterchainTokenServiceInstruction::DeployTokenManager]`.
///
/// To construct this type, use its builder API.
#[derive(Debug, PartialEq, Eq, Clone, TypedBuilder, BorshSerialize, BorshDeserialize)]
pub struct DeployTokenManagerInputs {
    /// The payer account for this transaction
    pub(crate) payer: Pubkey,

    /// The salt used to derive the tokenId associated with the token
    pub(crate) salt: [u8; 32],

    /// The program id of the gas service program.
    pub(crate) gas_service: Pubkey,

    /// The PDA of the account holding the gas service config.
    pub(crate) gas_config_pda: Pubkey,

    /// The chain where the `TokenManager` should be deployed.
    /// Deploys to Solana if `None`.
    #[builder(default, setter(strip_option))]
    pub(crate) destination_chain: Option<String>,

    /// Token manager type
    pub(crate) token_manager_type: state::token_manager::Type,

    /// Chain specific params for the token manager
    pub(crate) params: Vec<u8>,

    /// The gas value to be paid for the deploy transaction
    pub(crate) gas_value: u64,

    /// Required when deploying the [`TokenManager`] on Solana, this is the
    /// token program that owns the mint account, either `spl_token::id()` or
    /// `spl_token_2022::id()`.
    #[builder(default, setter(strip_option))]
    pub(crate) token_program: Option<Pubkey>,

    /// Signing PDA bump
    #[builder(default, setter(strip_option))]
    pub(crate) signing_pda_bump: Option<u8>,
}

/// Parameters for `[InterchainTokenServiceInstruction::InterchainTransfer]`.
///
/// To construct this type, use its builder API.
#[derive(Debug, PartialEq, Eq, Clone, TypedBuilder, BorshSerialize, BorshDeserialize)]
pub struct InterchainTransferInputs {
    /// The payer account for this transaction.
    pub(crate) payer: Pubkey,

    /// The source account.
    pub(crate) source_account: Pubkey,

    /// The program id of the gas service program.
    pub(crate) gas_service: Pubkey,

    /// The PDA of the account holding the gas service config.
    pub(crate) gas_config_pda: Pubkey,

    /// The source account owner. In case of a transfer using a Mint/BurnFrom
    /// `TokenManager`, this shouldn't be set as the authority will be the
    /// `TokenManager`.
    #[builder(default, setter(strip_option))]
    pub(crate) authority: Option<Pubkey>,

    /// The token id associated with the token
    pub(crate) token_id: [u8; 32],

    /// The token mint account. **This should be set in case the token is not
    /// ITS native**.
    ///
    /// When not set, the account is derived from the given `token_id`. The
    /// derived account is invalid if the token is not an ITS native token (not
    /// originally created/deployed by ITS).
    #[builder(default, setter(strip_option))]
    pub(crate) mint: Option<Pubkey>,

    /// The chain where the tokens are being transferred to.
    #[builder(setter(strip_option))]
    pub(crate) destination_chain: Option<String>,

    /// The address on the destination chain to send the tokens to.
    pub(crate) destination_address: Vec<u8>,

    pub(crate) amount: u64,

    /// Optional data for the call for additional effects (such as calling a
    /// destination contract).
    pub(crate) data: Vec<u8>,

    /// Hash of the call contract data being sent off-chain. The hash should be calculated on the
    /// final ITS message.
    #[builder(default, setter(skip))]
    pub(crate) payload_hash: Option<[u8; 32]>,

    /// The gas value to be paid for the deploy transaction
    pub(crate) gas_value: u64,

    /// Current chain's unix timestamp.
    pub(crate) timestamp: i64,

    /// The token program that owns the mint account, either `spl_token::id()`
    /// or `spl_token_2022::id()`. Assumes `spl_token_2022::id()` if not set.
    #[builder(default = spl_token_2022::id())]
    pub(crate) token_program: Pubkey,

    /// Signing PDA bump
    #[builder(default, setter(strip_option))]
    pub(crate) signing_pda_bump: Option<u8>,
}

/// Inputs for the `[InterchainTokenServiceInstruction::CallContractWithInterchainToken]`
pub type CallContractWithInterchainTokenInputs = InterchainTransferInputs;

/// Inputs for the [`its_gmp_payload`] function.
///
/// To construct this type, use its builder API.
///
/// # Example
///
/// ```ignore
/// use axelar_solana_its::instructions::ItsGmpInstructionInputs;
///
/// let inputs = ItsGmpInstructionInputs::builder()
///   .payer(payer_pubkey)
///   .incoming_message_pda(gateway_approved_message_pda)
///   .message(message)
///   .payload(payload)
///   .token_program(spl_token_2022::id())
///   .mint(mint_pubkey)
///   .bumps(bumps)
///   .build();
/// ```
#[derive(Debug, Clone, TypedBuilder)]
pub struct ItsGmpInstructionInputs {
    /// The payer account.
    pub(crate) payer: Pubkey,

    /// The PDA used to track the message status by the gateway program.
    pub(crate) incoming_message_pda: Pubkey,

    /// The PDA used to to store the message payload.
    pub(crate) message_payload_pda: Pubkey,

    /// The Axelar GMP metadata.
    pub(crate) message: Message,

    /// The ITS GMP payload.
    pub(crate) payload: GMPPayload,

    /// The token program required by the instruction (spl-token or
    /// spl-token-2022).
    pub(crate) token_program: Pubkey,

    /// The mint account required by the instruction. Hard requirement for
    /// `InterchainTransfer` instruction. Optional for `DeployTokenManager` and
    /// ignored by `DeployInterchainToken`.
    #[builder(default, setter(strip_option(fallback = mint_opt)))]
    pub(crate) mint: Option<Pubkey>,

    /// The current approximate timestamp. Required for `InterchainTransfer`s.
    #[builder(default, setter(strip_option(fallback = timestamp_opt)))]
    pub(crate) timestamp: Option<i64>,
}

/// Creates an [`InterchainTokenServiceInstruction::Initialize`] instruction.
///
/// # Errors
///
/// If serialization fails.
pub fn initialize(
    payer: Pubkey,
    gateway_root_pda: Pubkey,
    operator: Pubkey,
) -> Result<Instruction, ProgramError> {
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let (program_data_address, _) =
        Pubkey::find_program_address(&[crate::id().as_ref()], &bpf_loader_upgradeable::id());
    let (user_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::id(), &its_root_pda, &operator);

    let data = to_vec(&InterchainTokenServiceInstruction::Initialize)?;

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(program_data_address, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new(its_root_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(operator, false),
        AccountMeta::new(user_roles_pda, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::SetPauseStatus`] instruction.
///
/// # Errors
///
/// If serialization fails.
pub fn set_pause_status(payer: Pubkey, paused: bool) -> Result<Instruction, ProgramError> {
    let (program_data_address, _) =
        Pubkey::find_program_address(&[crate::id().as_ref()], &bpf_loader_upgradeable::id());
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);

    let data = to_vec(&InterchainTokenServiceInstruction::SetPauseStatus { paused })?;

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(program_data_address, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new(its_root_pda, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::DeployInterchainToken`]
/// instruction.
///
/// # Errors
///
/// If serialization fails.
pub fn deploy_interchain_token(
    mut params: DeployInterchainTokenInputs,
) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let payer = Pubkey::new_from_array(
        params
            .payer
            .as_ref()
            .try_into()
            .map_err(|_err| ProgramError::InvalidInstructionData)?,
    );

    let mut accounts = vec![AccountMeta::new(payer, true)];

    if params.destination_chain.is_none() {
        let (mut its_accounts, _) =
            derive_its_accounts(&params, gateway_root_pda, spl_token_2022::id(), None, None)?;

        accounts.append(&mut its_accounts);
    } else {
        let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
        let (call_contract_signing_pda, signing_pda_bump) =
            axelar_solana_gateway::get_call_contract_signing_pda(crate::ID);
        params.signing_pda_bump = Some(signing_pda_bump);

        accounts.push(AccountMeta::new_readonly(gateway_root_pda, false));
        accounts.push(AccountMeta::new_readonly(
            axelar_solana_gateway::id(),
            false,
        ));
        accounts.push(AccountMeta::new(params.gas_config_pda, false));
        accounts.push(AccountMeta::new_readonly(params.gas_service, false));
        accounts.push(AccountMeta::new_readonly(system_program::id(), false));
        accounts.push(AccountMeta::new_readonly(its_root_pda, false));
        accounts.push(AccountMeta::new_readonly(call_contract_signing_pda, false));
        accounts.push(AccountMeta::new_readonly(crate::ID, false));
    };

    let data = to_vec(&InterchainTokenServiceInstruction::DeployInterchainToken { params })?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::DeployTokenManager`]
/// instruction.
///
/// # Errors
///
/// If serialization fails.
pub fn deploy_token_manager(
    mut params: DeployTokenManagerInputs,
) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let payer = Pubkey::new_from_array(
        params
            .payer
            .as_ref()
            .try_into()
            .map_err(|_err| ProgramError::InvalidInstructionData)?,
    );
    let mut accounts = vec![AccountMeta::new(payer, true)];

    let optional_accounts_mask = if params.destination_chain.is_none() {
        let token_program = Pubkey::new_from_array(
            params
                .token_program
                .ok_or(ProgramError::InvalidInstructionData)?
                .as_ref()
                .try_into()
                .map_err(|_err| ProgramError::InvalidInstructionData)?,
        );

        let (mut its_accounts, optional_accounts_mask) =
            derive_its_accounts(&params, gateway_root_pda, token_program, None, None)?;

        accounts.append(&mut its_accounts);

        optional_accounts_mask
    } else {
        let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
        let (call_contract_signing_pda, signing_pda_bump) =
            axelar_solana_gateway::get_call_contract_signing_pda(crate::ID);
        params.signing_pda_bump = Some(signing_pda_bump);

        accounts.push(AccountMeta::new_readonly(gateway_root_pda, false));
        accounts.push(AccountMeta::new_readonly(
            axelar_solana_gateway::id(),
            false,
        ));
        accounts.push(AccountMeta::new(params.gas_config_pda, false));
        accounts.push(AccountMeta::new_readonly(params.gas_service, false));
        accounts.push(AccountMeta::new_readonly(system_program::id(), false));
        accounts.push(AccountMeta::new_readonly(its_root_pda, false));
        accounts.push(AccountMeta::new_readonly(call_contract_signing_pda, false));
        accounts.push(AccountMeta::new_readonly(crate::ID, false));

        OptionalAccountsFlags::empty()
    };

    let data = to_vec(&InterchainTokenServiceInstruction::DeployTokenManager {
        params,
        optional_accounts_mask,
    })?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::InterchainTransfer`]
/// instruction.
///
/// # Errors
///
/// If serialization fails.
pub fn interchain_transfer(
    mut params: InterchainTransferInputs,
) -> Result<Instruction, ProgramError> {
    let (accounts, signing_pda_bump) = interchain_transfer_accounts(&params)?;

    params.signing_pda_bump = Some(signing_pda_bump);
    let data = to_vec(&InterchainTokenServiceInstruction::InterchainTransfer { params })?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::CallContractWithInterchainToken`]
/// instruction.
///
/// # Errors
///
/// If serialization fails.
pub fn call_contract_with_interchain_token(
    mut params: CallContractWithInterchainTokenInputs,
) -> Result<Instruction, ProgramError> {
    let (accounts, signing_pda_bump) = interchain_transfer_accounts(&params)?;

    params.signing_pda_bump = Some(signing_pda_bump);
    let data =
        to_vec(&InterchainTokenServiceInstruction::CallContractWithInterchainToken { params })?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::CallContractWithInterchainTokenOffchainData`]
/// instruction.
///
/// # Errors
///
/// If serialization fails.
pub fn call_contract_with_interchain_token_offchain_data(
    mut params: CallContractWithInterchainTokenInputs,
) -> Result<(Instruction, Vec<u8>), ProgramError> {
    let (accounts, signing_pda_bump) = interchain_transfer_accounts(&params)?;

    let Some(destination_chain) = params.destination_chain.as_ref() else {
        return Err(ProgramError::InvalidArgument);
    };

    let inner_gmp_payload: GMPPayload = params.clone().try_into()?;
    let hub_payload = GMPPayload::SendToHub(SendToHub {
        selector: SendToHub::MESSAGE_TYPE_ID
            .try_into()
            .map_err(|_err| ProgramError::ArithmeticOverflow)?,
        destination_chain: destination_chain.clone(),
        payload: inner_gmp_payload.encode().into(),
    });
    let offchain_data = hub_payload.encode();

    params.payload_hash = Some(solana_program::keccak::hashv(&[&offchain_data]).0);
    params.signing_pda_bump = Some(signing_pda_bump);

    let data = to_vec(
        &InterchainTokenServiceInstruction::CallContractWithInterchainTokenOffchainData { params },
    )?;

    Ok((
        Instruction {
            program_id: crate::ID,
            accounts,
            data,
        },
        offchain_data,
    ))
}

fn interchain_transfer_accounts(
    inputs: &InterchainTransferInputs,
) -> Result<(Vec<AccountMeta>, u8), ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let (interchain_token_pda, _) =
        crate::find_interchain_token_pda(&its_root_pda, &inputs.token_id);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &inputs.token_id);
    let flow_epoch = flow_limit::flow_epoch_with_timestamp(inputs.timestamp)?;
    let (flow_slot_pda, _) = crate::find_flow_slot_pda(&token_manager_pda, flow_epoch);

    let (authority, signer) = match inputs.authority {
        Some(key) => (
            Pubkey::new_from_array(
                key.as_ref()
                    .try_into()
                    .map_err(|_err| ProgramError::InvalidInstructionData)?,
            ),
            true,
        ),
        None => (token_manager_pda, false),
    };
    let source_account = Pubkey::new_from_array(
        inputs
            .source_account
            .as_ref()
            .try_into()
            .map_err(|_err| ProgramError::InvalidInstructionData)?,
    );
    let mint = match inputs.mint {
        Some(key) => Pubkey::new_from_array(
            key.as_ref()
                .try_into()
                .map_err(|_err| ProgramError::InvalidInstructionData)?,
        ),
        None => interchain_token_pda,
    };
    let token_program = Pubkey::new_from_array(
        inputs
            .token_program
            .as_ref()
            .try_into()
            .map_err(|_err| ProgramError::InvalidInstructionData)?,
    );

    let payer = Pubkey::new_from_array(
        inputs
            .payer
            .as_ref()
            .try_into()
            .map_err(|_err| ProgramError::InvalidInstructionData)?,
    );

    let token_manager_ata =
        get_associated_token_address_with_program_id(&token_manager_pda, &mint, &token_program);
    let (call_contract_signing_pda, signing_pda_bump) =
        axelar_solana_gateway::get_call_contract_signing_pda(crate::ID);

    Ok((
        vec![
            AccountMeta::new_readonly(payer, true),
            AccountMeta::new_readonly(authority, signer),
            AccountMeta::new_readonly(gateway_root_pda, false),
            AccountMeta::new_readonly(axelar_solana_gateway::id(), false),
            AccountMeta::new(inputs.gas_config_pda, false),
            AccountMeta::new_readonly(inputs.gas_service, false),
            AccountMeta::new_readonly(system_program::id(), false),
            AccountMeta::new_readonly(its_root_pda, false),
            AccountMeta::new_readonly(call_contract_signing_pda, false),
            AccountMeta::new_readonly(crate::ID, false),
            AccountMeta::new(source_account, false),
            AccountMeta::new(mint, false),
            AccountMeta::new_readonly(token_manager_pda, false),
            AccountMeta::new(token_manager_ata, false),
            AccountMeta::new_readonly(token_program, false),
            AccountMeta::new(flow_slot_pda, false),
        ],
        signing_pda_bump,
    ))
}

/// Creates an [`InterchainTokenServiceInstruction::SetFlowLimit`].
///
/// # Errors
///
/// If serialization fails.
pub fn set_flow_limit(
    payer: Pubkey,
    token_id: [u8; 32],
    flow_limit: u64,
) -> Result<Instruction, ProgramError> {
    let (its_root_pda, _) =
        crate::find_its_root_pda(&axelar_solana_gateway::get_gateway_root_config_pda().0);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);

    let (its_user_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::id(), &its_root_pda, &payer);
    let (token_manager_user_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::id(), &token_manager_pda, &its_root_pda);

    let data = to_vec(&InterchainTokenServiceInstruction::SetFlowLimit { flow_limit })?;
    let accounts = vec![
        AccountMeta::new_readonly(payer, true),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new(token_manager_pda, false),
        AccountMeta::new_readonly(its_user_roles_pda, false),
        AccountMeta::new_readonly(token_manager_user_roles_pda, false),
    ];

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::ItsGmpPayload`] instruction.
///
/// # Errors
///
/// If serialization fails.
pub fn its_gmp_payload(inputs: ItsGmpInstructionInputs) -> Result<Instruction, ProgramError> {
    let mut accounts = prefix_accounts(
        &inputs.payer,
        &inputs.incoming_message_pda,
        &inputs.message_payload_pda,
        &inputs.message,
    );
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();

    let unwrapped_payload = match inputs.payload {
        GMPPayload::InterchainTransfer(_)
        | GMPPayload::DeployInterchainToken(_)
        | GMPPayload::DeployTokenManager(_) => inputs.payload,
        GMPPayload::SendToHub(inner) => GMPPayload::decode(&inner.payload)
            .map_err(|_err| ProgramError::InvalidInstructionData)?,
        GMPPayload::ReceiveFromHub(inner) => GMPPayload::decode(&inner.payload)
            .map_err(|_err| ProgramError::InvalidInstructionData)?,
    };

    let (mut its_accounts, optional_accounts_mask) = derive_its_accounts(
        &unwrapped_payload,
        gateway_root_pda,
        inputs.token_program,
        inputs.mint,
        inputs.timestamp,
    )?;

    accounts.append(&mut its_accounts);

    let data = to_vec(&InterchainTokenServiceInstruction::ItsGmpPayload {
        message: inputs.message,
        optional_accounts_mask,
    })?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::OperatorInstruction`]
/// instruction with the [`operator::Instruction::TransferOperatorship`]
/// variant.
///
/// # Errors
///
/// If serialization fails.
pub fn transfer_operatorship(payer: Pubkey, to: Pubkey) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let accounts = vec![AccountMeta::new_readonly(gateway_root_pda, false)];
    let (accounts, operator_instruction) =
        operator::transfer_operatorship(payer, its_root_pda, to, Some(accounts))?;
    let data = to_vec(&InterchainTokenServiceInstruction::OperatorInstruction(
        operator_instruction,
    ))?;

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::OperatorInstruction`]
/// instruction with the [`operator::Instruction::ProposeOperatorship`] variant.
///
/// # Errors
///
/// If serialization fails.
pub fn propose_operatorship(payer: Pubkey, to: Pubkey) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let accounts = vec![AccountMeta::new_readonly(gateway_root_pda, false)];
    let (accounts, operator_instruction) =
        operator::propose_operatorship(payer, its_root_pda, to, Some(accounts))?;
    let data = to_vec(&InterchainTokenServiceInstruction::OperatorInstruction(
        operator_instruction,
    ))?;

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::OperatorInstruction`]
/// instruction with the [`operator::Instruction::AcceptOperatorship`] variant.
///
/// # Errors
///
/// If serialization fails.
pub fn accept_operatorship(payer: Pubkey, from: Pubkey) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let accounts = vec![AccountMeta::new_readonly(gateway_root_pda, false)];
    let (accounts, operator_instruction) =
        operator::accept_operatorship(payer, its_root_pda, from, Some(accounts))?;
    let data = to_vec(&InterchainTokenServiceInstruction::OperatorInstruction(
        operator_instruction,
    ))?;

    Ok(Instruction {
        program_id: crate::id(),
        accounts,
        data,
    })
}

fn prefix_accounts(
    payer: &Pubkey,
    gateway_incoming_message_pda: &Pubkey,
    gateway_message_payload_pda: &Pubkey,
    message: &Message,
) -> Vec<AccountMeta> {
    let command_id = command_id(&message.cc_id.chain, &message.cc_id.id);
    let destination_program = DestinationProgramId(crate::id());
    let (gateway_approved_message_signing_pda, _) = destination_program.signing_pda(&command_id);

    vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(*gateway_incoming_message_pda, false),
        AccountMeta::new_readonly(*gateway_message_payload_pda, false),
        AccountMeta::new_readonly(gateway_approved_message_signing_pda, false),
        AccountMeta::new_readonly(axelar_solana_gateway::id(), false),
    ]
}

pub(crate) fn derive_its_accounts<'a, T>(
    payload: T,
    gateway_root_pda: Pubkey,
    token_program: Pubkey,
    maybe_mint: Option<Pubkey>,
    maybe_timestamp: Option<i64>,
) -> Result<(Vec<AccountMeta>, OptionalAccountsFlags), ProgramError>
where
    T: TryInto<ItsMessageRef<'a>, Error = ProgramError>,
{
    let message: ItsMessageRef<'_> = payload.try_into()?;
    if let ItsMessageRef::DeployInterchainToken { .. } = message {
        if token_program != spl_token_2022::id() {
            return Err(ProgramError::InvalidInstructionData);
        }
    }

    let (mut accounts, mint, token_manager_pda) =
        derive_common_its_accounts(gateway_root_pda, token_program, &message, maybe_mint)?;
    let (mut message_specific_accounts, optional_accounts_mask) = derive_specific_its_accounts(
        &message,
        mint,
        token_manager_pda,
        token_program,
        maybe_timestamp,
    )?;

    accounts.append(&mut message_specific_accounts);

    Ok((accounts, optional_accounts_mask))
}

fn derive_specific_its_accounts(
    message: &ItsMessageRef<'_>,
    mint_account: Pubkey,
    token_manager_pda: Pubkey,
    token_program: Pubkey,
    maybe_timestamp: Option<i64>,
) -> Result<(Vec<AccountMeta>, OptionalAccountsFlags), ProgramError> {
    let mut specific_accounts = Vec::new();
    let mut optional_accounts_mask = OptionalAccountsFlags::empty();

    match message {
        ItsMessageRef::InterchainTransfer {
            destination_address,
            data,
            ..
        } => {
            let destination_wallet = Pubkey::new_from_array(
                (*destination_address)
                    .try_into()
                    .map_err(|_err| ProgramError::InvalidInstructionData)?,
            );
            let destination_ata = get_associated_token_address_with_program_id(
                &destination_wallet,
                &mint_account,
                &token_program,
            );
            let Some(timestamp) = maybe_timestamp else {
                return Err(ProgramError::InvalidInstructionData);
            };
            let epoch = crate::state::flow_limit::flow_epoch_with_timestamp(timestamp)?;
            let (flow_slot_pda, _) = crate::find_flow_slot_pda(&token_manager_pda, epoch);

            let (metadata_account_key, _) =
                mpl_token_metadata::accounts::Metadata::find_pda(&mint_account);

            specific_accounts.push(AccountMeta::new(destination_wallet, false));
            specific_accounts.push(AccountMeta::new(destination_ata, false));
            specific_accounts.push(AccountMeta::new(flow_slot_pda, false));

            if !data.is_empty() {
                let execute_data = DataPayload::decode(data)
                    .map_err(|_err| ProgramError::InvalidInstructionData)?;

                specific_accounts.push(AccountMeta::new_readonly(mpl_token_metadata::ID, false));
                specific_accounts.push(AccountMeta::new(metadata_account_key, false));
                specific_accounts.extend(execute_data.account_meta().iter().cloned());
            }
        }
        ItsMessageRef::DeployInterchainToken { minter, .. } => {
            let (metadata_account_key, _) =
                mpl_token_metadata::accounts::Metadata::find_pda(&mint_account);

            specific_accounts.push(AccountMeta::new_readonly(sysvar::instructions::id(), false));
            specific_accounts.push(AccountMeta::new_readonly(mpl_token_metadata::ID, false));
            specific_accounts.push(AccountMeta::new(metadata_account_key, false));

            if minter.len() == axelar_solana_encoding::types::pubkey::ED25519_PUBKEY_LEN {
                let minter_key = Pubkey::new_from_array(
                    (*minter)
                        .try_into()
                        .map_err(|_err| ProgramError::InvalidInstructionData)?,
                );
                let (minter_roles_pda, _) = role_management::find_user_roles_pda(
                    &crate::id(),
                    &token_manager_pda,
                    &minter_key,
                );

                optional_accounts_mask |=
                    OptionalAccountsFlags::MINTER | OptionalAccountsFlags::MINTER_ROLES;

                specific_accounts.push(AccountMeta::new_readonly(minter_key, false));
                specific_accounts.push(AccountMeta::new(minter_roles_pda, false));
            }
        }
        ItsMessageRef::DeployTokenManager { params, .. } => {
            let (maybe_operator, maybe_mint_authority, _) =
                state::token_manager::decode_params(params)
                    .map_err(|_err| ProgramError::InvalidInstructionData)?;

            if let Some(mint_authority) = maybe_mint_authority {
                let (mint_authority_roles_pda, _) = role_management::find_user_roles_pda(
                    &crate::id(),
                    &token_manager_pda,
                    &mint_authority,
                );

                optional_accounts_mask |=
                    OptionalAccountsFlags::MINTER | OptionalAccountsFlags::MINTER_ROLES;

                specific_accounts.push(AccountMeta::new_readonly(mint_authority, false));
                specific_accounts.push(AccountMeta::new(mint_authority_roles_pda, false));
            }

            if let Some(operator) = maybe_operator {
                let (operator_roles_pda, _) = role_management::find_user_roles_pda(
                    &crate::id(),
                    &token_manager_pda,
                    &operator,
                );

                optional_accounts_mask |=
                    OptionalAccountsFlags::OPERATOR | OptionalAccountsFlags::OPERATOR_ROLES;

                specific_accounts.push(AccountMeta::new_readonly(operator, false));
                specific_accounts.push(AccountMeta::new(operator_roles_pda, false));
            }
        }
    };

    Ok((specific_accounts, optional_accounts_mask))
}

fn try_retrieve_mint(
    interchain_token_pda: &Pubkey,
    payload: &ItsMessageRef<'_>,
    maybe_mint: Option<Pubkey>,
) -> Result<Pubkey, ProgramError> {
    if let Some(mint) = maybe_mint {
        return Ok(mint);
    }

    match payload {
        ItsMessageRef::DeployTokenManager { params, .. } => {
            let token_mint = state::token_manager::decode_params(params)
                .map(|(_, _, token_mint)| Pubkey::try_from(token_mint.as_ref()))?
                .map_err(|_err| ProgramError::InvalidInstructionData)?;

            Ok(token_mint)
        }
        ItsMessageRef::InterchainTransfer { .. } => {
            maybe_mint.ok_or(ProgramError::InvalidInstructionData)
        }
        ItsMessageRef::DeployInterchainToken { .. } => Ok(*interchain_token_pda),
    }
}

fn derive_common_its_accounts(
    gateway_root_pda: Pubkey,
    token_program: Pubkey,
    message: &ItsMessageRef<'_>,
    maybe_mint: Option<Pubkey>,
) -> Result<(Vec<AccountMeta>, Pubkey, Pubkey), ProgramError> {
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let (interchain_token_pda, _) =
        crate::find_interchain_token_pda(&its_root_pda, message.token_id());
    let token_mint = try_retrieve_mint(&interchain_token_pda, message, maybe_mint)?;
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, message.token_id());

    let token_manager_ata = get_associated_token_address_with_program_id(
        &token_manager_pda,
        &token_mint,
        &token_program,
    );

    let (its_user_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::id(), &token_manager_pda, &its_root_pda);

    Ok((
        vec![
            AccountMeta::new_readonly(gateway_root_pda, false),
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(its_root_pda, false),
            AccountMeta::new(token_manager_pda, false),
            AccountMeta::new(token_mint, false),
            AccountMeta::new(token_manager_ata, false),
            AccountMeta::new_readonly(token_program, false),
            AccountMeta::new_readonly(spl_associated_token_account::id(), false),
            AccountMeta::new(its_user_roles_pda, false),
            AccountMeta::new_readonly(sysvar::rent::id(), false),
        ],
        token_mint,
        token_manager_pda,
    ))
}

pub(crate) trait OutboundInstructionInputs {
    fn destination_chain(&mut self) -> Option<String>;
    fn gas_value(&self) -> u64;
    fn signing_pda_bump(&self) -> Option<u8>;
}

impl OutboundInstructionInputs for DeployInterchainTokenInputs {
    fn destination_chain(&mut self) -> Option<String> {
        self.destination_chain.take()
    }

    fn gas_value(&self) -> u64 {
        self.gas_value
    }

    fn signing_pda_bump(&self) -> Option<u8> {
        self.signing_pda_bump
    }
}

impl OutboundInstructionInputs for DeployTokenManagerInputs {
    fn destination_chain(&mut self) -> Option<String> {
        self.destination_chain.take()
    }

    fn gas_value(&self) -> u64 {
        self.gas_value
    }
    fn signing_pda_bump(&self) -> Option<u8> {
        self.signing_pda_bump
    }
}

#[allow(dead_code)]
pub(crate) enum ItsMessageRef<'a> {
    InterchainTransfer {
        token_id: Cow<'a, [u8; 32]>,
        source_address: &'a [u8],
        destination_address: &'a [u8],
        amount: u64,
        data: &'a [u8],
    },
    DeployInterchainToken {
        token_id: Cow<'a, [u8; 32]>,
        name: &'a str,
        symbol: &'a str,
        decimals: u8,
        minter: &'a [u8],
    },
    DeployTokenManager {
        token_id: Cow<'a, [u8; 32]>,
        token_manager_type: state::token_manager::Type,
        params: &'a [u8],
    },
}

impl ItsMessageRef<'_> {
    /// Returns the token id for the message.
    pub(crate) fn token_id(&self) -> &[u8; 32] {
        match self {
            ItsMessageRef::InterchainTransfer { token_id, .. }
            | ItsMessageRef::DeployInterchainToken { token_id, .. }
            | ItsMessageRef::DeployTokenManager { token_id, .. } => token_id,
        }
    }
}

impl<'a> TryFrom<&'a GMPPayload> for ItsMessageRef<'a> {
    type Error = ProgramError;
    fn try_from(value: &'a GMPPayload) -> Result<Self, Self::Error> {
        Ok(match value {
            GMPPayload::InterchainTransfer(inner) => Self::InterchainTransfer {
                token_id: Cow::Borrowed(&inner.token_id.0),
                source_address: &inner.source_address.0,
                destination_address: inner.destination_address.as_ref(),
                amount: inner
                    .amount
                    .try_into()
                    .map_err(|_err| ProgramError::InvalidInstructionData)?,
                data: inner.data.as_ref(),
            },
            GMPPayload::DeployInterchainToken(inner) => Self::DeployInterchainToken {
                token_id: Cow::Borrowed(&inner.token_id.0),
                name: &inner.name,
                symbol: &inner.symbol,
                decimals: inner.decimals,
                minter: inner.minter.as_ref(),
            },
            GMPPayload::DeployTokenManager(inner) => Self::DeployTokenManager {
                token_id: Cow::Borrowed(&inner.token_id.0),
                token_manager_type: inner
                    .token_manager_type
                    .try_into()
                    .map_err(|_err| ProgramError::InvalidInstructionData)?,
                params: inner.params.as_ref(),
            },
            GMPPayload::SendToHub(_) | GMPPayload::ReceiveFromHub(_) => {
                return Err(ProgramError::InvalidArgument)
            }
        })
    }
}

impl<'a> TryFrom<&'a DeployInterchainTokenInputs> for ItsMessageRef<'a> {
    type Error = ProgramError;

    fn try_from(value: &'a DeployInterchainTokenInputs) -> Result<Self, Self::Error> {
        let token_id = crate::interchain_token_id(
            &Pubkey::new_from_array(
                value
                    .payer
                    .as_ref()
                    .try_into()
                    .map_err(|_err| ProgramError::InvalidInstructionData)?,
            ),
            &value.salt,
        );

        Ok(Self::DeployInterchainToken {
            token_id: Cow::Owned(token_id),
            name: &value.name,
            symbol: &value.symbol,
            decimals: value.decimals,
            minter: value.minter.as_ref(),
        })
    }
}

impl<'a> TryFrom<&'a DeployTokenManagerInputs> for ItsMessageRef<'a> {
    type Error = ProgramError;

    fn try_from(value: &'a DeployTokenManagerInputs) -> Result<Self, Self::Error> {
        let token_id = crate::interchain_token_id(
            &Pubkey::new_from_array(
                value
                    .payer
                    .as_ref()
                    .try_into()
                    .map_err(|_err| ProgramError::InvalidInstructionData)?,
            ),
            &value.salt,
        );

        Ok(Self::DeployTokenManager {
            token_id: Cow::Owned(token_id),
            token_manager_type: value.token_manager_type,
            params: value.params.as_ref(),
        })
    }
}

impl<'a> TryFrom<&'a InterchainTransferInputs> for ItsMessageRef<'a> {
    type Error = ProgramError;

    fn try_from(value: &'a InterchainTransferInputs) -> Result<Self, Self::Error> {
        Ok(Self::InterchainTransfer {
            token_id: Cow::Borrowed(&value.token_id),
            source_address: value.source_account.as_ref(),
            destination_address: value.destination_address.as_ref(),
            amount: value.amount,
            data: &value.data,
        })
    }
}

impl TryFrom<DeployInterchainTokenInputs> for DeployInterchainToken {
    type Error = ProgramError;

    fn try_from(value: DeployInterchainTokenInputs) -> Result<Self, Self::Error> {
        let token_id = crate::interchain_token_id(
            &Pubkey::new_from_array(
                value
                    .payer
                    .as_ref()
                    .try_into()
                    .map_err(|_err| ProgramError::InvalidInstructionData)?,
            ),
            value.salt.as_slice(),
        );

        Ok(Self {
            selector: alloy_primitives::U256::from(1_u8),
            token_id: token_id.into(),
            name: value.name,
            symbol: value.symbol,
            decimals: value.decimals,
            minter: value.minter.into(),
        })
    }
}

impl TryFrom<DeployTokenManagerInputs> for DeployTokenManager {
    type Error = ProgramError;

    fn try_from(value: DeployTokenManagerInputs) -> Result<Self, Self::Error> {
        let token_id = crate::interchain_token_id(
            &Pubkey::new_from_array(
                value
                    .payer
                    .as_ref()
                    .try_into()
                    .map_err(|_err| ProgramError::InvalidInstructionData)?,
            ),
            value.salt.as_slice(),
        );

        Ok(Self {
            selector: alloy_primitives::U256::from(2_u8),
            token_id: token_id.into(),
            token_manager_type: value.token_manager_type.into(),
            params: value.params.into(),
        })
    }
}

impl TryFrom<InterchainTransferInputs> for InterchainTransfer {
    type Error = ProgramError;

    fn try_from(value: InterchainTransferInputs) -> Result<Self, Self::Error> {
        Ok(Self {
            selector: alloy_primitives::U256::from(0_u8),
            token_id: value.token_id.into(),
            source_address: value.source_account.as_ref().to_vec().into(),
            destination_address: value.destination_address.into(),
            amount: alloy_primitives::U256::from(value.amount),
            data: value.data.into(),
        })
    }
}

impl TryFrom<DeployInterchainTokenInputs> for GMPPayload {
    type Error = ProgramError;

    fn try_from(value: DeployInterchainTokenInputs) -> Result<Self, Self::Error> {
        let inner = DeployInterchainToken::try_from(value)?;
        Ok(inner.into())
    }
}

impl TryFrom<DeployTokenManagerInputs> for GMPPayload {
    type Error = ProgramError;

    fn try_from(value: DeployTokenManagerInputs) -> Result<Self, Self::Error> {
        let inner = DeployTokenManager::try_from(value)?;
        Ok(inner.into())
    }
}

impl TryFrom<InterchainTransferInputs> for GMPPayload {
    type Error = ProgramError;

    fn try_from(value: InterchainTransferInputs) -> Result<Self, Self::Error> {
        let inner = InterchainTransfer::try_from(value)?;
        Ok(inner.into())
    }
}

impl TryFrom<RoleManagementInstruction<Roles>> for InterchainTokenServiceInstruction {
    type Error = ProgramError;

    fn try_from(value: RoleManagementInstruction<Roles>) -> Result<Self, Self::Error> {
        match value {
            // Adding and removing operators on the InterchainTokenService is not supported.
            RoleManagementInstruction::AddRoles(_) | RoleManagementInstruction::RemoveRoles(_) => {
                Err(ProgramError::InvalidInstructionData)
            }
            RoleManagementInstruction::TransferRoles(_)
            | RoleManagementInstruction::ProposeRoles(_)
            | RoleManagementInstruction::AcceptRoles(_) => {
                Ok(Self::OperatorInstruction(value.try_into()?))
            }
        }
    }
}
