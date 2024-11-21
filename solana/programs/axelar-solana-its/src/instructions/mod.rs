//! Instructions supported by the multicall program.

use std::borrow::Cow;
use std::error::Error;

use axelar_message_primitives::{DataPayload, DestinationProgramId, U256};
use axelar_rkyv_encoding::types::{GmpMetadata, PublicKey};
use gateway::hasher_impl;
use interchain_token_transfer_gmp::{
    DeployInterchainToken, DeployTokenManager, GMPPayload, InterchainTransfer,
};
use rkyv::bytecheck::EnumCheckError;
use rkyv::validation::validators::DefaultValidatorError;
use rkyv::{bytecheck, Archive, CheckBytes, Deserialize, Serialize};
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::{system_program, sysvar};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use typed_builder::TypedBuilder;

use crate::state::{self, flow_limit};

pub mod token_manager;

/// Convenience module with the indices of the accounts passed in the
/// [`ItsGmpPayload`] instruction (offset by the prefixed GMP accounts).
pub mod its_account_indices {
    /// The index of the system program account.
    pub const SYSTEM_PROGRAM_INDEX: usize = 0;

    /// The index of the ITS root PDA account.
    pub const ITS_ROOT_PDA_INDEX: usize = 1;

    /// The index of the token manager PDA account.
    pub const TOKEN_MANAGER_PDA_INDEX: usize = 2;

    /// The index of the token mint account.
    pub const TOKEN_MINT_INDEX: usize = 3;

    /// The index of the token manager ATA account.
    pub const TOKEN_MANAGER_ATA_INDEX: usize = 4;

    /// The index of the token program account.
    pub const TOKEN_PROGRAM_INDEX: usize = 5;

    /// The index of the associated token program account.
    pub const SPL_ASSOCIATED_TOKEN_ACCOUNT_INDEX: usize = 6;
}

