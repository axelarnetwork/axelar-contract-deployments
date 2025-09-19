#![allow(clippy::too_many_arguments)]
//! Instructions supported by the ITS program.

use std::borrow::Cow;

use axelar_message_primitives::{DataPayload, DestinationProgramId};
use axelar_solana_encoding::types::messages::Message;
use axelar_solana_gateway::state::incoming_message::command_id;
use borsh::{to_vec, BorshDeserialize, BorshSerialize};
use interchain_token_transfer_gmp::GMPPayload;
use solana_program::bpf_loader_upgradeable;
use solana_program::instruction::{AccountMeta, Instruction};
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::{system_program, sysvar};
use spl_associated_token_account::get_associated_token_address_with_program_id;
use typed_builder::TypedBuilder;

use crate::state;

pub mod interchain_token;
pub mod token_manager;

/// Instructions supported by the ITS program.
#[derive(Debug, PartialEq, Eq, BorshSerialize, BorshDeserialize)]
pub enum InterchainTokenServiceInstruction {
    /// Initializes the interchain token service program.
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. [writable,signer] The address of payer / sender
    /// 1. [] Program data account
    /// 2. [] Gateway root account
    /// 3. [writable] ITS root account
    /// 4. [] System program account
    /// 5. [] The account that will become the operator of the ITS
    /// 6. [writable] The address of the account that will store the roles of the operator account.
    Initialize {
        /// The name of the chain the ITS is running on.
        chain_name: String,

        /// The address of the ITS Hub
        its_hub_address: String,
    },

    /// Pauses or unpauses the interchain token service.
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. [writable,signer] The address of the payer, needs to be the ITS owner.
    /// 1. [] The program data account.
    /// 2. [] Gateway root account
    /// 3. [writable] ITS root pda.
    SetPauseStatus {
        /// The new pause status.
        paused: bool,
    },
    /// Sets a chain as trusted, allowing communication between this ITS and the ITS of that chain.
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. [writable,signer] The address of the payer, needs to be the ITS owner.
    /// 1. [] The program data account.
    /// 2. [] Gateway root account
    /// 3. [writable] ITS root pda.
    /// 4. [] The system program account.
    SetTrustedChain {
        /// The name of the chain to be trusted.
        chain_name: String,
    },

    /// Unsets a chain as trusted, disallowing communication between this ITS and the ITS of that chain.
    ///
    /// Accounts expected by this instruction:
    ///
    /// 0. [writable,signer] The address of the payer, needs to be the ITS owner.
    /// 1. [] The program data account.
    /// 2. [] Gateway root account
    /// 3. [writable] ITS root pda.
    /// 4. [] The system program account.
    RemoveTrustedChain {
        /// The name of the chain from which trust is removed.
        chain_name: String,
    },

    /// Approves the deployment of remote token with a destination minter
    ///
    /// 0. [writable,signer] The address of the payer, needs to have minter role on the token
    ///    manager.
    /// 1. [] The token manager account associated with the token
    /// 2. [] The account that holds the payer roles on the token manager
    /// 3. [writable] The account that will hold the approval of the deployment
    /// 4. [] The system program account
    ApproveDeployRemoteInterchainToken {
        /// The address of the account that deployed the `InterchainToken`
        deployer: Pubkey,
        /// The salt used to deploy the `InterchainToken`
        salt: [u8; 32],
        /// The remote chain where the `InterchainToken` will be deployed.
        destination_chain: String,
        /// The approved address of the minter on the destination chain
        destination_minter: Vec<u8>,
    },

    /// Revokes an approval of a deployment of remote token with a destination minter
    ///
    /// 0. [writable,signer] The address of the payer, needs to have minter role on the token
    ///    manager.
    /// 1. [] The token manager account associated with the token
    /// 2. [] The account that holds the payer roles on the token manager
    /// 3. [writable] The account holding the approval of the deployment that should be revoked
    /// 4. [] The system program account
    RevokeDeployRemoteInterchainToken {
        /// The address of the account that deployed the `InterchainToken`
        deployer: Pubkey,
        /// The salt used to deploy the `InterchainToken`
        salt: [u8; 32],
        /// The remote chain where the `InterchainToken` would be deployed.
        destination_chain: String,
    },

    /// Registers a canonical token as an interchain token and deploys its token manager.
    ///
    /// 0. [writable,signer] The address of the payer
    /// 1. [] The Metaplex metadata account associated with the mint
    /// 2. [] The GMP gateway root account
    /// 3. [] The system program account
    /// 4. [] The ITS root account
    /// 5. [writable] The token manager account derived from the `token_id` that will be initialized
    /// 6. [] The mint account (token address) of the original token
    /// 7. [] The token manager Associated Token Account
    /// 8. [] The token program account that was used to create the mint (`spl_token` vs `spl_token_2022`)
    /// 9. [] The Associated Token Account program account (`spl_associated_token_account`)
    /// 10. [] The rent sysvar account
    /// 11. [] The Metaplex metadata program account (`mpl_token_metadata`)
    RegisterCanonicalInterchainToken,

    /// Deploys a canonical interchain token on a remote chain.
    ///
    /// 0. [writable,signer] The account of the deployer, which is also paying for the transaction
    /// 1. [] The Metaplex metadata account associated with the mint
    /// 2. [] The GMP gateway root account
    /// 3. [] The system program account
    /// 4. [] The ITS root account
    /// 5. [writable] The token manager account associated with the interchain token
    /// 6. [writable] The mint account (token address) to deploy
    /// 7. [writable] The token manager Associated Token Account associated with the mint
    /// 8. [] The token program account that was used to create the mint (`spl_token` vs `spl_token_2022`)
    /// 9. [] The Associated Token Account program account (`spl_associated_token_account`)
    /// 10. [writable] The account holding the roles of the deployer on the ITS root account
    /// 11. [] The rent sysvar account
    /// 12. [] Optional account to set as operator on the `TokenManager`.
    /// 13. [writable] In case an operator is being set, this should be the account holding the roles of
    ///     the operator on the `TokenManager`
    DeployRemoteCanonicalInterchainToken {
        /// The remote chain where the `InterchainToken` should be deployed.
        destination_chain: String,
        /// The gas amount to be sent for deployment.
        gas_value: u64,
        /// The bump from the call contract signing account PDA derivation
        signing_pda_bump: u8,
    },

    /// Transfers interchain tokens.
    ///
    /// 0. [writable,signer] The address of the payer
    /// 1. [maybe signer] The address of the owner or delegate of the source account of the
    ///    transfer. In case it's the `TokenManager`, it shouldn't be set as signer as the signing
    ///    happens on chain.
    /// 2. [writable] The source account from which the tokens are being transferred
    /// 3. [] The mint account (token address)
    /// 4. [] The token manager account associated with the interchain token
    /// 5. [writable] The token manager Associated Token Account associated with the mint
    /// 6. [] The token program account that was used to create the mint (`spl_token` vs `spl_token_2022`)
    /// 7. [writable] The account tracking the flow of this mint for the current epoch
    /// 8. [] The GMP gateway root account
    /// 9. [] The GMP gateway program account
    /// 10. [writable] The GMP gas configuration account
    /// 11. [] The GMP gas service program account
    /// 12. [] The system program account
    /// 13. [] The ITS root account
    /// 14. [] The GMP call contract signing account
    /// 15. [] The ITS program account
    InterchainTransfer {
        /// The token id associated with the token
        token_id: [u8; 32],

        /// The chain where the tokens are being transferred to.
        destination_chain: String,

        /// The address on the destination chain to send the tokens to.
        destination_address: Vec<u8>,

        /// Amount of tokens being transferred.
        amount: u64,

        /// The gas value to be paid for the deploy transaction
        gas_value: u64,

        /// The bump from the call contract signing account PDA derivation
        signing_pda_bump: u8,
    },

