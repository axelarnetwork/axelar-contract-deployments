//! Main instructions for the governance contract.

use axelar_solana_encoding::types::messages::Message;
use borsh::{BorshDeserialize, BorshSerialize};

use crate::state::proposal::ExecuteProposalData;
use crate::state::GovernanceConfig;

/// Instructions supported by the governance program.
#[derive(Debug, Eq, PartialEq, Clone, BorshSerialize, BorshDeserialize)]
pub enum GovernanceInstruction {
    /// Initializes the governance configuration PDA account.
    ///
    /// 0. [WRITE, SIGNER] Payer account
    /// 1. [WRITE] Config PDA account
    /// 2. [] System program account
    InitializeConfig(GovernanceConfig),

    /// A GMP instruction coming from the axelar network.
    /// The very first accounts are the gateways accounts:  
    ///
    /// 0. [WRITE] Gateway incoming message PDA account
    /// 1. [] Message payload account
    /// 2. [] Approved message signing PDA account
    /// 3. [] Gateway program account
    ///
    /// Above accounts, are accompanied by GMP commands specific accounts:
    ///
    /// --> GMP Schedule time lock proposal
    ///
    /// 0. [] System program account
    /// 1. [WRITE, SIGNER] Payer account
    /// 2. [WRITE] Config PDA account
    /// 3. [WRITE] Prop PDA account
    ///
    /// --> GMP Cancel time lock proposal
    ///
    /// 0. [] System program account
    /// 1. [WRITE, SIGNER] Payer account
    /// 2. [WRITE] Config PDA account
    /// 3. [WRITE] Prop PDA account
    ///
    /// --> GMP Approve operator proposal
    ///
    /// 0. [] System program account
    /// 1. [WRITE, SIGNER] Payer account
    /// 2. [] Config PDA account
    /// 3. [WRITE] Prop operator account
    ///
    /// --> GMP Cancel operator proposal
    ///
    /// 0. [] System program account
    /// 1. [WRITE, SIGNER] Payer account
    /// 2. [] Config PDA account
    /// 3. [WRITE] Prop operator account
    ProcessGmp {
        /// The GMP message metadata. The payload is retrieved later from
        ///  its dedicated account.
        message: Message,
    },

    /// Execute a given proposal. Anyone from the Solana network can execute a
    /// proposal.
    ///
    ///
    /// 0. [] System program account
    /// 1. [] Payer account
    /// 2. [WRITE] Config PDA account
    /// 3. [WRITE] Prop PDA account
    ExecuteProposal(ExecuteProposalData),

    /// Execute a given proposal as operator. Only the designed operator can
    /// execute the proposal.
    ///
    ///
    /// 0. [] System program account
    /// 1. [] Payer account
    /// 2. [WRITE] Config PDA account
    /// 3. [WRITE] Prop PDA account
    /// 4. [] Operator PDA account
    /// 5. [WRITE] Prop operator account
    ExecuteOperatorProposal(ExecuteProposalData),

    /// Withdraw governing tokens from this program config account.
    ///
    /// 0. [] System program account
    /// 1. [WRITE, SIGNER] Config PDA account
    /// 2. [WRITE] Funds receiver account
    /// 3. [] Program ID account
    WithdrawTokens {
        /// The amount to withdraw.
        amount: u64,
    },

    /// Transfer the operatorship of the governance program. Only the current
    /// operator or this contract via a CPI call can transfer the operatorship.
    ///
    /// 0. [] System program account
    /// 1. [SIGNER] Payer account
    /// 2. [SIGNER] Operator PDA account
    /// 3. [WRITE] Config PDA account
    TransferOperatorship {
        /// The new operator pubkey bytes. See [`Pubkey::to_bytes`].
        new_operator: [u8; 32],
    },
}

#[allow(clippy::unwrap_used)] // All the unwraps are safe.
#[allow(clippy::must_use_candidate)]
#[allow(clippy::missing_panics_doc)] // It never will panic, as all the unwraps are safe.
#[allow(clippy::new_without_default)]
pub mod builder {
    //! A module for facilitating the construction of governance instructions
    //! from an user perspective, abstracting mundane, intermediate operations
    //! for the sake of user experience.
    //!
    //! It provides a builder pattern to enforce the correct order of operations
    //! with compile time safety. It also provide convenience getters in case
    //! access to intermediate data is needed. The attributes of the builder
    //! are public, so they can be accessed directly in execeptional cases,
    //! like testing edge cases by manipulating the builder's state.
    //!
    //! # Design
    //!
    //! The builder is a state machine whose states are documented below. It
    //! enforces a call tree that can be cloned at each stage, so the
    //! calculations can be reused. It also exposes certain getters to
    //! facilitate gathering the intermediate data like intermediate PDA
    //! accounts, hashes, etc. This is normally useful for testing purposes.
    //!
    //! Most of the data operations are around the proposal itself. All the
    //! intermediate PDA accounts, hashes, etc are not exposed to the user, so
    //! the first thing to require is the proposal data in most cases, except on
    //! very basic, first level instructions. From there, the builder can be
    //! used to create the needed instructions related to the previous provided
    //! proposal data. This is particularly useful, as the proposal data can
    //! be reused in different instructions once provided.
    //!
    //! # Example
    //!
    //! The fluent api should be used to create the instructions. Feel free to
    //! explore the [`self::test`] for more examples.

    use core::marker::PhantomData;