/// Instructions supported by the multicall program.
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub enum InterchainTokenServiceInstruction {
    /// Initializes the interchain token service program.
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. [writeable,signer] The address of payer / sender
    /// 1. [] gateway root pda
    /// 2. [writeable] ITS root pda
    /// 3. [] system program id
    /// 4. [] The account that will become the operator of the ITS
    /// 5. [writeable] The address of the PDA account that will store the roles
    ///    of the operator account.
    Initialize {
        /// The pda bump for the ITS root PDA
        its_root_pda_bump: u8,

        /// The bump PDA used to store the operator role for the ITS
        user_roles_pda_bump: u8,
    },

    /// Deploys an interchain token.
    ///
    /// 0. [writeable,signer] The address of payer / sender
    /// 1. [] Gateway root pda
    /// 2. [] System program id if local deployment OR Gateway program if remote
    ///    deployment
    /// 3. [] ITS root pda
    /// 4. [writeable] Token Manager PDA (if local deployment)
    /// 5. [writeable] The mint account to be created (if local deployment)
    /// 6. [writeable] The Token Manager ATA (if local deployment)
    /// 7. [] Token program id (if local deployment)
    /// 8. [] Associated token program id (if local deployment)
    /// 9. [] Rent sysvar (if local deployment)
    /// 10. [] The minter account (if local deployment)
    DeployInterchainToken {
        /// The deploy params containing token metadata as well as other
        /// required inputs.
        params: DeployInterchainTokenInputs,

        /// The PDA bumps for the ITS accounts, required if deploying the token
        /// on the local chain.
        bumps: Option<Bumps>,
    },

    /// Deploys a token manager.
    ///
    /// 0. [writeable,signer] The address of payer / sender
    /// 1. [] Gateway root pda
    /// 2. [] System program id if local deployment OR Gateway program if remote
    ///    deployment
    /// 3. [] ITS root pda
    /// 4. [writeable] Token Manager PDA (if local deployment)
    /// 5. [writeable] The mint account to be created (if local deployment)
    /// 6. [writeable] The Token Manager ATA (if local deployment)
    /// 7. [] Token program id (if local deployment)
    /// 8. [] Associated token program id (if local deployment)
    /// 9. [] Rent sysvar (if local deployment)
    /// 10. [] The minter account (if local deployment)
    DeployTokenManager {
        /// The deploy params containing token metadata as well as other
        /// required inputs.
        params: DeployTokenManagerInputs,

        /// The PDA bumps for the ITS accounts, required if deploying the token
        /// on the local chain.
        bumps: Option<Bumps>,
    },

    /// Transfers interchain tokens.
    ///
    /// 0. [maybe signer] The address of the authority signing the transfer. In
    ///    case it's the `TokenManager`, it shouldn't be set as signer as the
    ///    signing happens on chain.
    /// 1. [] Gateway root pda
    /// 2. [] Gateway program id
    /// 3. [] ITS root pda
    /// 4. [writeable] Interchain token PDA
    /// 5. [writeable] The account where the tokens are being transferred from
    /// 5. [writeable] The mint account
    /// 6. [writeable] The Token Manager PDA
    /// 6. [writeable] The Token Manager ATA
    /// 7. [] Token program id
    InterchainTransfer {
        /// The transfer parameters.
        params: InterchainTransferInputs,

        /// The PDA bumps for the ITS accounts.
        bumps: Bumps,
    },

    /// A GMP Interchain Token Service instruction.
    ///
    /// 0. [writeable,signer] The address of payer / sender
    /// 1. [] gateway root pda
    /// 2. [] ITS root pda
    ///
    /// 3..N Accounts depend on the inner ITS instruction.
    ItsGmpPayload {
        /// The GMP metadata
        gmp_metadata: GmpMetadata,

        /// The GMP payload
        abi_payload: Vec<u8>,

        /// The PDA bumps for the ITS accounts
        bumps: Bumps,
    },

    /// Sets the flow limit for an interchain token.
    SetFlowLimit {
        /// The new flow limit.
        flow_limit: u64,
    },

    /// A proxy instruction to mint tokens whose mint authority is a
    /// `TokenManager`. Only users with the `minter` role on the mint account
    /// can mint tokens.
    ///
    /// 0. [writeable] The mint account
    /// 1. [writeable] The account to mint tokens to
    /// 2. [] The interchain token PDA associated with the mint
    /// 3. [] The token manager PDA
    /// 4. [signer] The minter account
    /// 5. [] The token program id
    MintTo {
        /// The amount of tokens to mint
        amount: u64,
    },

    /// Instructions operating on deployed [`TokenManager`] instances.
    TokenManagerInstruction(token_manager::Instruction),
}

impl InterchainTokenServiceInstruction {
    /// Serializes the instruction into a byte array.
    ///
    /// # Errors
    ///
    /// If serialization fails.
    pub fn to_bytes(&self) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
        let bytes = rkyv::to_bytes::<_, 0>(self).map_err(Box::new)?;

        Ok(bytes.to_vec())
    }

    /// Deserializes the instruction from a byte array.
    ///
    /// # Errors
    ///
    /// If deserialization fails.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, Box<dyn Error + Send + Sync>> {
        // SAFETY:
        // - The byte slice represents an archived object
        // - The root of the object is stored at the end of the slice
        let bytes = unsafe { rkyv::from_bytes_unchecked::<Self>(bytes) }.map_err(Box::new)?;

        Ok(bytes)
    }
}

impl ArchivedInterchainTokenServiceInstruction {
    /// Interprets the given slice as an archived instruction.
    ///
    /// # Errors
    ///
    /// If validation fails.
    pub fn from_archived_bytes(
        bytes: &[u8],
    ) -> Result<&Self, rkyv::validation::CheckArchiveError<EnumCheckError<u8>, DefaultValidatorError>>
    {
        rkyv::check_archived_root::<InterchainTokenServiceInstruction>(bytes)
    }
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
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone, TypedBuilder)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct DeployInterchainTokenInputs {
    /// The payer account for this transaction
    #[builder(setter(transform = |key: Pubkey| PublicKey::new_ed25519(key.to_bytes())))]
    pub(crate) payer: PublicKey,

    /// The salt used to derive the tokenId associated with the token
    pub(crate) salt: [u8; 32],

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
    #[builder(setter(transform = |x: u128| U256::from(x)))]
    pub(crate) gas_value: U256,
}