    /// Transfers interchain tokens via Cross-Program Invocation (CPI) from a program PDA.
    /// This variant is designed for CPI-initiated transfers and includes
    /// the source program ID and PDA seeds for proper attribution.
    ///
    /// 0. [writable,signer] The address of the sender
    /// 1. [maybe signer] The address of the owner or delegate of the source account of the
    ///    transfer. In case it's the `TokenManager`, it shouldn't be set as signer as the signing
    ///    happens on chain.
    /// 2. [writable] The source account from which the tokens are being transferred
    /// 3. [] The mint account (token address)
    /// 4. [] The token manager account associated with the interchain token
    /// 5. [writable] The token manager Associated Token Account associated with the mint
    /// 6. [] The token program account that was used to create the mint (`spl_token` vs `spl_token_2022`)
    /// 7. [writable] The account tracking the flow of this mint for the current epoch
    /// 8. [] The GMP gateway root account
    /// 9. [] The GMP gateway program account
    /// 10. [writable] The GMP gas configuration account
    /// 11. [] The GMP gas service program account
    /// 12. [] The system program account
    /// 13. [] The ITS root account
    /// 14. [] The GMP call contract signing account
    /// 15. [] The ITS program account
    CpiInterchainTransfer {
        /// The token id associated with the token
        token_id: [u8; 32],

        /// The chain where the tokens are being transferred to.
        destination_chain: String,

        /// The address on the destination chain to send the tokens to.
        destination_address: Vec<u8>,

        /// Amount of tokens being transferred.
        amount: u64,

        /// The gas value to be paid for the deploy transaction
        gas_value: u64,

        /// The bump from the call contract signing account PDA derivation
        signing_pda_bump: u8,

        /// The program ID that owns the PDA initiating the transfer
        /// This will be used as the source address in events
        source_program_id: Pubkey,

        /// The seeds used to derive the PDA that's initiating the transfer
        /// This allows the processor to validate the PDA derivation
        pda_seeds: Vec<Vec<u8>>,
    },

    /// Deploys an interchain token.
    ///
    /// 0. [writable,signer] The account of the deployer, which is also paying for the transaction
    /// 1. [] The GMP gateway root account
    /// 2. [] The system program account
    /// 3. [] The ITS root account
    /// 4. [writable] The token manager account associated with the interchain token
    /// 5. [writable] The mint account (token address) to deploy
    /// 6. [writable] The token manager Associated Token Account associated with the mint
    /// 7. [] The token program account (`spl_token_2022`)
    /// 8. [] The Associated Token Account program account (`spl_associated_token_account`)
    /// 9. [writable] The account holding the roles of the deployer on the ITS root account
    /// 10. [] The rent sysvar account
    /// 11. [] The instructions sysvar account
    /// 12. [] The Metaplex metadata program account (`mpl_token_metadata`)
    /// 13. [writable] The Metaplex metadata account associated with the mint
    /// 14. [] The account to set as minter of the token
    /// 15. [writable] The account holding the roles of the minter account on the `TokenManager`
    DeployInterchainToken {
        /// The salt used to derive the tokenId associated with the token
        salt: [u8; 32],

        /// Token name
        name: String,

        /// Token symbol
        symbol: String,

        /// Token decimals
        decimals: u8,

        /// Initial supply
        initial_supply: u64,
    },

    /// Deploys a remote interchain token
    ///
    /// 0. [writable,signer] The address of the payer
    /// 1. [] The mint account (token address)
    /// 2. [] The Metaplex metadata account associated with the mint
    /// 3. [] The instructions sysvar account
    /// 4. [] The Metaplex metadata program account (`mpl_token_metadata`)
    /// 5. [] The GMP gateway root account
    /// 6. [] The GMP gateway program account
    /// 7. [writable] The GMP gas configuration account
    /// 8. [] The GMP gas service program account
    /// 9. [] The system program account
    /// 10. [] The ITS root account
    /// 11. [] The GMP call contract signing account
    /// 12. [] The ITS program account
    DeployRemoteInterchainToken {
        /// The salt used to derive the tokenId associated with the token
        salt: [u8; 32],

        /// The chain where the `InterchainToken` should be deployed.
        destination_chain: String,

        /// The gas value to be paid for the deploy transaction
        gas_value: u64,

        /// Signing PDA bump
        signing_pda_bump: u8,
    },

    /// Deploys a remote interchain token with associated minter
    ///
    /// 0. [writable,signer] The address of the payer
    /// 1. [] The mint account (token address)
    /// 2. [] The Metaplex metadata account associated with the mint
    /// 3. [] The account of the minter that approved the deployment
    /// 4. [writable] The account holding the approval for the deployment
    /// 5. [] The account holding the roles of the minter on the token manager associated with the
    ///    interchain token
    /// 6. [] The token manager account associated with the interchain token
    /// 7. [] The instructions sysvar account
    /// 8. [] The Metaplex metadata program account (`mpl_token_metadata`)
    /// 9. [] The GMP gateway root account
    /// 10. [] The GMP gateway program account
    /// 11. [writable] The GMP gas configuration account
    /// 12. [] The GMP gas service program account
    /// 13. [] The system program account
    /// 14. [] The ITS root account
    /// 15. [] The GMP call contract signing account
    /// 16. [] The ITS program account
    DeployRemoteInterchainTokenWithMinter {
        /// The salt used to derive the tokenId associated with the token
        salt: [u8; 32],

        /// The chain where the `InterchainToken` should be deployed.
        destination_chain: String,

        /// The minter on the destination chain
        destination_minter: Vec<u8>,

        /// The gas value to be paid for the deploy transaction
        gas_value: u64,

        /// Signing PDA bump
        signing_pda_bump: u8,
    },

    /// Registers token metadata.
    ///
    /// 0. [writable,signer] The address of the payer
    /// 1. [] The mint account (token address)
    /// 2. [] The token program account that was used to create the mint (`spl_token` vs `spl_token_2022`)
    /// 3. [] The GMP gateway root account
    /// 4. [] The GMP gateway program account
    /// 5. [writable] The GMP gas configuration account
    /// 6. [] The GMP gas service program account
    /// 7. [] The system program account
    /// 8. [] The ITS root account
    /// 9. [] The GMP call contract signing account
    /// 10. [] The ITS program account
    RegisterTokenMetadata {
        /// The gas value to be paid for the GMP transaction
        gas_value: u64,
        /// The signing PDA bump
        signing_pda_bump: u8,
    },

    /// Registers a custom token with ITS, deploying a new [`TokenManager`] to manage it.
    ///
    /// 0. [writable,signer] The account of the deployer, which is also paying for the transaction
    /// 1. [] The Metaplex metadata account associated with the mint
    /// 2. [] The GMP gateway root account
    /// 3. [] The system program account
    /// 4. [] The ITS root account
    /// 5. [writable] The token manager account associated with the interchain token
    /// 6. [writable] The mint account (token address) to deploy
    /// 7. [writable] The token manager Associated Token Account associated with the mint
    /// 8. [] The token program account that was used to create the mint (`spl_token` vs `spl_token_2022`)
    /// 9. [] The Associated Token Account program account (`spl_associated_token_account`)
    /// 10. [writable] The account holding the roles of the deployer on the ITS root account
    /// 11. [] The rent sysvar account
    /// 12. [] Optional account to set as operator on the `TokenManager`.
    /// 13. [writable] In case an operator is being set, this should be the account holding the roles of
    ///     the operator on the `TokenManager`
    RegisterCustomToken {
        /// Salt used to derive the `token_id` associated with the token.
        salt: [u8; 32],
        /// The token manager type.
        token_manager_type: state::token_manager::Type,
        /// The operator account
        operator: Option<Pubkey>,
    },

    /// Link a local token derived from salt and payer to another token on the `destination_chain`,
    /// at the `destination_token_address`.
    ///
    /// 0. [writable,signer] The address of the payer
    /// 1. [] The `TokenManager` account associated with the token being linked
    /// 2. [] The GMP gateway root account
    /// 3. [] The GMP gateway program account
    /// 4. [writable] The GMP gas configuration account
    /// 5. [] The GMP gas service program account
    /// 6. [] The system program account
    /// 7. [] The ITS root account
    /// 8. [] The GMP call contract signing account
    /// 9. [] The ITS program account
    LinkToken {
        /// Salt used to derive the `token_id` associated with the token.
        salt: [u8; 32],
        /// The chain where the token is being linked to.
        destination_chain: String,
        /// The address of the token on the destination chain.
        destination_token_address: Vec<u8>,
        /// The type of token manager used on the destination chain.
        token_manager_type: state::token_manager::Type,
        /// The params required on the destination chain.
        link_params: Vec<u8>,
        /// The gas value to be paid for the GMP transaction
        gas_value: u64,
        /// The signing PDA bump
        signing_pda_bump: u8,
    },