    use alloy_sol_types::SolValue;
    use axelar_solana_encoding::types::messages::Message;
    use axelar_solana_gateway::state::incoming_message::command_id;
    use borsh::to_vec;
    use governance_gmp::alloy_primitives::Uint;
    use governance_gmp::{GovernanceCommand, GovernanceCommandPayload};
    use program_utils::{checked_from_u256_le_bytes_to_u64, from_u64_to_u256_le_bytes};
    use solana_program::instruction::{AccountMeta, Instruction};
    use solana_program::keccak::hash;
    use solana_program::program_error::ProgramError;
    use solana_program::pubkey::Pubkey;
    use solana_program::{bpf_loader_upgradeable, msg, system_program};

    use super::GovernanceInstruction;
    use crate::processor::gmp;
    use crate::state::operator::derive_managed_proposal_pda;
    use crate::state::proposal::{
        ExecutableProposal, ExecuteProposalCallData, ExecuteProposalData,
    };
    use crate::state::GovernanceConfig;

    /// The initial stage of the builder. This instantiates the builder itself
    /// with all it's data set to None, which goes in cascade and its updated on
    /// each stage accordingly. Through its associated functions, next
    /// stages can be:
    ///
    /// * [`ProposalRelated`].
    /// * [`TransferOperatorshipBuild`].
    /// * [`ConfigBuild`].
    #[derive(Clone, Debug)]
    pub struct Init;

    /// After setting the proposal data, the next stage is to decide what do do
    /// with that information. It also provides getters for getting the computed
    /// information from proposal data calculations.
    ///
    ///  By using the builder's functions, the next stages can be:
    /// * [`GmpMeta`].
    /// * [`ExecuteOperatorProposalBuild`].
    /// * [`ExecuteProposalBuild`].
    #[derive(Clone, Debug)]
    pub struct ProposalRelated;

    /// At this stage of the builder, the metadata for the GMP instruction is
    /// set. The next stage is to decide what to do with the GMP instruction.
    /// This functions stage provide access to:
    /// * [`GmpIx`].
    #[derive(Clone, Debug)]
    pub struct GmpMeta;

    /// At this stage of the builder, the GMP instructions are built. Functions
    /// of this stage provide access to:
    /// * [`GmpBuild`].
    #[derive(Clone, Debug)]
    pub struct GmpIx;

    /// Stage of the builder where the instruction for initializing the
    /// governance config its built.
    #[derive(Clone, Debug)]
    pub struct ConfigBuild;

    /// Stage of the builder where the instruction for a GMP command is built.
    #[derive(Clone, Debug)]
    pub struct GmpBuild;

    /// Stage of the builder where the instruction for executing a proposal is
    /// built.
    #[derive(Clone, Debug)]
    pub struct ExecuteProposalBuild;

    /// Stage of the builder where the instruction for executing an operator
    /// approved proposal is built.
    #[derive(Clone, Debug)]
    pub struct ExecuteOperatorProposalBuild;

    /// Stage of the builder where the instruction for transferring the
    /// operatorship is built.
    #[derive(Clone, Debug)]
    pub struct TransferOperatorshipBuild;

    /// A builder for governance instructions.
    #[allow(clippy::module_name_repetitions)]
    #[derive(Clone, Debug)]
    pub struct IxBuilder<Stage = Init> {
        /// The accounts needed for the instruction.
        pub accounts: Option<Vec<AccountMeta>>,
        /// The governance config. Only used in the [`ConfigBuild`] stage.
        pub config: Option<GovernanceConfig>,
        /// The new operator pubkey. Only used in the
        /// [`TransferOperatorshipBuild`] stage.
        pub new_operator: Option<Pubkey>,
        /// The stage of the builder.
        pub stage: PhantomData<Stage>,
        /// The GMP metadata. Only used in the [`GmpMeta`] stage.
        pub gmp_msg_meta: Option<Message>,
        /// The GMP command. Only used in the [`GmpBuild`] stage.
        pub gmp_command: Option<GovernanceCommand>,
        /// The proposal target pubkey. Only used in the [`ProposalRelated`]
        /// stage.
        pub prop_target: Option<Pubkey>,
        /// The proposal native value. Only used in the [`ProposalRelated`]
        /// stage.
        pub prop_native_value: Option<u64>,
        /// The proposal ETA. Only used in the [`ProposalRelated`] stage.
        pub prop_eta: Option<u64>,
        /// The proposal PDA. Only used in the [`ProposalRelated`] stage.
        pub prop_pda: Option<Pubkey>,
        /// The proposal hash. Only used in the [`ProposalRelated`] stage.
        pub prop_hash: Option<[u8; 32]>,
        /// The proposal operator PDA. Only used in the [`ProposalRelated`]
        /// stage.
        pub prop_operator_pda: Option<Pubkey>,
        /// The proposal call data. Only used in the [`ProposalRelated`] stage.
        pub prop_call_data: Option<ExecuteProposalCallData>,
    }