/// Parameters for `[InterchainTokenServiceInstruction::DeployTokenManager]`.
///
/// To construct this type, use its builder API.
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone, TypedBuilder)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct DeployTokenManagerInputs {
    /// The payer account for this transaction
    #[builder(setter(transform = |key: Pubkey| PublicKey::new_ed25519(key.to_bytes())))]
    pub(crate) payer: PublicKey,

    /// The salt used to derive the tokenId associated with the token
    pub(crate) salt: [u8; 32],

    /// The chain where the `TokenManager` should be deployed.
    /// Deploys to Solana if `None`.
    #[builder(default, setter(strip_option))]
    pub(crate) destination_chain: Option<String>,

    /// Token manager type
    pub(crate) token_manager_type: state::token_manager::Type,

    /// Chain specific params for the token manager
    pub(crate) params: Vec<u8>,

    /// The gas value to be paid for the deploy transaction
    #[builder(setter(transform = |x: u128| U256::from(x)))]
    pub(crate) gas_value: U256,

    /// Required when deploying the [`TokenManager`] on Solana, this is the
    /// token program that owns the mint account, either `spl_token::id()` or
    /// `spl_token_2022::id()`.
    #[builder(default, setter(transform = |key: Pubkey| Some(PublicKey::new_ed25519(key.to_bytes()))))]
    pub(crate) token_program: Option<PublicKey>,
}

/// Parameters for `[InterchainTokenServiceInstruction::InterchainTransfer]`.
///
/// To construct this type, use its builder API.
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone, TypedBuilder)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct InterchainTransferInputs {
    /// The payer account for this transaction.
    #[builder(setter(transform = |key: Pubkey| PublicKey::new_ed25519(key.to_bytes())))]
    pub(crate) payer: PublicKey,

    /// The source account.
    #[builder(setter(transform = |key: Pubkey| PublicKey::new_ed25519(key.to_bytes())))]
    pub(crate) source_account: PublicKey,

    /// The source account owner. In case of a transfer using a Mint/BurnFrom
    /// `TokenManager`, this shouldn't be set as the authority will be the
    /// `TokenManager`.
    #[builder(default, setter(transform = |key: Pubkey| Some(PublicKey::new_ed25519(key.to_bytes()))))]
    pub(crate) authority: Option<PublicKey>,

    /// The token id associated with the token
    pub(crate) token_id: [u8; 32],

    /// The token mint account. **This should be set in case the token is not
    /// ITS native**.
    ///
    /// When not set, the account is derived from the given `token_id`. The
    /// derived account is invalid if the token is not an ITS native token (not
    /// originally created/deployed by ITS).
    #[builder(default, setter(transform = |key: Pubkey| Some(PublicKey::new_ed25519(key.to_bytes()))))]
    pub(crate) mint: Option<PublicKey>,

    /// The chain where the tokens are being transferred to.
    #[builder(setter(strip_option))]
    pub(crate) destination_chain: Option<String>,

    /// The address on the destination chain to send the tokens to.
    pub(crate) destination_address: Vec<u8>,

    pub(crate) amount: u64,

    /// Optional metadata for the call for additional effects (such as calling a
    /// destination contract).
    pub(crate) metadata: Vec<u8>,

    /// The gas value to be paid for the deploy transaction
    #[builder(setter(transform = |x: u128| U256::from(x)))]
    pub(crate) gas_value: U256,

    /// Current chain's unix timestamp.
    pub(crate) timestamp: i64,

    /// The token program that owns the mint account, either `spl_token::id()`
    /// or `spl_token_2022::id()`. Assumes `spl_token_2022::id()` if not set.
    #[builder(default = PublicKey::new_ed25519(spl_token_2022::id().to_bytes()), setter(transform = |key: Pubkey| PublicKey::new_ed25519(key.to_bytes())))]
    pub(crate) token_program: PublicKey,
}