    /// Transfers tokens to a contract on the destination chain and call the give instruction on
    /// it. This instruction is is the same as [`InterchainTransfer`], but will fail if call data
    /// is empty.
    ///
    /// 0. [writable,signer] The address of the sender
    /// 1. [maybe signer] The address of the owner or delegate of the source account of the
    ///    transfer. In case it's the `TokenManager`, it shouldn't be set as signer as the signing
    ///    happens on chain.
    /// 2. [writable] The source account from which the tokens are being transferred
    /// 3. [] The mint account (token address)
    /// 4. [] The token manager account associated with the interchain token
    /// 5. [writable] The token manager Associated Token Account associated with the mint
    /// 6. [] The token program account that was used to create the mint (`spl_token` vs `spl_token_2022`)
    /// 7. [writable] The account tracking the flow of this mint for the current epoch
    /// 8. [] The GMP gateway root account
    /// 9. [] The GMP gateway program account
    /// 10. [writable] The GMP gas configuration account
    /// 11. [] The GMP gas service program account
    /// 12. [] The system program account
    /// 13. [] The ITS root account
    /// 14. [] The GMP call contract signing account
    /// 15. [] The ITS program account
    CallContractWithInterchainToken {
        /// The token id associated with the token
        token_id: [u8; 32],

        /// The chain where the tokens are being transferred to.
        destination_chain: String,

        /// The address on the destination chain to send the tokens to.
        destination_address: Vec<u8>,

        /// Amount of tokens being transferred.
        amount: u64,

        /// Call data
        data: Vec<u8>,

        /// The gas value to be paid for the deploy transaction
        gas_value: u64,

        /// Signing PDA bump
        signing_pda_bump: u8,
    },

    /// Transfers tokens via Cross-Program Invocation (CPI) to a contract on the destination chain
    /// and calls the given instruction on it. This instruction is designed for CPI-initiated
    /// transfers and includes the source program ID and PDA seeds for proper attribution.
    ///
    /// 0. [writable,signer] The address of the sender
    /// 1. [maybe signer] The address of the owner or delegate of the source account of the
    ///    transfer. In case it's the `TokenManager`, it shouldn't be set as signer as the signing
    ///    happens on chain.
    /// 2. [writable] The source account from which the tokens are being transferred
    /// 3. [] The mint account (token address)
    /// 4. [] The token manager account associated with the interchain token
    /// 5. [writable] The token manager Associated Token Account associated with the mint
    /// 6. [] The token program account that was used to create the mint (`spl_token` vs `spl_token_2022`)
    /// 7. [writable] The account tracking the flow of this mint for the current epoch
    /// 8. [] The GMP gateway root account
    /// 9. [] The GMP gateway program account
    /// 10. [writable] The GMP gas configuration account
    /// 11. [] The GMP gas service program account
    /// 12. [] The system program account
    /// 13. [] The ITS root account
    /// 14. [] The GMP call contract signing account
    /// 15. [] The ITS program account
    CpiCallContractWithInterchainToken {
        /// The token id associated with the token
        token_id: [u8; 32],

        /// The chain where the tokens are being transferred to.
        destination_chain: String,

        /// The address on the destination chain to send the tokens to.
        destination_address: Vec<u8>,

        /// Amount of tokens being transferred.
        amount: u64,

        /// Call data
        data: Vec<u8>,

        /// The gas value to be paid for the deploy transaction
        gas_value: u64,

        /// Signing PDA bump
        signing_pda_bump: u8,

        /// The program ID that owns the PDA initiating the transfer
        /// This will be used as the source address in events
        source_program_id: Pubkey,

        /// The seeds used to derive the PDA that's initiating the transfer
        /// This allows the processor to validate the PDA derivation
        pda_seeds: Vec<Vec<u8>>,
    },

    /// Sets the flow limit for an interchain token.
    ///
    /// 0. [writable,signer] The address of the payer
    /// 1. [] The ITS root account
    /// 2. [writable] The token manager account associated with the interchain token
    /// 3. [writable] The account holding the roles of the payer on the ITS root account
    /// 4. [writable] The account holding the roles of the payer on the `TokenManager`
    SetFlowLimit {
        /// The new flow limit.
        flow_limit: Option<u64>,
    },

    /// Transfers operatorship to another account.
    ///
    /// 0. [] System program account.
    /// 1. [writable, signer] Payer account.
    /// 2. [] PDA for the payer roles on the resource which the operatorship is being transferred
    ///    from.
    /// 3. [] PDA for the resource.
    /// 4. [] Account to transfer operatorship to.
    /// 5. [writable] PDA with the roles on the resource the
    ///    operatorship is being transferred to.
    TransferOperatorship,

    /// Proposes operatorship transfer to another account.
    ///
    /// 0. [] System program account.
    /// 1. [writable, signer] Payer account.
    /// 2. [] PDA for the payer roles on the resource.
    /// 3. [] PDA for the resource.
    /// 4. [] Account to transfer operatorship to.
    /// 5. [writable] PDA with the roles on the resource for the accounts the
    ///    operatorship is being transferred to.
    /// 6. [] Account which the operatorship is being transferred from.
    /// 7. [writable] PDA with the roles on the resource for the account the
    ///    operatorship is being transferred from.
    /// 8. [writable] PDA for the proposal
    ProposeOperatorship,

    /// Accepts operatorship transfer from another account.
    ///
    /// 0. [] System program account.
    /// 1. [writable, signer] Payer account.
    /// 2. [] PDA for the payer roles on the resource.
    /// 3. [] PDA for the resource.
    /// 4. [] Account to transfer operatorship to.
    /// 5. [writable] PDA with the roles on the resource for the accounts the
    ///    operatorship is being transferred to.
    /// 6. [] Account which the operatorship is being transferred from.
    /// 7. [writable] PDA with the roles on the resource for the account the
    ///    operatorship is being transferred from.
    /// 8. [writable] PDA for the proposal
    AcceptOperatorship,

    /// Adds a flow limiter to a [`TokenManager`].
    ///
    /// 0. [] System program account.
    /// 1. [writable, signer] Payer account (must have operator role).
    /// 2. [] PDA for the payer roles on the token manager.
    /// 3. [] PDA for the token manager.
    /// 4. [] Account to add as flow limiter.
    /// 5. [writable] PDA with the roles on the token manager for the flow limiter being added.
    AddTokenManagerFlowLimiter,

    /// Removes a flow limiter from a [`TokenManager`].
    ///
    /// 0. [] System program account.
    /// 1. [writable, signer] Payer account (must have operator role).
    /// 2. [] PDA for the payer roles on the token manager.
    /// 3. [] PDA for the token manager.
    /// 4. [] Account to remove as flow limiter.
    /// 5. [writable] PDA with the roles on the token manager for the flow limiter being removed.
    RemoveTokenManagerFlowLimiter,

    /// Sets the flow limit for an interchain token.
    ///
    /// 0. [signer] Payer account.
    /// 1. [] ITS root PDA account.
    /// 2. [writable] The [`TokenManager`] PDA account.
    /// 3. [] The PDA account with the user roles on the [`TokenManager`].
    /// 4. [] The PDA account with the user roles on ITS.
    SetTokenManagerFlowLimit {
        /// The new flow limit.
        flow_limit: Option<u64>,
    },

    /// Transfers operatorship to another account.
    ///
    /// 0. [] ITS root PDA.
    /// 1. [] System program account.
    /// 2. [writable, signer] Payer account.
    /// 3. [] PDA for the payer roles on the resource which the operatorship is being transferred
    ///    from.
    /// 4. [] PDA for the resource.
    /// 5. [] Account to transfer operatorship to.
    /// 6. [writable] PDA with the roles on the resource the
    ///    operatorship is being transferred to.
    TransferTokenManagerOperatorship,

    /// Proposes operatorship transfer to another account.
    ///
    /// 0. [] System program account.
    /// 1. [writable, signer] Payer account.
    /// 2. [] PDA for the payer roles on the resource.
    /// 3. [] PDA for the resource.
    /// 4. [] Account to transfer operatorship to.
    /// 5. [writable] PDA with the roles on the resource for the accounts the
    ///    operatorship is being transferred to.
    /// 6. [] Account which the operatorship is being transferred from.
    /// 7. [writable] PDA with the roles on the resource for the account the
    ///    operatorship is being transferred from.
    /// 8. [writable] PDA for the proposal
    ProposeTokenManagerOperatorship,

    /// Accepts operatorship transfer from another account.
    ///
    /// 0. [] System program account.
    /// 1. [writable, signer] Payer account.
    /// 2. [] PDA for the payer roles on the resource.
    /// 3. [] PDA for the resource.
    /// 4. [] Account to transfer operatorship to.
    /// 5. [writable] PDA with the roles on the resource for the accounts the
    ///    operatorship is being transferred to.
    /// 6. [] Account which the operatorship is being transferred from.
    /// 7. [writable] PDA with the roles on the resource for the account the
    ///    operatorship is being transferred from.
    /// 8. [writable] PDA for the proposal
    AcceptTokenManagerOperatorship,