    impl IxBuilder<Init> {
        /// Creates a new builder.
        pub const fn new() -> Self {
            Self {
                accounts: None,
                config: None,
                new_operator: None,
                stage: PhantomData::<Init>,
                gmp_command: None,
                gmp_msg_meta: None,
                prop_target: None,
                prop_native_value: None,
                prop_eta: None,
                prop_pda: None,
                prop_hash: None,
                prop_operator_pda: None,
                prop_call_data: None,
            }
        }
        /// Sets the proposal data for the builder. All subsequent operations
        /// for pdas and hashes calculations will be shared in next
        /// stages. It provides access to next stage [`ProposalRelated`].
        pub fn with_proposal_data(
            self,
            target: Pubkey,
            native_value: u64,
            eta: u64,
            native_value_target_account: Option<AccountMeta>,
            gmp_prop_target_accounts: &[AccountMeta],
            data: Vec<u8>,
        ) -> IxBuilder<ProposalRelated> {
            let gmp_prop_target_accounts = gmp_prop_target_accounts
                .iter()
                .map(core::convert::Into::into)
                .collect();

            let gmp_prop_native_value_target_account =
                native_value_target_account.map(core::convert::Into::into);

            let call_data = ExecuteProposalCallData::new(
                gmp_prop_target_accounts,
                gmp_prop_native_value_target_account,
                data,
            );

            let hash = ExecutableProposal::calculate_hash(
                &target,
                &call_data,
                &from_u64_to_u256_le_bytes(native_value),
            );
            let (gov_proposal_pda, _) = ExecutableProposal::pda(&hash);
            let (operator_proposal_managed_pda, _) = derive_managed_proposal_pda(&hash);

            IxBuilder {
                accounts: self.accounts,
                config: self.config,
                new_operator: self.new_operator,
                stage: PhantomData::<ProposalRelated>,
                gmp_command: None,
                gmp_msg_meta: self.gmp_msg_meta,
                prop_target: Some(target),
                prop_native_value: Some(native_value),
                prop_eta: Some(eta),
                prop_pda: Some(gov_proposal_pda),
                prop_hash: Some(hash),
                prop_operator_pda: Some(operator_proposal_managed_pda),
                prop_call_data: Some(call_data),
            }
        }
        /// Creates a new instruction for the governance config initialization.
        /// It provides access to next stage [`ConfigBuild`].
        pub fn initialize_config(
            self,
            payer: &Pubkey,
            config_pda: &Pubkey,
            config: GovernanceConfig,
        ) -> IxBuilder<ConfigBuild> {
            let program_data_pda = bpf_loader_upgradeable::get_program_data_address(&crate::ID);
            let accounts = vec![
                AccountMeta::new(*payer, true),
                AccountMeta::new_readonly(program_data_pda, false),
                AccountMeta::new(*config_pda, false),
                AccountMeta::new_readonly(system_program::ID, false),
            ];

            IxBuilder {
                accounts: Some(accounts),
                config: Some(config),
                new_operator: self.new_operator,
                stage: PhantomData::<ConfigBuild>,
                gmp_command: None,
                gmp_msg_meta: self.gmp_msg_meta,
                prop_target: self.prop_target,
                prop_native_value: self.prop_native_value,
                prop_eta: self.prop_eta,
                prop_pda: self.prop_pda,
                prop_hash: self.prop_hash,
                prop_operator_pda: self.prop_operator_pda,
                prop_call_data: self.prop_call_data,
            }
        }
        /// Creates a new instruction for transferring the operatorship of the
        /// governance program. It provides access to next builder stage
        /// [`TransferOperatorshipBuild`].
        pub fn transfer_operatorship(
            self,
            payer: &Pubkey,
            operator_pda: &Pubkey,
            config_pda: &Pubkey,
            new_operator: &Pubkey,
        ) -> IxBuilder<TransferOperatorshipBuild> {
            let accounts = vec![
                AccountMeta::new_readonly(system_program::ID, false),
                AccountMeta::new_readonly(*payer, true),
                AccountMeta::new_readonly(*operator_pda, true),
                AccountMeta::new(*config_pda, false),
            ];

            IxBuilder {
                accounts: Some(accounts),
                config: self.config,
                new_operator: Some(*new_operator),
                stage: PhantomData::<TransferOperatorshipBuild>,
                gmp_command: self.gmp_command,
                gmp_msg_meta: self.gmp_msg_meta,
                prop_target: self.prop_target,
                prop_native_value: self.prop_native_value,
                prop_eta: self.prop_eta,
                prop_pda: self.prop_pda,
                prop_hash: self.prop_hash,
                prop_operator_pda: self.prop_operator_pda,
                prop_call_data: self.prop_call_data,
            }
        }

        /// Prepares the builder for sending an scheduled time lock proposal
        /// that targets the `bpf_loader_upgradeable` program for upgrade.
        pub fn builder_for_program_upgrade(
            target_program: &Pubkey,
            buffer_address: &Pubkey,
            authority_address: &Pubkey,
            spill_address: &Pubkey,
            proposal_eta: u64,
        ) -> IxBuilder<ProposalRelated> {
            let Instruction {
                mut accounts, data, ..
            } = solana_program::bpf_loader_upgradeable::upgrade(
                target_program,
                buffer_address,
                authority_address,
                spill_address,
            );
            accounts.push(AccountMeta::new_readonly(bpf_loader_upgradeable::ID, false));

            Self::new().with_proposal_data(
                // The proposal execution processor should target the bpf_loader_upgradeable::ID .
                bpf_loader_upgradeable::ID,
                0,
                proposal_eta,
                None,
                &accounts,
                data,
            )
        }