/// Bumps for the ITS PDA accounts.
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone, Copy, Default)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
pub struct Bumps {
    /// The bump for the ITS root PDA.
    pub its_root_pda_bump: u8,

    /// The bump for the interchain token PDA.
    pub interchain_token_pda_bump: u8,

    /// The bump for the token manager PDA.
    pub token_manager_pda_bump: u8,

    /// The bump for the flow slot PDA.
    pub flow_slot_pda_bump: Option<u8>,

    /// The bump for the user roles PDA on ITS resource required for the
    /// instruction, if any.
    pub its_user_roles_pda_bump: Option<u8>,

    /// The bump for the user roles PDA on the [`TokenManager`] required for the
    /// instruction, if any.
    pub token_manager_user_roles_pda_bump: Option<u8>,
}

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
///   .gateway_approved_message_pda(gateway_approved_message_pda)
///   .gateway_root_pda(gateway_root_pda)
///   .gmp_metadata(metadata)
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

    /// The PDA that tracks the approval of this message by the gateway program.
    pub(crate) gateway_approved_message_pda: Pubkey,

    /// The root PDA for the gateway program.
    pub(crate) gateway_root_pda: Pubkey,

    /// The Axelar GMP metadata.
    pub(crate) gmp_metadata: GmpMetadata,

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

    /// Bumps used to derive the ITS accounts. If not set, the
    /// `find_program_address` is used which is more expensive.
    #[builder(default, setter(strip_option(fallback = bumps_opt)))]
    pub(crate) bumps: Option<Bumps>,
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
    let (its_root_pda, its_root_pda_bump) = crate::find_its_root_pda(&gateway_root_pda);
    let (user_roles_pda, user_roles_pda_bump) =
        role_management::find_user_roles_pda(&crate::id(), &its_root_pda, &operator);

    let instruction = InterchainTokenServiceInstruction::Initialize {
        its_root_pda_bump,
        user_roles_pda_bump,
    };

    let data = instruction
        .to_bytes()
        .map_err(|_err| ProgramError::InvalidInstructionData)?;

    let accounts = vec![
        AccountMeta::new(payer, true),
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

/// Creates an [`InterchainTokenServiceInstruction::DeployInterchainToken`]
/// instruction.
///
/// # Errors
///
/// If serialization fails.
pub fn deploy_interchain_token(
    params: DeployInterchainTokenInputs,
) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = gateway::get_gateway_root_config_pda();
    let payer = Pubkey::new_from_array(
        params
            .payer
            .as_ref()
            .try_into()
            .map_err(|_err| ProgramError::InvalidInstructionData)?,
    );
    let mut accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(gateway_root_pda, false),
    ];

    let bumps = if params.destination_chain.is_none() {
        let (mut its_accounts, bumps) = derive_its_accounts(
            &gateway_root_pda,
            &params,
            spl_token_2022::id(),
            None,
            None,
            None,
        )?;

        accounts.append(&mut its_accounts);

        Some(bumps)
    } else {
        let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);

        accounts.push(AccountMeta::new_readonly(gateway::id(), false));
        accounts.push(AccountMeta::new_readonly(its_root_pda, false));

        None
    };

    let data = InterchainTokenServiceInstruction::DeployInterchainToken { params, bumps }
        .to_bytes()
        .map_err(|_err| ProgramError::InvalidInstructionData)?;

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
pub fn deploy_token_manager(params: DeployTokenManagerInputs) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = gateway::get_gateway_root_config_pda();
    let payer = Pubkey::new_from_array(
        params
            .payer
            .as_ref()
            .try_into()
            .map_err(|_err| ProgramError::InvalidInstructionData)?,
    );
    let mut accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(gateway_root_pda, false),
    ];

    let bumps = if params.destination_chain.is_none() {
        let token_program = Pubkey::new_from_array(
            params
                .token_program
                .ok_or(ProgramError::InvalidInstructionData)?
                .as_ref()
                .try_into()
                .map_err(|_err| ProgramError::InvalidInstructionData)?,
        );

        let (mut its_accounts, bumps) =
            derive_its_accounts(&gateway_root_pda, &params, token_program, None, None, None)?;

        accounts.append(&mut its_accounts);

        Some(bumps)
    } else {
        let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);

        accounts.push(AccountMeta::new_readonly(gateway::id(), false));
        accounts.push(AccountMeta::new_readonly(its_root_pda, false));

        None
    };

    let data = InterchainTokenServiceInstruction::DeployTokenManager { params, bumps }
        .to_bytes()
        .map_err(|_err| ProgramError::InvalidInstructionData)?;

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
pub fn interchain_transfer(params: InterchainTransferInputs) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = gateway::get_gateway_root_config_pda();
    let (its_root_pda, its_root_pda_bump) = crate::find_its_root_pda(&gateway_root_pda);
    let (interchain_token_pda, interchain_token_pda_bump) =
        crate::find_interchain_token_pda(&its_root_pda, &params.token_id);
    let (token_manager_pda, token_manager_pda_bump) =
        crate::find_token_manager_pda(&interchain_token_pda);
    let flow_epoch = flow_limit::flow_epoch_with_timestamp(params.timestamp)?;
    let (flow_slot_pda, flow_slot_pda_bump) =
        crate::find_flow_slot_pda(&token_manager_pda, flow_epoch);

    let bumps = Bumps {
        its_root_pda_bump,
        interchain_token_pda_bump,
        token_manager_pda_bump,
        flow_slot_pda_bump: Some(flow_slot_pda_bump),
        ..Default::default()
    };
    let (authority, signer) = match params.authority {
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
        params
            .source_account
            .as_ref()
            .try_into()
            .map_err(|_err| ProgramError::InvalidInstructionData)?,
    );
    let mint = match params.mint {
        Some(key) => Pubkey::new_from_array(
            key.as_ref()
                .try_into()
                .map_err(|_err| ProgramError::InvalidInstructionData)?,
        ),
        None => interchain_token_pda,
    };
    let token_program = Pubkey::new_from_array(
        params
            .token_program
            .as_ref()
            .try_into()
            .map_err(|_err| ProgramError::InvalidInstructionData)?,
    );

    let payer = Pubkey::new_from_array(
        params
            .payer
            .as_ref()
            .try_into()
            .map_err(|_err| ProgramError::InvalidInstructionData)?,
    );

    let token_manager_ata =
        get_associated_token_address_with_program_id(&token_manager_pda, &mint, &token_program);

    let accounts = vec![
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(payer, true),
        AccountMeta::new_readonly(authority, signer),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(gateway::id(), false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(interchain_token_pda, false),
        AccountMeta::new(source_account, false),
        AccountMeta::new(mint, false),
        AccountMeta::new_readonly(token_manager_pda, false),
        AccountMeta::new(token_manager_ata, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new(flow_slot_pda, false),
    ];

    let data = InterchainTokenServiceInstruction::InterchainTransfer { params, bumps }
        .to_bytes()
        .map_err(|_err| ProgramError::InvalidInstructionData)?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
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
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway::get_gateway_root_config_pda().0);
    let (interchain_token_pda, _) = crate::find_interchain_token_pda(&its_root_pda, &token_id);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&interchain_token_pda);

    let (its_user_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::id(), &its_root_pda, &payer);
    let (token_manager_user_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::id(), &token_manager_pda, &its_root_pda);

    let instruction = InterchainTokenServiceInstruction::SetFlowLimit { flow_limit };

    let data = instruction
        .to_bytes()
        .map_err(|_err| ProgramError::InvalidInstructionData)?;

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
        &inputs.gateway_approved_message_pda,
        &inputs.gateway_root_pda,
        &inputs.gmp_metadata,
    );

    let abi_payload = inputs.payload.encode();

    let unwrapped_payload = match inputs.payload {
        GMPPayload::InterchainTransfer(_)
        | GMPPayload::DeployInterchainToken(_)
        | GMPPayload::DeployTokenManager(_) => inputs.payload,
        GMPPayload::SendToHub(inner) => GMPPayload::decode(&inner.payload)
            .map_err(|_err| ProgramError::InvalidInstructionData)?,
        GMPPayload::ReceiveFromHub(inner) => GMPPayload::decode(&inner.payload)
            .map_err(|_err| ProgramError::InvalidInstructionData)?,
    };

    let (mut its_accounts, bumps) = derive_its_accounts(
        &inputs.gateway_root_pda,
        &unwrapped_payload,
        inputs.token_program,
        inputs.mint,
        inputs.timestamp,
        inputs.bumps,
    )?;

    accounts.append(&mut its_accounts);

    let data = InterchainTokenServiceInstruction::ItsGmpPayload {
        abi_payload,
        gmp_metadata: inputs.gmp_metadata,
        bumps,
    }
    .to_bytes()
    .map_err(|_err| ProgramError::InvalidInstructionData)?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::MintTo`] instruction.
///
/// # Errors
/// If serialization fails.
pub fn mint_to(
    token_id: [u8; 32],
    mint: Pubkey,
    account: Pubkey,
    minter: Pubkey,
    token_program: Pubkey,
    amount: u64,
) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda(&gateway_root_pda);
    let (interchain_token_pda, _) = crate::find_interchain_token_pda(&its_root_pda, &token_id);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&interchain_token_pda);

    let instruction = InterchainTokenServiceInstruction::MintTo { amount };

    let data = instruction
        .to_bytes()
        .map_err(|_err| ProgramError::InvalidInstructionData)?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts: vec![
            AccountMeta::new(mint, false),
            AccountMeta::new(account, false),
            AccountMeta::new_readonly(interchain_token_pda, false),
            AccountMeta::new_readonly(token_manager_pda, false),
            AccountMeta::new_readonly(minter, true),
            AccountMeta::new_readonly(token_program, false),
        ],
        data,
    })
}

fn prefix_accounts(
    payer: &Pubkey,
    gateway_approved_message_pda: &Pubkey,
    gateway_root_pda: &Pubkey,
    gmp_metadata: &GmpMetadata,
) -> Vec<AccountMeta> {
    let command_id = gmp_metadata.cross_chain_id.command_id(hasher_impl());
    let destination_program = DestinationProgramId(crate::id());
    let (gateway_approved_message_signing_pda, _) = destination_program.signing_pda(&command_id);

    vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(*gateway_approved_message_pda, false),
        AccountMeta::new_readonly(gateway_approved_message_signing_pda, false),
        AccountMeta::new_readonly(*gateway_root_pda, false),
        AccountMeta::new_readonly(gateway::id(), false),
    ]
}

pub(crate) fn derive_its_accounts<'a, T>(
    gateway_root_pda: &Pubkey,
    payload: T,
    token_program: Pubkey,
    mint: Option<Pubkey>,
    maybe_timestamp: Option<i64>,
    maybe_bumps: Option<Bumps>,
) -> Result<(Vec<AccountMeta>, Bumps), ProgramError>
where
    T: TryInto<ItsMessageRef<'a>, Error = ProgramError>,
{
    let (
        maybe_its_root_pda_bump,
        maybe_interchain_token_pda_bump,
        maybe_token_manager_pda_bump,
        mut maybe_flow_slot_pda_bump,
    ) = maybe_bumps.map_or((None, None, None, None), |bumps| {
        (
            Some(bumps.its_root_pda_bump),
            Some(bumps.interchain_token_pda_bump),
            Some(bumps.token_manager_pda_bump),
            bumps.flow_slot_pda_bump,
        )
    });

    let message: ItsMessageRef<'_> = payload.try_into()?;

    let token_id = message.token_id();
    let (its_root_pda, its_root_pda_bump) =
        crate::its_root_pda(gateway_root_pda, maybe_its_root_pda_bump);
    let (interchain_token_pda, interchain_token_pda_bump) =
        crate::interchain_token_pda(&its_root_pda, token_id, maybe_interchain_token_pda_bump);
    let (token_manager_pda, token_manager_pda_bump) =
        crate::token_manager_pda(&interchain_token_pda, maybe_token_manager_pda_bump);
    let token_mint = try_retrieve_mint(&interchain_token_pda, &message, mint)?;

    if let ItsMessageRef::DeployInterchainToken { .. } = message {
        if token_program != spl_token_2022::id() {
            return Err(ProgramError::InvalidInstructionData);
        }
    }

    let mut accounts =
        derive_common_its_accounts(its_root_pda, token_mint, token_manager_pda, token_program);

    match message {
        ItsMessageRef::InterchainTransfer {
            destination_address,
            data,
            ..
        } => {
            let destination_wallet = Pubkey::new_from_array(
                destination_address
                    .try_into()
                    .map_err(|_err| ProgramError::InvalidInstructionData)?,
            );
            let destination_ata = get_associated_token_address_with_program_id(
                &destination_wallet,
                &token_mint,
                &token_program,
            );
            let Some(timestamp) = maybe_timestamp else {
                return Err(ProgramError::InvalidInstructionData);
            };
            let epoch = crate::state::flow_limit::flow_epoch_with_timestamp(timestamp)?;
            let (flow_slot_pda, flow_slot_pda_bump) =
                crate::flow_slot_pda(&token_manager_pda, epoch, maybe_flow_slot_pda_bump);

            maybe_flow_slot_pda_bump = Some(flow_slot_pda_bump);

            accounts.push(AccountMeta::new(destination_wallet, false));
            accounts.push(AccountMeta::new(destination_ata, false));
            accounts.push(AccountMeta::new(flow_slot_pda, false));

            if !data.is_empty() {
                let execute_data = DataPayload::decode(data)
                    .map_err(|_err| ProgramError::InvalidInstructionData)?;

                accounts.extend(execute_data.account_meta().iter().cloned());
            }
        }
        ItsMessageRef::DeployInterchainToken { minter, .. } => {
            accounts.push(AccountMeta::new_readonly(sysvar::rent::id(), false));
            if minter.len() == axelar_rkyv_encoding::types::ED25519_PUBKEY_LEN {
                accounts.push(AccountMeta::new_readonly(
                    Pubkey::new_from_array(
                        minter
                            .try_into()
                            .map_err(|_err| ProgramError::InvalidInstructionData)?,
                    ),
                    false,
                ));
            }
        }
        ItsMessageRef::DeployTokenManager { .. } => {}
    };

    Ok((
        accounts,
        Bumps {
            its_root_pda_bump,
            interchain_token_pda_bump,
            token_manager_pda_bump,
            flow_slot_pda_bump: maybe_flow_slot_pda_bump,
            ..Default::default()
        },
    ))
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
                .map(|(_, token_mint)| Pubkey::try_from(token_mint.as_ref()))?
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
    its_root_pda: Pubkey,
    mint_account: Pubkey,
    token_manager_pda: Pubkey,
    token_program: Pubkey,
) -> Vec<AccountMeta> {
    let token_manager_ata = get_associated_token_address_with_program_id(
        &token_manager_pda,
        &mint_account,
        &token_program,
    );

    vec![
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new(token_manager_pda, false),
        AccountMeta::new(mint_account, false),
        AccountMeta::new(token_manager_ata, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(spl_associated_token_account::id(), false),
    ]
}

pub(crate) trait OutboundInstruction {
    fn destination_chain(&mut self) -> Option<String>;
    fn gas_value(&self) -> U256;
}

impl OutboundInstruction for DeployInterchainTokenInputs {
    fn destination_chain(&mut self) -> Option<String> {
        self.destination_chain.take()
    }

    fn gas_value(&self) -> U256 {
        self.gas_value
    }
}

impl OutboundInstruction for DeployTokenManagerInputs {
    fn destination_chain(&mut self) -> Option<String> {
        self.destination_chain.take()
    }

    fn gas_value(&self) -> U256 {
        self.gas_value
    }
}

impl OutboundInstruction for InterchainTransferInputs {
    fn destination_chain(&mut self) -> Option<String> {
        self.destination_chain.take()
    }

    fn gas_value(&self) -> U256 {
        self.gas_value
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
            data: &value.metadata,
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
            data: value.metadata.into(),
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