    /// Transfers the mint authority to the token manager allowing it to mint tokens and manage
    /// minters. The account transferring the authority gains minter role on the [`TokenManager`] and
    /// thus can then mint tokens through the ITS mitn instruction.
    ///
    /// 0. [writable, signer] Payer, current mint authority
    /// 1. [writable] The mint for which the authority is being handed over
    /// 2. [] ITS root account
    /// 3. [] The [`TokenManager`] account associated with the mint
    /// 4. [] The account that will hold the roles of the former authority on the [`TokenManager`]
    /// 5. [] The token program used to create the mint
    /// 6. [] The system program account
    HandoverMintAuthority {
        /// The id of the token registered with ITS for which the authority is being handed over.
        token_id: [u8; 32],
    },

    /// A proxy instruction to mint tokens whose mint authority is a
    /// `TokenManager`. Only users with the `minter` role on the mint account
    /// can mint tokens.
    ///
    /// 0. [writable] The mint account
    /// 1. [writable] The account to mint tokens to
    /// 2. [] The interchain token PDA associated with the mint
    /// 3. [] The token manager PDA
    /// 4. [signer] The minter account
    /// 5. [] The token program id
    MintInterchainToken {
        /// The amount of tokens to mint.
        amount: u64,
    },

    /// Transfers mintership to another account.
    ///
    /// 0. [] ITS root PDA.
    /// 1. [] System program account.
    /// 2. [writable, signer] Payer account.
    /// 3. [] PDA for the payer roles on the resource which the mintership is being transferred
    ///    from.
    /// 4. [] PDA for the resource.
    /// 5. [] Account to transfer mintership to.
    /// 6. [writable] PDA with the roles on the resource the
    ///    mintership is being transferred to.
    TransferInterchainTokenMintership,

    /// Proposes mintership transfer to another account.
    ///
    /// 0. [] System program account.
    /// 1. [writable, signer] Payer account.
    /// 2. [] PDA for the payer roles on the resource.
    /// 3. [] PDA for the resource.
    /// 4. [] Account to transfer operatorship to.
    /// 5. [writable] PDA with the roles on the resource for the accounts the
    ///    operatorship is being transferred to.
    /// 6. [] Account which the operatorship is being transferred from.
    /// 7. [writable] PDA with the roles on the resource for the account the
    ///    operatorship is being transferred from.
    /// 8. [writable] PDA for the proposal
    ProposeInterchainTokenMintership,

    /// Accepts mintership transfer from another account.
    ///
    /// 0. [] System program account.
    /// 1. [writable, signer] Payer account.
    /// 2. [] PDA for the payer roles on the resource.
    /// 3. [] PDA for the resource.
    /// 4. [] Account to transfer operatorship to.
    /// 5. [writable] PDA with the roles on the resource for the accounts the
    ///    operatorship is being transferred to.
    /// 6. [] Account which the operatorship is being transferred from.
    /// 7. [writable] PDA with the roles on the resource for the account the
    ///    operatorship is being transferred from.
    /// 8. [writable] PDA for the proposal
    AcceptInterchainTokenMintership,

    /// A GMP Interchain Token Service instruction.
    ///
    /// 0. [writable,signer] The address of payer / sender
    /// 1. [] gateway root pda
    /// 2. [] ITS root pda
    ///
    /// 3..N Accounts depend on the inner ITS instruction.
    Execute {
        /// The GMP metadata
        message: Message,
    },
}

/// Inputs for the [`execute`] function.
///
/// To construct this type, use its builder API.
///
/// # Example
///
/// ```ignore
/// use axelar_solana_its::instructions::ExecuteInstructionInputs;
///
/// let inputs = ExecuteInstructionInputs::builder()
///   .payer(payer_pubkey)
///   .incoming_message_pda(gateway_approved_message_pda)
///   .message(message)
///   .payload(payload)
///   .token_program(spl_token_2022::ID)
///   .mint(mint_pubkey)
///   .bumps(bumps)
///   .build();
/// ```
#[derive(Debug, Clone, TypedBuilder)]
pub struct ExecuteInstructionInputs {
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
}