        /// This is a builder of a builder. It loads into the builder a proposal
        /// that targets the governance program itself for operatorship
        /// transfer.
        ///
        /// It provides access to the next builder stage [`ProposalRelated`]. In
        /// which a GMP instruction for scheduling the created proposal
        /// can be built (among others).
        pub fn builder_for_operatorship_transfership(
            self,
            payer: &Pubkey,
            config_pda: &Pubkey,
            operator_pda: &Pubkey,
            new_operator_pda: &Pubkey,
            eta: u64,
        ) -> IxBuilder<ProposalRelated> {
            let gmp_prop_target_accounts = &[
                AccountMeta::new_readonly(system_program::ID, false),
                AccountMeta::new_readonly(*payer, true),
                AccountMeta::new_readonly(*operator_pda, false),
                AccountMeta::new(*config_pda, true),
                AccountMeta::new_readonly(crate::ID, false),
            ];

            let data = to_vec(&GovernanceInstruction::TransferOperatorship {
                new_operator: new_operator_pda.to_bytes(),
            })
            .unwrap();

            Self::new().with_proposal_data(crate::ID, 0, eta, None, gmp_prop_target_accounts, data)
        }

        /// This is a builder of a builder. It loads into the builder a proposal
        /// that targets the governance program itself for funds withdrawal.
        ///
        /// It provides access to the next builder stage [`ProposalRelated`]. In
        /// which a GMP instruction for scheduling the created proposal
        /// can be built (among others).
        pub fn builder_for_withdraw_tokens(
            self,
            config_pda: &Pubkey,
            funds_receiver: &Pubkey,
            amount: u64,
            eta: u64,
        ) -> IxBuilder<ProposalRelated> {
            let target_accounts = &[
                AccountMeta::new_readonly(system_program::ID, false),
                AccountMeta::new(*config_pda, true),
                AccountMeta::new(*funds_receiver, false),
                AccountMeta::new_readonly(crate::ID, false),
            ];

            Self::new().with_proposal_data(
                crate::ID,
                0,
                eta,
                None,
                target_accounts,
                to_vec(&GovernanceInstruction::WithdrawTokens { amount }).unwrap(),
            )
        }
    }

    impl IxBuilder<ProposalRelated> {
        /// Creates a GMP instruction for the previously provided proposal.
        ///
        /// It provides access to the next builder stage [`GmpMeta`].
        pub fn gmp_ix(self) -> IxBuilder<GmpMeta> {
            IxBuilder {
                accounts: self.accounts,
                config: self.config,
                new_operator: self.new_operator,
                stage: PhantomData::<GmpMeta>,
                gmp_command: None,
                gmp_msg_meta: self.gmp_msg_meta,
                prop_target: self.prop_target,
                prop_native_value: self.prop_native_value,
                prop_eta: self.prop_eta,
                prop_pda: self.prop_pda,
                prop_hash: self.prop_hash,
                prop_operator_pda: self.prop_operator_pda,
                prop_call_data: self.prop_call_data,
            }
        }

        /// Creates an instruction for executing the previously provided
        /// proposal.
        pub fn execute_proposal(
            self,
            payer: &Pubkey,
            config_pda: &Pubkey,
        ) -> IxBuilder<ExecuteProposalBuild> {
            let mut accounts = vec![
                AccountMeta::new_readonly(system_program::ID, false),
                AccountMeta::new_readonly(*payer, true),
                AccountMeta::new(*config_pda, false),
                AccountMeta::new(self.prop_pda.unwrap(), false),
            ];

            // Accounts needed for the target contract. Read them from the proposal data.
            let call_data = self.prop_call_data.clone().unwrap();
            let target_program_accounts: Vec<AccountMeta> = call_data
                .solana_accounts
                .iter()
                .filter_map(|acc| {
                    // Avoid a repeated config_pda account, that's normally specified in the
                    // proposal data, for targeting the withdraw tokens instruction of
                    // this contract. A CPI call to itself.
                    if acc.pubkey == config_pda.to_bytes() {
                        return None;
                    }
                    Some(AccountMeta::from(acc))
                })
                .collect();

            accounts.extend(target_program_accounts);

            // Add the account to receive the native value if it exists.
            if let Some(ref fund_acc) = call_data.solana_native_value_receiver_account {
                accounts.push(fund_acc.into());
            }

            IxBuilder {
                accounts: Some(accounts),
                config: self.config,
                new_operator: self.new_operator,
                stage: PhantomData::<ExecuteProposalBuild>,
                gmp_command: None,
                gmp_msg_meta: self.gmp_msg_meta,
                prop_target: self.prop_target,
                prop_native_value: self.prop_native_value,
                prop_eta: self.prop_eta,
                prop_pda: self.prop_pda,
                prop_hash: self.prop_hash,
                prop_operator_pda: self.prop_operator_pda,
                prop_call_data: self.prop_call_data,
            }
        }

        /// Creates an instruction for executing the previously provided
        /// proposal, that was previously approved by the Axelar infrastructure
        /// via GMP.
        ///
        /// It provides access to the next builder stage
        /// [`ExecuteOperatorProposalBuild`].
        pub fn execute_operator_proposal(
            self,
            payer: &Pubkey,
            config_pda: &Pubkey,
            operator_pda: &Pubkey,
        ) -> IxBuilder<ExecuteOperatorProposalBuild> {
            let mut accounts = vec![
                AccountMeta::new_readonly(system_program::ID, false),
                AccountMeta::new_readonly(*payer, true),
                AccountMeta::new(*config_pda, false),
                AccountMeta::new(self.prop_pda.unwrap(), false),
                AccountMeta::new_readonly(*operator_pda, true),
                AccountMeta::new(self.prop_operator_pda.unwrap(), false),
            ];

            // Accounts needed for the target contract. Read them from the proposal data.
            let call_data = self.prop_call_data.clone().unwrap();
            let target_program_accounts: Vec<AccountMeta> = call_data
                .solana_accounts
                .iter()
                .filter_map(|acc| {
                    // Avoid a repeated config_pda account, that's normally specified in the
                    // proposal data, for targeting the withdraw tokens instruction of
                    // this contract. A CPI call to itself.
                    if acc.pubkey == config_pda.to_bytes() {
                        return None;
                    }
                    Some(AccountMeta::from(acc))
                })
                .collect();

            accounts.extend(target_program_accounts);

            // Add the account to receive the native value if it exists.
            if let Some(ref fund_acc) = call_data.solana_native_value_receiver_account {
                accounts.push(fund_acc.into());
            }

            IxBuilder {
                accounts: Some(accounts),
                config: self.config,
                new_operator: self.new_operator,
                stage: PhantomData::<ExecuteOperatorProposalBuild>,
                gmp_command: None,
                gmp_msg_meta: self.gmp_msg_meta,
                prop_target: self.prop_target,
                prop_native_value: self.prop_native_value,
                prop_eta: self.prop_eta,
                prop_pda: self.prop_pda,
                prop_hash: self.prop_hash,
                prop_operator_pda: self.prop_operator_pda,
                prop_call_data: self.prop_call_data,
            }
        }

        /// The calculated proposal PDA.
        pub fn proposal_pda(&self) -> Pubkey {
            self.prop_pda.unwrap()
        }
        /// The calculated proposal operator marker PDA.
        pub fn proposal_operator_marker_pda(&self) -> Pubkey {
            self.prop_operator_pda.unwrap()
        }
        /// The calculated proposal hash.
        pub fn proposal_hash(&self) -> [u8; 32] {
            self.prop_hash.unwrap()
        }
        /// The proposal target pubkey.
        pub fn proposal_target_address(&self) -> Pubkey {
            self.prop_target.unwrap()
        }
        /// The proposal call data
        pub fn proposal_call_data(&self) -> ExecuteProposalCallData {
            self.prop_call_data.clone().unwrap()
        }
        /// The proposal native value. U256 le representation.
        pub fn proposal_u256_le_native_value(&self) -> [u8; 32] {
            from_u64_to_u256_le_bytes(self.prop_native_value.unwrap())
        }
        /// The proposal ETA. U256 le representation.
        pub fn proposal_u256_le_eta(&self) -> [u8; 32] {
            from_u64_to_u256_le_bytes(self.prop_eta.unwrap())
        }
    }

    impl IxBuilder<GmpMeta> {
        /// Introduces the gmp metadata for the subsequent GMP instruction.
        ///
        /// It provides access to the next builder stage [`GmpIx`].
        pub fn with_msg_metadata(self, message: Message) -> IxBuilder<GmpIx> {
            IxBuilder {
                accounts: self.accounts,
                config: self.config,
                new_operator: self.new_operator,
                stage: PhantomData::<GmpIx>,
                gmp_command: self.gmp_command,
                gmp_msg_meta: Some(message),
                prop_target: self.prop_target,
                prop_native_value: self.prop_native_value,
                prop_eta: self.prop_eta,
                prop_pda: self.prop_pda,
                prop_hash: self.prop_hash,
                prop_operator_pda: self.prop_operator_pda,
                prop_call_data: self.prop_call_data,
            }
        }
    }

    impl IxBuilder<GmpIx> {
        /// Builds the schedule time lock proposal instruction. It will take the
        /// proposal data from previous stages of the builder.
        ///
        /// It provides access to the next builder stage [`GmpBuild`].
        pub fn schedule_time_lock_proposal(
            self,
            payer: &Pubkey,
            config_pda: &Pubkey,
        ) -> IxBuilder<GmpBuild> {
            let accounts = vec![
                AccountMeta::new_readonly(system_program::ID, false),
                AccountMeta::new(*payer, true),
                AccountMeta::new(*config_pda, false),
                AccountMeta::new(self.prop_pda.unwrap(), false),
            ];
            IxBuilder {
                accounts: Some(accounts),
                config: self.config,
                new_operator: self.new_operator,
                stage: PhantomData::<GmpBuild>,
                gmp_command: Some(GovernanceCommand::ScheduleTimeLockProposal),
                gmp_msg_meta: self.gmp_msg_meta,
                prop_target: self.prop_target,
                prop_native_value: self.prop_native_value,
                prop_eta: self.prop_eta,
                prop_pda: self.prop_pda,
                prop_hash: self.prop_hash,
                prop_operator_pda: self.prop_operator_pda,
                prop_call_data: self.prop_call_data,
            }
        }

        /// Builds the cancel time lock proposal instruction. It will take the
        /// proposal data from previous stages of the builder.
        ///
        /// It provides access to the next builder stage [`GmpBuild`].
        ///
        /// # Panics
        ///
        /// If the builder data is not set. But this should never happen.
        pub fn cancel_time_lock_proposal(
            self,
            payer: &Pubkey,
            config_pda: &Pubkey,
        ) -> IxBuilder<GmpBuild> {
            let accounts = vec![
                AccountMeta::new_readonly(system_program::ID, false),
                AccountMeta::new(*payer, true),
                AccountMeta::new(*config_pda, false),
                AccountMeta::new(self.prop_pda.unwrap(), false),
            ];

            IxBuilder {
                accounts: Some(accounts),
                config: self.config,
                new_operator: self.new_operator,
                stage: PhantomData::<GmpBuild>,
                gmp_command: Some(GovernanceCommand::CancelTimeLockProposal),
                gmp_msg_meta: self.gmp_msg_meta,
                prop_target: self.prop_target,
                prop_native_value: self.prop_native_value,
                prop_eta: self.prop_eta,
                prop_pda: self.prop_pda,
                prop_hash: self.prop_hash,
                prop_operator_pda: self.prop_operator_pda,
                prop_call_data: self.prop_call_data,
            }
        }