/// Creates an [`InterchainTokenServiceInstruction::Initialize`] instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn initialize(
    payer: Pubkey,
    operator: Pubkey,
    chain_name: String,
    its_hub_address: String,
) -> Result<Instruction, ProgramError> {
    let (its_root_pda, _) = crate::find_its_root_pda();
    let (program_data_address, _) =
        Pubkey::find_program_address(&[crate::ID.as_ref()], &bpf_loader_upgradeable::ID);
    let (user_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::ID, &its_root_pda, &operator);

    let data = to_vec(&InterchainTokenServiceInstruction::Initialize {
        chain_name,
        its_hub_address,
    })?;

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(program_data_address, false),
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
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn set_pause_status(payer: Pubkey, paused: bool) -> Result<Instruction, ProgramError> {
    let (program_data_address, _) =
        Pubkey::find_program_address(&[crate::ID.as_ref()], &bpf_loader_upgradeable::ID);
    let (its_root_pda, _) = crate::find_its_root_pda();

    let data = to_vec(&InterchainTokenServiceInstruction::SetPauseStatus { paused })?;

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(program_data_address, false),
        AccountMeta::new(its_root_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::SetTrustedChain`] instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn set_trusted_chain(payer: Pubkey, chain_name: String) -> Result<Instruction, ProgramError> {
    let (program_data_address, _) =
        Pubkey::find_program_address(&[crate::ID.as_ref()], &bpf_loader_upgradeable::ID);

    let (its_root_pda, _) = crate::find_its_root_pda();
    let (payer_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::ID, &its_root_pda, &payer);

    let data = to_vec(&InterchainTokenServiceInstruction::SetTrustedChain { chain_name })?;

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(payer_roles_pda, false),
        AccountMeta::new_readonly(program_data_address, false),
        AccountMeta::new(its_root_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::RemoveTrustedChain`] instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn remove_trusted_chain(
    payer: Pubkey,
    chain_name: String,
) -> Result<Instruction, ProgramError> {
    let (program_data_address, _) =
        Pubkey::find_program_address(&[crate::ID.as_ref()], &bpf_loader_upgradeable::ID);
    let (its_root_pda, _) = crate::find_its_root_pda();
    let (payer_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::ID, &its_root_pda, &payer);

    let data = to_vec(&InterchainTokenServiceInstruction::RemoveTrustedChain { chain_name })?;

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(payer_roles_pda, false),
        AccountMeta::new_readonly(program_data_address, false),
        AccountMeta::new(its_root_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::ApproveDeployRemoteInterchainToken`] instruction.
///
/// Allow the minter to approve the deployer for a remote interchain token deployment that uses a
/// custom `destination_minter` address. This ensures that a token deployer can't choose the
/// `destination_minter` itself, and requires the approval of the minter to reduce trust assumptions
/// on the deployer.
///
/// # Parameters
///
/// `payer`: The account paying for the transaction, also with minter role on the `TokenManager`.
/// `deployer`: The address of the account that deployed the `InterchainToken`.
/// `salt`: The unique salt for deploying the token.
/// `destination_chain`: The name of the destination chain.
/// `destination_minter`: The minter address to set on the deployed token on the destination chain. This can be arbitrary bytes since the encoding of the account is dependent on the destination chain.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn approve_deploy_remote_interchain_token(
    payer: Pubkey,
    deployer: Pubkey,
    salt: [u8; 32],
    destination_chain: String,
    destination_minter: Vec<u8>,
) -> Result<Instruction, ProgramError> {
    let (its_root_pda, _) = crate::find_its_root_pda();
    let token_id = crate::interchain_token_id(&deployer, &salt);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let (roles_pda, _) =
        role_management::find_user_roles_pda(&crate::ID, &token_manager_pda, &payer);
    let (deploy_approval_pda, _) =
        crate::find_deployment_approval_pda(&payer, &token_id, &destination_chain);

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(token_manager_pda, false),
        AccountMeta::new_readonly(roles_pda, false),
        AccountMeta::new(deploy_approval_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    let data = to_vec(
        &InterchainTokenServiceInstruction::ApproveDeployRemoteInterchainToken {
            deployer,
            salt,
            destination_chain,
            destination_minter,
        },
    )?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::RevokeDeployRemoteInterchainToken`] instruction.
///
/// Allows the minter to revoke a deployer's approval for a remote interchain token deployment that
/// uses a custom `destination_minter` address.
///
/// # Parameters
///
/// `payer`: The account paying for the transaction, also with minter role on the `TokenManager`.
/// `deployer`: The address of the account that deployed the `InterchainToken`.
/// `salt`: The unique salt for deploying the token.
/// `destination_chain`: The name of the destination chain.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn revoke_deploy_remote_interchain_token(
    payer: Pubkey,
    deployer: Pubkey,
    salt: [u8; 32],
    destination_chain: String,
) -> Result<Instruction, ProgramError> {
    let token_id = crate::interchain_token_id(&deployer, &salt);
    let (deploy_approval_pda, _) =
        crate::find_deployment_approval_pda(&payer, &token_id, &destination_chain);

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new(deploy_approval_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    let data = to_vec(
        &InterchainTokenServiceInstruction::RevokeDeployRemoteInterchainToken {
            deployer,
            salt,
            destination_chain,
        },
    )?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::RegisterCanonicalInterchainToken`]
/// instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn register_canonical_interchain_token(
    payer: Pubkey,
    mint: Pubkey,
    token_program: Pubkey,
) -> Result<Instruction, ProgramError> {
    let (its_root_pda, _) = crate::find_its_root_pda();
    let token_id = crate::canonical_interchain_token_id(&mint);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let token_manager_ata =
        get_associated_token_address_with_program_id(&token_manager_pda, &mint, &token_program);
    let (token_metadata_account, _) = mpl_token_metadata::accounts::Metadata::find_pda(&mint);
    let (its_user_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::ID, &token_manager_pda, &its_root_pda);

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(token_metadata_account, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new(token_manager_pda, false),
        AccountMeta::new(mint, false),
        AccountMeta::new(token_manager_ata, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        AccountMeta::new(its_user_roles_pda, false),
        AccountMeta::new_readonly(sysvar::rent::ID, false),
    ];

    let data = to_vec(&InterchainTokenServiceInstruction::RegisterCanonicalInterchainToken)?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::DeployRemoteInterchainToken`]
/// instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn deploy_remote_canonical_interchain_token(
    payer: Pubkey,
    mint: Pubkey,
    destination_chain: String,
    gas_value: u64,
) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda();
    let (call_contract_signing_pda, signing_pda_bump) =
        axelar_solana_gateway::get_call_contract_signing_pda(crate::ID);
    let (metadata_account_key, _) = mpl_token_metadata::accounts::Metadata::find_pda(&mint);
    let (gas_config_pda, _bump) = axelar_solana_gas_service::get_config_pda();
    let token_id = crate::canonical_interchain_token_id(&mint);
    let (token_manager, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new_readonly(metadata_account_key, false),
        AccountMeta::new_readonly(token_manager, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(axelar_solana_gateway::ID, false),
        AccountMeta::new(gas_config_pda, false),
        AccountMeta::new_readonly(axelar_solana_gas_service::ID, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(call_contract_signing_pda, false),
        AccountMeta::new_readonly(crate::ID, false),
    ];

    let data = to_vec(
        &InterchainTokenServiceInstruction::DeployRemoteCanonicalInterchainToken {
            destination_chain,
            gas_value,
            signing_pda_bump,
        },
    )?;

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
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn deploy_interchain_token(
    payer: Pubkey,
    salt: [u8; 32],
    name: String,
    symbol: String,
    decimals: u8,
    initial_supply: u64,
    minter: Option<Pubkey>,
) -> Result<Instruction, ProgramError> {
    let (its_root_pda, _) = crate::find_its_root_pda();
    let token_id = crate::interchain_token_id(&payer, &salt);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let (mint, _) = crate::find_interchain_token_pda(&its_root_pda, &token_id);
    let token_manager_ata = get_associated_token_address_with_program_id(
        &token_manager_pda,
        &mint,
        &spl_token_2022::ID,
    );
    let payer_ata =
        get_associated_token_address_with_program_id(&payer, &mint, &spl_token_2022::ID);
    let (its_user_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::ID, &token_manager_pda, &its_root_pda);
    let (metadata_account_key, _) = mpl_token_metadata::accounts::Metadata::find_pda(&mint);

    let mut accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new(token_manager_pda, false),
        AccountMeta::new(mint, false),
        AccountMeta::new(token_manager_ata, false),
        AccountMeta::new_readonly(spl_token_2022::ID, false),
        AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        AccountMeta::new(its_user_roles_pda, false),
        AccountMeta::new_readonly(sysvar::rent::ID, false),
        AccountMeta::new_readonly(sysvar::instructions::ID, false),
        AccountMeta::new_readonly(mpl_token_metadata::ID, false),
        AccountMeta::new(metadata_account_key, false),
        AccountMeta::new(payer_ata, false),
    ];

    if let Some(minter) = minter {
        let (minter_roles_pda, _) =
            role_management::find_user_roles_pda(&crate::ID, &token_manager_pda, &minter);
        accounts.push(AccountMeta::new_readonly(minter, false));
        accounts.push(AccountMeta::new(minter_roles_pda, false));
    }

    let data = to_vec(&InterchainTokenServiceInstruction::DeployInterchainToken {
        salt,
        name,
        symbol,
        decimals,
        initial_supply,
    })?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::DeployRemoteInterchainToken`]
/// instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn deploy_remote_interchain_token(
    payer: Pubkey,
    salt: [u8; 32],
    destination_chain: String,
    gas_value: u64,
) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda();
    let token_id = crate::interchain_token_id(&payer, &salt);
    let (mint, _) = crate::find_interchain_token_pda(&its_root_pda, &token_id);
    let (call_contract_signing_pda, signing_pda_bump) =
        axelar_solana_gateway::get_call_contract_signing_pda(crate::ID);
    let (metadata_account_key, _) = mpl_token_metadata::accounts::Metadata::find_pda(&mint);
    let (gas_config_pda, _bump) = axelar_solana_gas_service::get_config_pda();
    let (token_manager, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new_readonly(metadata_account_key, false),
        AccountMeta::new_readonly(token_manager, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(axelar_solana_gateway::ID, false),
        AccountMeta::new(gas_config_pda, false),
        AccountMeta::new_readonly(axelar_solana_gas_service::ID, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(call_contract_signing_pda, false),
        AccountMeta::new_readonly(crate::ID, false),
    ];

    let data = to_vec(
        &InterchainTokenServiceInstruction::DeployRemoteInterchainToken {
            salt,
            destination_chain,
            gas_value,
            signing_pda_bump,
        },
    )?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::DeployRemoteInterchainTokenWithMinter`]
/// instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn deploy_remote_interchain_token_with_minter(
    payer: Pubkey,
    salt: [u8; 32],
    minter: Pubkey,
    destination_chain: String,
    destination_minter: Vec<u8>,
    gas_value: u64,
) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda();
    let token_id = crate::interchain_token_id(&payer, &salt);
    let (mint, _) = crate::find_interchain_token_pda(&its_root_pda, &token_id);
    let (call_contract_signing_pda, signing_pda_bump) =
        axelar_solana_gateway::get_call_contract_signing_pda(crate::ID);
    let (metadata_account_key, _) = mpl_token_metadata::accounts::Metadata::find_pda(&mint);
    let (deploy_approval, _) =
        crate::find_deployment_approval_pda(&minter, &token_id, &destination_chain);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let (minter_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::ID, &token_manager_pda, &minter);
    let (gas_config_pda, _bump) = axelar_solana_gas_service::get_config_pda();

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new_readonly(metadata_account_key, false),
        AccountMeta::new_readonly(token_manager_pda, false),
        AccountMeta::new_readonly(minter, false),
        AccountMeta::new(deploy_approval, false),
        AccountMeta::new_readonly(minter_roles_pda, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(axelar_solana_gateway::ID, false),
        AccountMeta::new(gas_config_pda, false),
        AccountMeta::new_readonly(axelar_solana_gas_service::ID, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(call_contract_signing_pda, false),
        AccountMeta::new_readonly(crate::ID, false),
    ];

    let data = to_vec(
        &InterchainTokenServiceInstruction::DeployRemoteInterchainTokenWithMinter {
            salt,
            destination_chain,
            gas_value,
            signing_pda_bump,
            destination_minter,
        },
    )?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates [`InterchainTokenServiceInstruction::RegisterTokenMetadata`] instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn register_token_metadata(
    payer: Pubkey,
    mint: Pubkey,
    gas_value: u64,
) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda();
    let (call_contract_signing_pda, signing_pda_bump) =
        axelar_solana_gateway::get_call_contract_signing_pda(crate::ID);
    let (gas_config_pda, _bump) = axelar_solana_gas_service::get_config_pda();

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(mint, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(axelar_solana_gateway::ID, false),
        AccountMeta::new(gas_config_pda, false),
        AccountMeta::new_readonly(axelar_solana_gas_service::ID, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(call_contract_signing_pda, false),
        AccountMeta::new_readonly(crate::ID, false),
    ];

    let data = to_vec(&InterchainTokenServiceInstruction::RegisterTokenMetadata {
        gas_value,
        signing_pda_bump,
    })?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::RegisterCustomToken`]
/// instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn register_custom_token(
    payer: Pubkey,
    salt: [u8; 32],
    mint: Pubkey,
    token_manager_type: state::token_manager::Type,
    token_program: Pubkey,
    operator: Option<Pubkey>,
) -> Result<Instruction, ProgramError> {
    let (its_root_pda, _) = crate::find_its_root_pda();
    let token_id = crate::linked_token_id(&payer, &salt);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let token_manager_ata =
        get_associated_token_address_with_program_id(&token_manager_pda, &mint, &token_program);
    let (token_metadata_account, _) = mpl_token_metadata::accounts::Metadata::find_pda(&mint);
    let (its_user_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::ID, &token_manager_pda, &its_root_pda);

    let mut accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(token_metadata_account, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new(token_manager_pda, false),
        AccountMeta::new(mint, false),
        AccountMeta::new(token_manager_ata, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(spl_associated_token_account::ID, false),
        AccountMeta::new(its_user_roles_pda, false),
        AccountMeta::new_readonly(sysvar::rent::ID, false),
    ];

    if let Some(operator) = operator {
        let (operator_roles_pda, _) =
            role_management::find_user_roles_pda(&crate::ID, &token_manager_pda, &operator);
        accounts.push(AccountMeta::new(operator, false));
        accounts.push(AccountMeta::new(operator_roles_pda, false));
    }

    let data = to_vec(&InterchainTokenServiceInstruction::RegisterCustomToken {
        salt,
        token_manager_type,
        operator,
    })?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::LinkToken`] instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn link_token(
    payer: Pubkey,
    salt: [u8; 32],
    destination_chain: String,
    destination_token_address: Vec<u8>,
    token_manager_type: state::token_manager::Type,
    link_params: Vec<u8>,
    gas_value: u64,
) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda();
    let (call_contract_signing_pda, signing_pda_bump) =
        axelar_solana_gateway::get_call_contract_signing_pda(crate::ID);
    let token_id = crate::linked_token_id(&payer, &salt);
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let (gas_config_pda, _bump) = axelar_solana_gas_service::get_config_pda();

    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(token_manager_pda, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(axelar_solana_gateway::ID, false),
        AccountMeta::new(gas_config_pda, false),
        AccountMeta::new_readonly(axelar_solana_gas_service::ID, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(call_contract_signing_pda, false),
        AccountMeta::new_readonly(crate::ID, false),
    ];

    let data = to_vec(&InterchainTokenServiceInstruction::LinkToken {
        salt,
        destination_chain,
        destination_token_address,
        token_manager_type,
        link_params,
        gas_value,
        signing_pda_bump,
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
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn interchain_transfer(
    payer: Pubkey,
    source_account: Pubkey,
    token_id: [u8; 32],
    destination_chain: String,
    destination_address: Vec<u8>,
    amount: u64,
    mint: Pubkey,
    token_program: Pubkey,
    gas_value: u64,
) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda();
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let token_manager_ata =
        get_associated_token_address_with_program_id(&token_manager_pda, &mint, &token_program);
    let (call_contract_signing_pda, signing_pda_bump) =
        axelar_solana_gateway::get_call_contract_signing_pda(crate::ID);
    let (gas_config_pda, _bump) = axelar_solana_gas_service::get_config_pda();

    let accounts = vec![
        AccountMeta::new_readonly(payer, true),
        AccountMeta::new(source_account, false),
        AccountMeta::new(mint, false),
        AccountMeta::new(token_manager_pda, false),
        AccountMeta::new(token_manager_ata, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(axelar_solana_gateway::ID, false),
        AccountMeta::new(gas_config_pda, false),
        AccountMeta::new_readonly(axelar_solana_gas_service::ID, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(call_contract_signing_pda, false),
        AccountMeta::new_readonly(crate::ID, false),
    ];

    let data = to_vec(&InterchainTokenServiceInstruction::InterchainTransfer {
        token_id,
        destination_chain,
        destination_address,
        amount,
        gas_value,
        signing_pda_bump,
    })?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::CpiInterchainTransfer`] instruction.
///
/// This variant is for CPI-initiated transfers and includes source program attribution.
///
/// # Errors
///
/// This function will return an error if the instruction data cannot be serialized.
pub fn cpi_interchain_transfer(
    sender: Pubkey,
    source_account: Pubkey,
    token_id: [u8; 32],
    destination_chain: String,
    destination_address: Vec<u8>,
    amount: u64,
    mint: Pubkey,
    token_program: Pubkey,
    gas_value: u64,
    source_program_id: Pubkey,
    pda_seeds: Vec<Vec<u8>>,
) -> Result<Instruction, ProgramError> {
    let (gateway_root_pda, _) = axelar_solana_gateway::get_gateway_root_config_pda();
    let (its_root_pda, _) = crate::find_its_root_pda();
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let token_manager_ata =
        get_associated_token_address_with_program_id(&token_manager_pda, &mint, &token_program);
    let (call_contract_signing_pda, signing_pda_bump) =
        axelar_solana_gateway::get_call_contract_signing_pda(crate::ID);
    let (gas_config_pda, _bump) = axelar_solana_gas_service::get_config_pda();

    let accounts = vec![
        AccountMeta::new_readonly(sender, true),
        AccountMeta::new(source_account, false),
        AccountMeta::new(mint, false),
        AccountMeta::new(token_manager_pda, false),
        AccountMeta::new(token_manager_ata, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(gateway_root_pda, false),
        AccountMeta::new_readonly(axelar_solana_gateway::ID, false),
        AccountMeta::new(gas_config_pda, false),
        AccountMeta::new_readonly(axelar_solana_gas_service::ID, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(call_contract_signing_pda, false),
        AccountMeta::new_readonly(crate::ID, false),
    ];

    let data = to_vec(&InterchainTokenServiceInstruction::CpiInterchainTransfer {
        token_id,
        destination_chain,
        destination_address,
        amount,
        gas_value,
        signing_pda_bump,
        source_program_id,
        pda_seeds,
    })?;

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
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn call_contract_with_interchain_token(
    payer: Pubkey,
    source_account: Pubkey,
    token_id: [u8; 32],
    destination_chain: String,
    destination_address: Vec<u8>,
    amount: u64,
    mint: Pubkey,
    data: Vec<u8>,
    token_program: Pubkey,
    gas_value: u64,
) -> Result<Instruction, ProgramError> {
    let (its_root_pda, _) = crate::find_its_root_pda();
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let token_manager_ata =
        get_associated_token_address_with_program_id(&token_manager_pda, &mint, &token_program);
    let (call_contract_signing_pda, signing_pda_bump) =
        axelar_solana_gateway::get_call_contract_signing_pda(crate::ID);
    let (gas_config_pda, _bump) = axelar_solana_gas_service::get_config_pda();

    let accounts = vec![
        AccountMeta::new_readonly(payer, true),
        AccountMeta::new(source_account, false),
        AccountMeta::new(mint, false),
        AccountMeta::new_readonly(token_manager_pda, false),
        AccountMeta::new(token_manager_ata, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(axelar_solana_gateway::ID, false),
        AccountMeta::new(gas_config_pda, false),
        AccountMeta::new_readonly(axelar_solana_gas_service::ID, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(call_contract_signing_pda, false),
        AccountMeta::new_readonly(crate::ID, false),
    ];

    let data = to_vec(
        &InterchainTokenServiceInstruction::CallContractWithInterchainToken {
            token_id,
            destination_chain,
            destination_address,
            amount,
            gas_value,
            signing_pda_bump,
            data,
        },
    )?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::CpiCallContractWithInterchainToken`] instruction.
///
/// This variant is for CPI-initiated transfers with contract calls and includes source program attribution.
///
/// # Errors
///
/// This function will return an error if the instruction data cannot be serialized.
pub fn cpi_call_contract_with_interchain_token(
    sender: Pubkey,
    source_account: Pubkey,
    token_id: [u8; 32],
    destination_chain: String,
    destination_address: Vec<u8>,
    amount: u64,
    mint: Pubkey,
    data: Vec<u8>,
    token_program: Pubkey,
    gas_value: u64,
    source_program_id: Pubkey,
    pda_seeds: Vec<Vec<u8>>,
) -> Result<Instruction, ProgramError> {
    let (its_root_pda, _) = crate::find_its_root_pda();
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);
    let token_manager_ata =
        get_associated_token_address_with_program_id(&token_manager_pda, &mint, &token_program);
    let (call_contract_signing_pda, signing_pda_bump) =
        axelar_solana_gateway::get_call_contract_signing_pda(crate::ID);
    let (gas_config_pda, _bump) = axelar_solana_gas_service::get_config_pda();

    let accounts = vec![
        AccountMeta::new_readonly(sender, true),
        AccountMeta::new(source_account, false),
        AccountMeta::new(mint, false),
        AccountMeta::new_readonly(token_manager_pda, false),
        AccountMeta::new(token_manager_ata, false),
        AccountMeta::new_readonly(token_program, false),
        AccountMeta::new_readonly(axelar_solana_gateway::ID, false),
        AccountMeta::new(gas_config_pda, false),
        AccountMeta::new_readonly(axelar_solana_gas_service::ID, false),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(call_contract_signing_pda, false),
        AccountMeta::new_readonly(crate::ID, false),
    ];

    let data = to_vec(
        &InterchainTokenServiceInstruction::CpiCallContractWithInterchainToken {
            token_id,
            destination_chain,
            destination_address,
            amount,
            data,
            gas_value,
            signing_pda_bump,
            source_program_id,
            pda_seeds,
        },
    )?;

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
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn set_flow_limit(
    payer: Pubkey,
    token_id: [u8; 32],
    flow_limit: Option<u64>,
) -> Result<Instruction, ProgramError> {
    let (its_root_pda, _) = crate::find_its_root_pda();
    let (token_manager_pda, _) = crate::find_token_manager_pda(&its_root_pda, &token_id);

    let (its_user_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::ID, &its_root_pda, &payer);
    let (token_manager_user_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::ID, &token_manager_pda, &its_root_pda);

    let data = to_vec(&InterchainTokenServiceInstruction::SetFlowLimit { flow_limit })?;
    let accounts = vec![
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new(token_manager_pda, false),
        AccountMeta::new_readonly(its_user_roles_pda, false),
        AccountMeta::new_readonly(token_manager_user_roles_pda, false),
        AccountMeta::new_readonly(system_program::ID, false),
    ];

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::Execute`] instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn execute(inputs: ExecuteInstructionInputs) -> Result<Instruction, ProgramError> {
    let mut accounts = prefix_accounts(
        &inputs.payer,
        &inputs.incoming_message_pda,
        &inputs.message_payload_pda,
        &inputs.message,
    );

    let unwrapped_payload = match inputs.payload {
        GMPPayload::InterchainTransfer(_)
        | GMPPayload::DeployInterchainToken(_)
        | GMPPayload::LinkToken(_)
        | GMPPayload::RegisterTokenMetadata(_) => inputs.payload,
        GMPPayload::SendToHub(inner) => GMPPayload::decode(&inner.payload)
            .map_err(|_err| ProgramError::InvalidInstructionData)?,
        GMPPayload::ReceiveFromHub(inner) => GMPPayload::decode(&inner.payload)
            .map_err(|_err| ProgramError::InvalidInstructionData)?,
    };

    let mut its_accounts =
        derive_its_accounts(&unwrapped_payload, inputs.token_program, inputs.mint)?;

    accounts.append(&mut its_accounts);

    let data = to_vec(&InterchainTokenServiceInstruction::Execute {
        message: inputs.message,
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
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn transfer_operatorship(payer: Pubkey, to: Pubkey) -> Result<Instruction, ProgramError> {
    let (its_root_pda, _) = crate::find_its_root_pda();
    let (payer_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::id(), &its_root_pda, &payer);
    let (destination_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::id(), &its_root_pda, &to);

    let accounts = vec![
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new(payer, true),
        AccountMeta::new(payer_roles_pda, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(to, false),
        AccountMeta::new(destination_roles_pda, false),
    ];

    let data = to_vec(&InterchainTokenServiceInstruction::TransferOperatorship)?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::ProposeOperatorship`] instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn propose_operatorship(payer: Pubkey, to: Pubkey) -> Result<Instruction, ProgramError> {
    let (its_root_pda, _) = crate::find_its_root_pda();
    let (payer_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::id(), &its_root_pda, &payer);
    let (destination_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::id(), &its_root_pda, &to);
    let (proposal_pda, _) =
        role_management::find_roles_proposal_pda(&crate::id(), &its_root_pda, &payer, &to);

    let accounts = vec![
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
        AccountMeta::new(payer, true),
        AccountMeta::new_readonly(payer_roles_pda, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(to, false),
        AccountMeta::new_readonly(destination_roles_pda, false),
        AccountMeta::new(proposal_pda, false),
    ];

    let data = to_vec(&InterchainTokenServiceInstruction::ProposeOperatorship)?;

    Ok(Instruction {
        program_id: crate::ID,
        accounts,
        data,
    })
}

/// Creates an [`InterchainTokenServiceInstruction::AcceptOperatorship`] instruction.
///
/// # Errors
///
/// [`ProgramError::BorshIoError`]: When instruction serialization fails.
pub fn accept_operatorship(payer: Pubkey, from: Pubkey) -> Result<Instruction, ProgramError> {
    let (its_root_pda, _) = crate::find_its_root_pda();
    let (payer_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::id(), &its_root_pda, &payer);
    let (origin_roles_pda, _) =
        role_management::find_user_roles_pda(&crate::id(), &its_root_pda, &from);
    let (proposal_pda, _) =
        role_management::find_roles_proposal_pda(&crate::id(), &its_root_pda, &from, &payer);

    let accounts = vec![
        AccountMeta::new_readonly(solana_program::system_program::id(), false),
        AccountMeta::new(payer, true),
        AccountMeta::new(payer_roles_pda, false),
        AccountMeta::new_readonly(its_root_pda, false),
        AccountMeta::new_readonly(from, false),
        AccountMeta::new(origin_roles_pda, false),
        AccountMeta::new(proposal_pda, false),
    ];

    let data = to_vec(&InterchainTokenServiceInstruction::AcceptOperatorship)?;

    Ok(Instruction {
        program_id: crate::ID,
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
    let destination_program = DestinationProgramId(crate::ID);
    let (gateway_approved_message_signing_pda, _) = destination_program.signing_pda(&command_id);

    vec![
        AccountMeta::new(*payer, true),
        AccountMeta::new(*gateway_incoming_message_pda, false),
        AccountMeta::new_readonly(*gateway_message_payload_pda, false),
        AccountMeta::new_readonly(gateway_approved_message_signing_pda, false),
        AccountMeta::new_readonly(axelar_solana_gateway::ID, false),
    ]
}

pub(crate) fn derive_its_accounts<'a, T>(
    payload: T,
    token_program: Pubkey,
    maybe_mint: Option<Pubkey>,
) -> Result<Vec<AccountMeta>, ProgramError>
where
    T: TryInto<ItsMessageRef<'a>>,
{
    let message: ItsMessageRef<'_> = payload
        .try_into()
        .map_err(|_err| ProgramError::InvalidInstructionData)?;
    if let ItsMessageRef::DeployInterchainToken { .. } = message {
        if token_program != spl_token_2022::ID {
            return Err(ProgramError::InvalidInstructionData);
        }
    }

    let (mut accounts, mint, token_manager_pda) =
        derive_common_its_accounts(token_program, &message, maybe_mint)?;

    let mut message_specific_accounts =
        derive_specific_its_accounts(&message, mint, token_manager_pda, token_program)?;

    accounts.append(&mut message_specific_accounts);

    Ok(accounts)
}

fn derive_specific_its_accounts(
    message: &ItsMessageRef<'_>,
    mint_account: Pubkey,
    token_manager_pda: Pubkey,
    token_program: Pubkey,
) -> Result<Vec<AccountMeta>, ProgramError> {
    let mut specific_accounts = Vec::new();

    match message {
        ItsMessageRef::InterchainTransfer {
            destination_address,
            data,
            ..
        } => {
            let wallet = Pubkey::new_from_array(
                (*destination_address)
                    .try_into()
                    .map_err(|_err| ProgramError::InvalidInstructionData)?,
            );

            let destination_ata = get_associated_token_address_with_program_id(
                &wallet,
                &mint_account,
                &token_program,
            );

            specific_accounts.push(AccountMeta::new(wallet, false));
            specific_accounts.push(AccountMeta::new(destination_ata, false));

            if !data.is_empty() {
                let (its_transfer_execute_pda, _bump) =
                    crate::find_interchain_transfer_execute_pda(&wallet);
                specific_accounts.push(AccountMeta::new(its_transfer_execute_pda, false));
                let execute_data = DataPayload::decode(data)
                    .map_err(|_err| ProgramError::InvalidInstructionData)?;
                specific_accounts.extend(execute_data.account_meta().iter().cloned());
            }
        }
        ItsMessageRef::DeployInterchainToken { minter, .. } => {
            let (metadata_account_key, _) =
                mpl_token_metadata::accounts::Metadata::find_pda(&mint_account);

            specific_accounts.push(AccountMeta::new_readonly(sysvar::instructions::ID, false));
            specific_accounts.push(AccountMeta::new_readonly(mpl_token_metadata::ID, false));
            specific_accounts.push(AccountMeta::new(metadata_account_key, false));
            // No initial supply is ever set through GMP, so no payer ATA required.
            specific_accounts.push(AccountMeta::new_readonly(crate::ID, false));

            if !minter.is_empty() {
                let minter_key = Pubkey::new_from_array(
                    (*minter)
                        .try_into()
                        .map_err(|_err| ProgramError::InvalidInstructionData)?,
                );
                let (minter_roles_pda, _) = role_management::find_user_roles_pda(
                    &crate::ID,
                    &token_manager_pda,
                    &minter_key,
                );

                specific_accounts.push(AccountMeta::new_readonly(minter_key, false));
                specific_accounts.push(AccountMeta::new(minter_roles_pda, false));
            }
        }
        ItsMessageRef::LinkToken { link_params, .. } => {
            if let Ok(operator) = Pubkey::try_from(*link_params) {
                let (operator_roles_pda, _) =
                    role_management::find_user_roles_pda(&crate::ID, &token_manager_pda, &operator);

                specific_accounts.push(AccountMeta::new_readonly(operator, false));
                specific_accounts.push(AccountMeta::new(operator_roles_pda, false));
            }
        }
    };

    Ok(specific_accounts)
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
        ItsMessageRef::LinkToken {
            destination_token_address,
            ..
        } => Pubkey::try_from(*destination_token_address)
            .map_err(|_err| ProgramError::InvalidInstructionData),
        ItsMessageRef::InterchainTransfer { .. } => {
            maybe_mint.ok_or(ProgramError::InvalidInstructionData)
        }
        ItsMessageRef::DeployInterchainToken { .. } => Ok(*interchain_token_pda),
    }
}

fn derive_common_its_accounts(
    token_program: Pubkey,
    message: &ItsMessageRef<'_>,
    maybe_mint: Option<Pubkey>,
) -> Result<(Vec<AccountMeta>, Pubkey, Pubkey), ProgramError> {
    let (its_root_pda, _) = crate::find_its_root_pda();
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
        role_management::find_user_roles_pda(&crate::ID, &token_manager_pda, &its_root_pda);

    Ok((
        vec![
            AccountMeta::new_readonly(system_program::ID, false),
            AccountMeta::new_readonly(its_root_pda, false),
            AccountMeta::new(token_manager_pda, false),
            AccountMeta::new(token_mint, false),
            AccountMeta::new(token_manager_ata, false),
            AccountMeta::new_readonly(token_program, false),
            AccountMeta::new_readonly(spl_associated_token_account::ID, false),
            AccountMeta::new(its_user_roles_pda, false),
            AccountMeta::new_readonly(sysvar::rent::ID, false),
        ],
        token_mint,
        token_manager_pda,
    ))
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
    LinkToken {
        token_id: Cow<'a, [u8; 32]>,
        source_token_address: &'a [u8],
        destination_token_address: &'a [u8],
        token_manager_type: state::token_manager::Type,
        link_params: &'a [u8],
    },
}

impl ItsMessageRef<'_> {
    /// Returns the token id for the message.
    pub(crate) fn token_id(&self) -> &[u8; 32] {
        match self {
            ItsMessageRef::InterchainTransfer { token_id, .. }
            | ItsMessageRef::DeployInterchainToken { token_id, .. }
            | ItsMessageRef::LinkToken { token_id, .. } => token_id,
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
            GMPPayload::LinkToken(inner) => Self::LinkToken {
                token_id: Cow::Borrowed(&inner.token_id.0),
                source_token_address: inner.source_token_address.as_ref(),
                destination_token_address: inner.destination_token_address.as_ref(),
                token_manager_type: inner
                    .token_manager_type
                    .try_into()
                    .map_err(|_err| ProgramError::InvalidInstructionData)?,
                link_params: inner.link_params.as_ref(),
            },
            GMPPayload::RegisterTokenMetadata(_)
            | GMPPayload::SendToHub(_)
            | GMPPayload::ReceiveFromHub(_) => return Err(ProgramError::InvalidArgument),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use solana_program::{program_error::ProgramError, pubkey::Pubkey};

    use super::{derive_specific_its_accounts, ItsMessageRef};

    #[test]
    fn test_deploy_interchain_token_with_invalid_minter_pubkey() {
        let token_id = [0u8; 32];
        let name = "Test Token";
        let symbol = "TST";
        let decimals = 6;

        // Test with invalid minter (not 32 bytes)
        let invalid_minter = vec![1, 2, 3, 4, 5]; // Only 5 bytes, should be 32

        let message = ItsMessageRef::DeployInterchainToken {
            token_id: Cow::Borrowed(&token_id),
            name,
            symbol,
            decimals,
            minter: &invalid_minter,
        };

        let mint_account = Pubkey::new_unique();
        let token_manager_pda = Pubkey::new_unique();
        let token_program = spl_token_2022::ID;

        // This should fail with InvalidInstructionData because minter is not empty but also not 32 bytes
        let result =
            derive_specific_its_accounts(&message, mint_account, token_manager_pda, token_program);

        assert_eq!(result.unwrap_err(), ProgramError::InvalidInstructionData);
    }

    #[test]
    fn test_deploy_interchain_token_with_empty_minter() {
        let token_id = [0u8; 32];
        let name = "Test Token";
        let symbol = "TST";
        let decimals = 6;

        // Test with empty minter
        let empty_minter = vec![];

        let message = ItsMessageRef::DeployInterchainToken {
            token_id: Cow::Borrowed(&token_id),
            name,
            symbol,
            decimals,
            minter: &empty_minter,
        };

        let mint_account = Pubkey::new_unique();
        let token_manager_pda = Pubkey::new_unique();
        let token_program = spl_token_2022::ID;

        // This should succeed because empty minter is allowed
        let result =
            derive_specific_its_accounts(&message, mint_account, token_manager_pda, token_program);

        assert!(result.is_ok());
        let accounts = result.unwrap();

        // Should have 4 accounts (sysvar::instructions, mpl_token_metadata, metadata_account, crate::ID)
        // but no minter-related accounts
        assert_eq!(accounts.len(), 4);
    }

    #[test]
    fn test_deploy_interchain_token_with_valid_minter() {
        let token_id = [0u8; 32];
        let name = "Test Token";
        let symbol = "TST";
        let decimals = 6;

        // Test with valid minter (32 bytes)
        let valid_minter_pubkey = Pubkey::new_unique();
        let valid_minter = valid_minter_pubkey.to_bytes().to_vec();

        let message = ItsMessageRef::DeployInterchainToken {
            token_id: Cow::Borrowed(&token_id),
            name,
            symbol,
            decimals,
            minter: &valid_minter,
        };

        let mint_account = Pubkey::new_unique();
        let token_manager_pda = Pubkey::new_unique();
        let token_program = spl_token_2022::ID;

        // This should succeed because minter is exactly 32 bytes
        let result =
            derive_specific_its_accounts(&message, mint_account, token_manager_pda, token_program);

        assert!(result.is_ok());
        let accounts = result.unwrap();

        // Should have 6 accounts (4 base accounts + minter_key + minter_roles_pda)
        assert_eq!(accounts.len(), 6);
    }
}