        /// Builds the approve operator proposal instruction. It will take the
        /// proposal data from previous stages of the builder.
        ///
        /// It provides access to the next builder stage [`GmpBuild`].
        pub fn approve_operator_proposal(
            self,
            payer: &Pubkey,
            config_pda: &Pubkey,
        ) -> IxBuilder<GmpBuild> {
            let accounts = vec![
                AccountMeta::new_readonly(system_program::ID, false),
                AccountMeta::new(*payer, true),
                AccountMeta::new_readonly(*config_pda, false),
                AccountMeta::new_readonly(self.prop_pda.unwrap(), false),
                AccountMeta::new(self.prop_operator_pda.unwrap(), false),
            ];

            IxBuilder {
                accounts: Some(accounts),
                config: self.config,
                new_operator: self.new_operator,
                stage: PhantomData::<GmpBuild>,
                gmp_command: Some(GovernanceCommand::ApproveOperatorProposal),
                gmp_msg_meta: self.gmp_msg_meta,
                prop_target: self.prop_target,
                prop_native_value: self.prop_native_value,
                prop_eta: self.prop_eta,
                prop_pda: self.prop_pda,
                prop_hash: self.prop_hash,
                prop_operator_pda: self.prop_operator_pda,
                prop_call_data: self.prop_call_data,
            }
        }

        /// Builds the schedule time lock proposal instruction. It will take the
        /// proposal data from previous stages of the builder.
        ///
        /// It provides access to the next builder stage [`GmpBuild`].
        pub fn cancel_operator_proposal(
            self,
            payer: &Pubkey,
            config_pda: &Pubkey,
        ) -> IxBuilder<GmpBuild> {
            let accounts = vec![
                AccountMeta::new_readonly(system_program::ID, false),
                AccountMeta::new(*payer, true),
                AccountMeta::new(*config_pda, false),
                AccountMeta::new_readonly(self.prop_pda.unwrap(), false),
                AccountMeta::new(self.prop_operator_pda.unwrap(), false),
            ];

            IxBuilder {
                accounts: Some(accounts),
                config: self.config,
                new_operator: self.new_operator,
                stage: PhantomData::<GmpBuild>,
                gmp_command: Some(GovernanceCommand::CancelOperatorApproval),
                gmp_msg_meta: self.gmp_msg_meta,
                prop_target: self.prop_target,
                prop_native_value: self.prop_native_value,
                prop_eta: self.prop_eta,
                prop_pda: self.prop_pda,
                prop_hash: self.prop_hash,
                prop_operator_pda: self.prop_operator_pda,
                prop_call_data: self.prop_call_data,
            }
        }
    }

    impl IxBuilder<ExecuteProposalBuild> {
        /// Builds the instruction for executing the proposal. This is a final
        /// builder stage.
        pub fn build(self) -> Instruction {
            let accounts = self.accounts.unwrap();
            let target_address = self.prop_target.unwrap();
            let call_data = self.prop_call_data.unwrap();
            let native_value = from_u64_to_u256_le_bytes(self.prop_native_value.unwrap());

            let gov_instruction = GovernanceInstruction::ExecuteProposal(ExecuteProposalData::new(
                target_address.to_bytes(),
                call_data,
                native_value,
            ));

            let data = to_vec(&gov_instruction).expect("Unable to encode GovernanceInstruction");

            Instruction {
                program_id: crate::id(),
                accounts,
                data,
            }
        }
    }

    impl IxBuilder<ExecuteOperatorProposalBuild> {
        /// Builds the instruction for executing the operator approved proposal.
        /// This is a final builder stage.
        pub fn build(self) -> Instruction {
            let accounts = self.accounts.unwrap();
            let target_address = self.prop_target.unwrap();
            let call_data = self.prop_call_data.unwrap();
            let native_value = from_u64_to_u256_le_bytes(self.prop_native_value.unwrap());

            let gov_instruction = GovernanceInstruction::ExecuteOperatorProposal(
                ExecuteProposalData::new(target_address.to_bytes(), call_data, native_value),
            );

            let data = to_vec(&gov_instruction).expect("Unable to encode GovernanceInstruction");

            Instruction {
                program_id: crate::id(),
                accounts,
                data,
            }
        }
    }

    impl IxBuilder<ConfigBuild> {
        /// Builds the instruction for initializing the governance config. This
        /// is a final builder stage.
        pub fn build(self) -> Instruction {
            let accounts = self.accounts.unwrap();
            let config = self.config.unwrap();

            let data = to_vec(&GovernanceInstruction::InitializeConfig(config))
                .expect("Unable to encode GovernanceInstruction");

            Instruction {
                program_id: crate::id(),
                accounts,
                data,
            }
        }
    }

    impl IxBuilder<TransferOperatorshipBuild> {
        /// Builds the instruction for transferring the operatorship. This is a
        /// final builder stage.
        pub fn build(self) -> Instruction {
            let accounts = self.accounts.unwrap();
            let new_operator = self.new_operator.unwrap();

            let data = to_vec(&GovernanceInstruction::TransferOperatorship {
                new_operator: new_operator.to_bytes(),
            })
            .expect("Unable to encode GovernanceInstruction");

            Instruction {
                program_id: crate::id(),
                accounts,
                data,
            }
        }
    }

    impl IxBuilder<GmpBuild> {
        /// Builds the instruction for the GMP command. This is a final builder
        /// stage.
        pub fn build(self) -> GmpCallData {
            let accounts = self.accounts.unwrap();
            let mut gmp_msg_meta = self.gmp_msg_meta.unwrap();
            let gmp_command = self.gmp_command.unwrap();
            let gmp_prop_target = self.prop_target.unwrap();
            let gmp_prop_native_value = self.prop_native_value.unwrap();
            let gmp_prop_eta = self.prop_eta.unwrap();
            let gmp_prop_call_data = self.prop_call_data.unwrap();

            let governance_command = GovernanceCommandPayload {
                command: gmp_command,
                target: gmp_prop_target.to_bytes().into(),
                call_data: to_vec(&gmp_prop_call_data).unwrap().into(),
                native_value: Uint::from(gmp_prop_native_value),
                eta: Uint::from(gmp_prop_eta),
            };

            let payload = governance_command.abi_encode();
            gmp_msg_meta.payload_hash = hash(&payload).to_bytes();

            let gov_instruction = GovernanceInstruction::ProcessGmp {
                message: gmp_msg_meta.clone(),
            };
            let data = to_vec(&gov_instruction).unwrap();

            GmpCallData::new(
                Instruction {
                    program_id: crate::id(),
                    accounts,
                    data,
                },
                gmp_msg_meta,
                payload,
            )
        }
    }

    /// A struct representing the GMP call data.
    pub struct GmpCallData {
        /// The ix generated by the builder, ready to be sent.
        pub ix: Instruction,
        /// The message metadata contained in the serialised instruction.
        pub msg_meta: Message,
        /// The raw message payload.
        pub msg_payload: Vec<u8>,
    }

    impl GmpCallData {
        /// Creates a new `GmpCallData` instance.
        ///
        /// # Arguments
        ///
        /// * `ix` - The instruction.
        /// * `msg_meta` - The message metadata.
        /// * `msg_payload` - The message payload.
        pub fn new(ix: Instruction, msg_meta: Message, msg_payload: Vec<u8>) -> Self {
            Self {
                ix,
                msg_meta,
                msg_payload,
            }
        }
    }

    /// Calculates the GMP instruction for a given GMP message.
    ///
    /// # Errors
    ///
    /// Returns an error if the payload is not valid.
    pub fn calculate_gmp_ix(
        payer: Pubkey,
        gateway_incoming_message_pda: Pubkey,
        gateway_message_payload_pda: Pubkey,
        message: &Message,
        payload: &[u8],
    ) -> Result<Instruction, ProgramError> {
        let payload = gmp::payload_conversions::decode_payload(payload)?;
        let call_data = gmp::payload_conversions::decode_payload_call_data(&payload.call_data)?;
        let target = gmp::payload_conversions::decode_payload_target(&payload.target)?;
        let ix_builder = IxBuilder::new();

        let account = call_data
            .solana_native_value_receiver_account
            .map(AccountMeta::from);

        let solana_accounts = call_data
            .solana_accounts
            .iter()
            .map(AccountMeta::from)
            .collect::<Vec<_>>();

        let ix_builder = ix_builder
            .with_proposal_data(
                target,
                checked_from_u256_le_bytes_to_u64(&payload.native_value.to_le_bytes())?,
                checked_from_u256_le_bytes_to_u64(&payload.eta.to_le_bytes())?,
                account,
                &solana_accounts,
                call_data.call_data,
            )
            .gmp_ix()
            .with_msg_metadata(message.clone());

        let config_pda = GovernanceConfig::pda().0;

        let ix_builder = match payload.command {
            GovernanceCommand::ScheduleTimeLockProposal => {
                ix_builder.schedule_time_lock_proposal(&payer, &config_pda)
            }
            GovernanceCommand::CancelTimeLockProposal => {
                ix_builder.cancel_time_lock_proposal(&payer, &config_pda)
            }
            GovernanceCommand::ApproveOperatorProposal => {
                ix_builder.approve_operator_proposal(&payer, &config_pda)
            }
            GovernanceCommand::CancelOperatorApproval => {
                ix_builder.cancel_operator_proposal(&payer, &config_pda)
            }
            _ => {
                msg!("Governance command is not implemented, wrong payload");
                return Err(ProgramError::InvalidInstructionData);
            }
        };

        let mut ix = ix_builder.build().ix;

        prepend_gateway_accounts_to_ix(
            &mut ix,
            gateway_incoming_message_pda,
            gateway_message_payload_pda,
            message,
        );

        Ok(ix)
    }

    /// Prepends the gateway accounts to the instruction.
    /// This is useful for instructions that require the gateway accounts for
    /// message verification in GMP flows.
    pub fn prepend_gateway_accounts_to_ix(
        ix: &mut Instruction,
        gw_incoming_message: Pubkey,
        gw_message_payload: Pubkey,
        message: &Message,
    ) {
        let command_id = command_id(&message.cc_id.chain, &message.cc_id.id);
        let (gateway_approved_message_signing_pda, _) =
            axelar_solana_gateway::get_validate_message_signing_pda(crate::id(), command_id);

        let mut new_accounts = vec![
            AccountMeta::new(gw_incoming_message, false),
            AccountMeta::new_readonly(gw_message_payload, false),
            AccountMeta::new_readonly(gateway_approved_message_signing_pda, false),
            AccountMeta::new_readonly(axelar_solana_gateway::id(), false),
        ];
        // Append the new accounts to the existing ones.
        new_accounts.extend_from_slice(&ix.accounts);
        ix.accounts = new_accounts;
    }
    #[cfg(test)]
    #[allow(clippy::shadow_unrelated)]
    mod test {

        use axelar_solana_encoding::types::messages::CrossChainId;

        use super::*;

        #[test]
        fn simplest_use_case() {
            let payer = Pubkey::new_unique();
            let config_pda = Pubkey::new_unique();
            let config =
                GovernanceConfig::new([0_u8; 32], [0_u8; 32], 1, Pubkey::new_unique().to_bytes());

            let _ix = IxBuilder::new()
                .initialize_config(&payer, &config_pda, config)
                .build();

            // send ix
        }

        #[test]
        fn execute_proposal_use_case() {
            let target = Pubkey::new_unique();
            let native_value = 1;
            let eta = 1;
            let gmp_proposal_target_accounts = [];
            let data = vec![];

            let _ix = IxBuilder::new()
                .with_proposal_data(
                    target,
                    native_value,
                    eta,
                    None,
                    &gmp_proposal_target_accounts,
                    data,
                )
                .execute_proposal(&Pubkey::new_unique(), &Pubkey::new_unique())
                .build();
            // Send ix
        }

        #[test]
        fn sharing_building_stages_for_ix_chaining() {
            // Proposal dummy data
            let target = Pubkey::new_unique();
            let native_value = 1;
            let eta = 1;
            let gmp_proposal_target_accounts = [];
            let data = vec![];

            // Create a base builder with proposal data.
            let base_ix_builder = IxBuilder::new().with_proposal_data(
                target,
                native_value,
                eta,
                None,
                &gmp_proposal_target_accounts,
                data,
            );

            // Schedule the proposal via GMP
            let payer = Pubkey::new_unique();
            let config_pda = Pubkey::new_unique();
            let _ix = base_ix_builder
                .clone()
                .gmp_ix()
                .with_msg_metadata(gmp_sample_metadata())
                .schedule_time_lock_proposal(&payer, &config_pda);

            // Send ix

            // Execute the proposal, no need to replay the data.
            let _ix = base_ix_builder
                .execute_proposal(&Pubkey::new_unique(), &Pubkey::new_unique())
                .build();
        }

        #[test]
        fn using_builders_of_builders_for_creating_things_like_self_calling_complex_proposals() {
            let config_pda = Pubkey::new_unique();
            let funds_receiver = Pubkey::new_unique();
            let eta = 1;

            // This prepares a builder with a proposal that targets the governance module
            // itself and that should be executed later following the
            // traditional flow.
            let base_ix_builder = IxBuilder::new().builder_for_withdraw_tokens(
                &config_pda,
                &funds_receiver,
                eta,
                500,
            );

            // Scheduling the proposal
            let payer = Pubkey::new_unique();
            let _ix = base_ix_builder
                .clone()
                .gmp_ix()
                .with_msg_metadata(gmp_sample_metadata())
                .schedule_time_lock_proposal(&payer, &config_pda);
            // Send ix

            // Executing the proposal, no need to replay data.
            let _ix = base_ix_builder
                .execute_proposal(&Pubkey::new_unique(), &config_pda)
                .build();
            // Send ix
        }

        #[test]
        fn builder_stages_can_have_convenient_getters_per_each_stage() {
            let target = Pubkey::new_unique();
            let native_value = 1;
            let eta = 1;
            let gmp_proposal_target_accounts = [];
            let data = vec![];

            let ix_builder = IxBuilder::new().with_proposal_data(
                target,
                native_value,
                eta,
                None,
                &gmp_proposal_target_accounts,
                data,
            );

            // Get the internally computed from proposal data operator marker PDA,
            // used to mark a proposal as "managed by operator".
            let _proposal_marker_pda = ix_builder.proposal_operator_marker_pda();

            // Check with banks clients it exists ... (for example)
        }

        #[test]
        fn builder_stages_are_strongly_typed_and_can_be_shared_with_other_funcs() {
            let target = Pubkey::new_unique();
            let native_value = 1;
            let eta = 1;
            let gmp_proposal_target_accounts = [];
            let data = vec![];

            let ix_builder = IxBuilder::new().with_proposal_data(
                target,
                native_value,
                eta,
                None,
                &gmp_proposal_target_accounts,
                data,
            );
            other_func(&ix_builder);
        }

        #[allow(clippy::use_debug)]
        #[allow(clippy::print_stdout)]
        fn other_func(ix_builder: &IxBuilder<ProposalRelated>) {
            let _ix = ix_builder
                .clone()
                .execute_proposal(&Pubkey::new_unique(), &Pubkey::new_unique())
                .build();

            // Send ix

            println!(
                "Hello, just executed prop: {:x?}",
                ix_builder.proposal_hash()
            );
        }

        fn gmp_sample_metadata() -> Message {
            Message {
                cc_id: CrossChainId {
                    chain: "chain".to_owned(),
                    id: "09af".to_owned(),
                },
                source_address: "0x0".to_owned(),
                destination_address: "0x0".to_owned(),
                destination_chain: "solana".to_owned(),
                payload_hash: [0_u8; 32],
            }
        }
    }
}
