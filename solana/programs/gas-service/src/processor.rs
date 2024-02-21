//! Program processor

use auth_weighted::types::u256::U256;
use borsh::BorshDeserialize;
use gateway::types::PubkeyWrapper;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::entrypoint::ProgramResult;
use solana_program::program::invoke_signed;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::rent::Rent;
use solana_program::system_program;
use solana_program::sysvar::Sysvar;

use crate::accounts::GasServiceRootPDA;
use crate::error::GasServiceError;
use crate::events::emit_refunded_event;
use crate::instruction::GasServiceInstruction;
use crate::solana_program::system_instruction;
use crate::{events, get_gas_service_root_pda, TxHash};

/// Program handler.
pub struct Processor;

impl Processor {
    /// Processes an instruction.
    pub fn process_instruction(
        program_id: &Pubkey,
        accounts: &[AccountInfo],
        input: &[u8],
    ) -> ProgramResult {
        crate::check_program_account(program_id)?;
        let Ok(instruction) = borsh::de::from_slice(input) else {
            return Err(GasServiceError::InvalidInstruction)?;
        };

        match instruction {
            GasServiceInstruction::Initialize => Self::init_root_pda(accounts)?,
            GasServiceInstruction::PayNativeGasForContractCall {
                destination_chain,
                destination_address,
                payload,
                fees,
                refund_address,
            } => Self::pay_native_gas_for_contract_call(
                accounts,
                destination_chain,
                destination_address,
                payload,
                fees,
                refund_address,
            )?,
            GasServiceInstruction::PayNativeGasForContractCallWithToken {
                destination_chain,
                destination_address,
                payload,
                symbol,
                amount,
                fees,
                refund_address,
            } => Self::pay_native_gas_for_contract_call_with_token(
                accounts,
                destination_chain,
                destination_address,
                payload,
                symbol,
                amount,
                fees,
                refund_address,
            )?,
            GasServiceInstruction::PayNativeGasForExpressCall {
                destination_chain,
                destination_address,
                payload,
                fees,
                refund_address,
            } => Self::pay_native_gas_for_express_call(
                accounts,
                destination_chain,
                destination_address,
                payload,
                fees,
                refund_address,
            )?,
            GasServiceInstruction::PayNativeGasForExpressCallWithToken {
                destination_chain,
                destination_address,
                payload,
                symbol,
                amount,
                fees,
                refund_address,
            } => Self::pay_native_gas_for_express_call_with_token(
                accounts,
                destination_chain,
                destination_address,
                payload,
                symbol,
                amount,
                fees,
                refund_address,
            )?,
            GasServiceInstruction::AddNativeGas {
                tx_hash,
                log_index,
                fees,
                refund_address,
            } => Self::add_native_gas(accounts, tx_hash, log_index, fees, refund_address)?,
            GasServiceInstruction::AddNativeExpressGas {
                tx_hash,
                log_index,
                fees,
                refund_address,
            } => Self::add_native_express_gas(accounts, tx_hash, log_index, fees, refund_address)?,
            GasServiceInstruction::CollectFees { amount } => Self::collect_fees(accounts, amount)?,
            GasServiceInstruction::Refund {
                tx_hash,
                log_index,
                fees,
            } => Self::refund(accounts, tx_hash, log_index, fees)?,
        };

        Ok(())
    }

    /// Initialize Root PDA account, used to store authority and native
    /// currency.
    fn init_root_pda(accounts: &[AccountInfo]) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();

        // Payer.
        let initializer_account = next_account_info(accounts_iter)?;
        let new_account_root_pda = next_account_info(accounts_iter)?;

        if new_account_root_pda.lamports() > 0 {
            return Err(GasServiceError::RootPDAAccountAlreadyInitialized.into());
        }

        let serialized_data = borsh::to_vec(&GasServiceRootPDA::new(PubkeyWrapper::from(
            *initializer_account.key,
        )))?;

        let space = serialized_data.len();
        let rent_sysvar = Rent::get()?;
        let rent = rent_sysvar.minimum_balance(space);

        assert!(initializer_account.is_signer);
        assert!(initializer_account.is_writable);
        // // Note that `new_account_root_pda` is not a signer yet.
        // // This program will sign for it via `invoke_signed`.
        assert!(!new_account_root_pda.is_signer);
        assert!(new_account_root_pda.is_writable);
        assert_eq!(new_account_root_pda.owner, &system_program::ID);

        // Check: Gateway Config account uses the canonical bump.
        let (root_pda, bump) = get_gas_service_root_pda();
        if *new_account_root_pda.key != root_pda {
            return Err(GasServiceError::InvalidGasServiceRootPDAAccount.into());
        }

        invoke_signed(
            &system_instruction::create_account(
                initializer_account.key,
                new_account_root_pda.key,
                rent,
                space
                    .try_into()
                    .map_err(|_| ProgramError::ArithmeticOverflow)?,
                &crate::ID,
            ),
            &[initializer_account.clone(), new_account_root_pda.clone()],
            &[&[&[bump]]],
        )?;
        let mut account_data = new_account_root_pda.try_borrow_mut_data()?;
        account_data[..space].copy_from_slice(&serialized_data);

        Ok(())
    }

    /// Pay for gas using native currency for a contract call on a destination
    /// chain.
    ///
    /// [destination_chain] The target chain where the contract call will be
    /// made.
    ///
    /// [destination_address] The target address on the destination chain.
    ///
    /// [payload] Data payload for the contract call.
    ///
    /// [fees] The amount of SOL to pay for gas.
    ///
    /// [refund_address] The address where refunds, if any, should be sent.
    fn pay_native_gas_for_contract_call(
        accounts: &[AccountInfo],
        destination_chain: String,
        destination_address: Vec<u8>,
        payload: Vec<u8>,
        fees: u64,
        refund_address: PubkeyWrapper,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();

        // Payer.
        let sender_account = next_account_info(accounts_iter)?;
        let gas_service_root_pda_account = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;

        // Check: System Program Account.
        if !system_program::check_id(system_account.key) {
            return Err(GasServiceError::InvalidSystemAccount.into());
        }

        // Check: Root PDA Account.
        let (addr, bumb) = get_gas_service_root_pda();
        if gas_service_root_pda_account.key != &addr {
            return Err(GasServiceError::InvalidGasServiceRootPDAAccount.into());
        }

        // Check: Sender Account muttable.
        if !sender_account.is_writable {
            return Err(GasServiceError::SenderAccountIsNotWrittable.into());
        }

        // Check: Gas Service Account is signer.
        if !sender_account.is_signer {
            return Err(GasServiceError::SenderAccountIsNotSigner.into());
        }

        invoke_signed(
            &system_instruction::transfer(
                sender_account.key,
                gas_service_root_pda_account.key,
                fees,
            ),
            &[sender_account.clone(), gas_service_root_pda_account.clone()],
            &[&[&[bumb]]],
        )?;

        events::emit_native_gas_paid_for_contract_call_event(
            (*sender_account.key).into(),
            destination_chain,
            destination_address,
            payload,
            fees,
            (*refund_address).into(),
        )?;

        Ok(())
    }

    /// Pay for gas using native currency for a contract call with tokens.
    ///
    /// [destination_chain] The target chain where the contract call will be
    /// made.
    ///
    /// [destination_address] The target address on the destination chain.
    ///
    /// [payload] Data payload for the contract call.
    ///
    /// [symbol] The symbol of the token used to pay for gas.
    ///
    /// [amount] The amount of tokens.
    ///
    /// [fees] The amount of SOL to pay for gas.
    ///
    /// [refund_address] The address where refunds, if any, should be sent.
    #[allow(clippy::too_many_arguments)]
    fn pay_native_gas_for_contract_call_with_token(
        accounts: &[AccountInfo],
        destination_chain: String,
        destination_address: Vec<u8>,
        payload: Vec<u8>,
        symbol: Vec<u8>,
        amount: U256,
        fees: u64,
        refund_address: PubkeyWrapper,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();

        // Payer.
        let sender_account = next_account_info(accounts_iter)?;
        let gas_service_root_pda_account = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;

        // Check: System Program Account.
        if !system_program::check_id(system_account.key) {
            return Err(GasServiceError::InvalidSystemAccount.into());
        }

        // Check: Root PDA Account.
        let (addr, bumb) = get_gas_service_root_pda();
        if gas_service_root_pda_account.key != &addr {
            return Err(GasServiceError::InvalidGasServiceRootPDAAccount.into());
        }

        // Check: Sender Account muttable.
        if !sender_account.is_writable {
            return Err(GasServiceError::SenderAccountIsNotWrittable.into());
        }

        // Check: Gas Service Account is signer.
        if !sender_account.is_signer {
            return Err(GasServiceError::SenderAccountIsNotSigner.into());
        }

        invoke_signed(
            &system_instruction::transfer(
                sender_account.key,
                gas_service_root_pda_account.key,
                fees,
            ),
            &[sender_account.clone(), gas_service_root_pda_account.clone()],
            &[&[&[bumb]]],
        )?;

        events::emit_native_gas_paid_for_contract_call_with_token_event(
            (*sender_account.key).into(),
            destination_chain,
            destination_address,
            payload,
            symbol,
            amount,
            fees,
            (*refund_address).into(),
        )?;

        Ok(())
    }

    /// Pay for gas using native currency for an express contract call on a
    /// destination chain. This function is called on the source chain before
    /// calling the gateway to execute a remote contract.
    ///
    /// [destination_chain] The target chain where the contract call will be
    /// made.
    ///
    /// [destination_address] The target address on the destination chain.
    ///
    /// [payload] Data payload for the contract call.
    ///
    /// [fees] The amount of SOL to pay for gas.
    ///
    /// [refund_address] The address where refunds, if any, should be sent.
    fn pay_native_gas_for_express_call(
        accounts: &[AccountInfo],
        destination_chain: String,
        destination_address: Vec<u8>,
        payload: Vec<u8>,
        fees: u64,
        refund_address: PubkeyWrapper,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();

        // Payer.
        let sender_account = next_account_info(accounts_iter)?;
        let gas_service_root_pda_account = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;

        // Check: System Program Account.
        if !system_program::check_id(system_account.key) {
            return Err(GasServiceError::InvalidSystemAccount.into());
        }

        // Check: Root PDA Account.
        let (addr, bumb) = get_gas_service_root_pda();
        if gas_service_root_pda_account.key != &addr {
            return Err(GasServiceError::InvalidGasServiceRootPDAAccount.into());
        }

        // Check: Sender Account muttable.
        if !sender_account.is_writable {
            return Err(GasServiceError::SenderAccountIsNotWrittable.into());
        }

        // Check: Gas Service Account is signer.
        if !sender_account.is_signer {
            return Err(GasServiceError::SenderAccountIsNotSigner.into());
        }

        invoke_signed(
            &system_instruction::transfer(
                sender_account.key,
                gas_service_root_pda_account.key,
                fees,
            ),
            &[sender_account.clone(), gas_service_root_pda_account.clone()],
            &[&[&[bumb]]],
        )?;

        events::emit_native_gas_paid_for_express_call_event(
            (*sender_account.key).into(),
            destination_chain,
            destination_address,
            payload,
            fees,
            (*refund_address).into(),
        )?;

        Ok(())
    }

    /// Pay for gas using native currency for an express contract call on a
    /// destination chain with tokens. This function is called on the source
    /// chain
    ///
    /// [destination_chain] The target chain where the contract call will be
    /// made.
    ///
    /// [destination_address] The target address on the destination chain.
    ///
    /// [payload] Data payload for the contract call.
    ///
    /// [symbol] The symbol of the token.
    ///
    /// [amount] The amount of tokens.
    ///
    /// [fees] The amount of SOL to pay for gas.
    ///
    /// [refund_address] The address where refunds, if any, should be sent.
    #[allow(clippy::too_many_arguments)]
    fn pay_native_gas_for_express_call_with_token(
        accounts: &[AccountInfo],
        destination_chain: String,
        destination_address: Vec<u8>,
        payload: Vec<u8>,
        symbol: Vec<u8>,
        amount: U256,
        fees: u64,
        refund_address: PubkeyWrapper,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();

        // Payer.
        let sender_account = next_account_info(accounts_iter)?;
        let gas_service_root_pda_account = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;

        // Check: System Program Account.
        if !system_program::check_id(system_account.key) {
            return Err(GasServiceError::InvalidSystemAccount.into());
        }

        // Check: Root PDA Account.
        let (addr, bumb) = get_gas_service_root_pda();
        if gas_service_root_pda_account.key != &addr {
            return Err(GasServiceError::InvalidGasServiceRootPDAAccount.into());
        }

        // Check: Sender Account muttable.
        if !sender_account.is_writable {
            return Err(GasServiceError::SenderAccountIsNotWrittable.into());
        }

        // Check: Gas Service Account is signer.
        if !sender_account.is_signer {
            return Err(GasServiceError::SenderAccountIsNotSigner.into());
        }

        invoke_signed(
            &system_instruction::transfer(
                sender_account.key,
                gas_service_root_pda_account.key,
                fees,
            ),
            &[sender_account.clone(), gas_service_root_pda_account.clone()],
            &[&[&[bumb]]],
        )?;

        events::emit_native_gas_paid_for_express_call_with_token_event(
            (*sender_account.key).into(),
            destination_chain,
            destination_address,
            payload,
            symbol,
            amount,
            fees,
            (*refund_address).into(),
        )?;

        Ok(())
    }

    /// Add additional gas payment using native currency after initiating a
    /// cross-chain call. This function can be called on the source chain after
    /// calling the gateway to execute a remote contract.
    ///
    /// [tx_hash] The hash of the transaction on the destination chain.
    ///
    /// [log_index] The log index of the event.
    ///
    /// [fees] The amount of SOL to pay for gas.
    ///
    /// [refund_address] The address where refunds, if any, should be sent.
    fn add_native_gas(
        accounts: &[AccountInfo],
        tx_hash: TxHash,
        log_index: u64,
        fees: u64,
        refund_address: PubkeyWrapper,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();

        // Payer.
        let sender_account = next_account_info(accounts_iter)?;
        let gas_service_root_pda_account = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;

        // Check: System Program Account.
        if !system_program::check_id(system_account.key) {
            return Err(GasServiceError::InvalidSystemAccount.into());
        }

        // Check: Root PDA Account.
        let (addr, bumb) = get_gas_service_root_pda();
        if gas_service_root_pda_account.key != &addr {
            return Err(GasServiceError::InvalidGasServiceRootPDAAccount.into());
        }

        // Check: Sender Account muttable.
        if !sender_account.is_writable {
            return Err(GasServiceError::SenderAccountIsNotWrittable.into());
        }

        // Check: Gas Service Account is signer.
        if !sender_account.is_signer {
            return Err(GasServiceError::SenderAccountIsNotSigner.into());
        }

        invoke_signed(
            &system_instruction::transfer(
                sender_account.key,
                gas_service_root_pda_account.key,
                fees,
            ),
            &[sender_account.clone(), gas_service_root_pda_account.clone()],
            &[&[&[bumb]]],
        )?;

        events::emit_native_gas_added_event(tx_hash, log_index, fees, refund_address)?;

        Ok(())
    }

    /// Add additional gas payment using native currency after initiating an
    /// express cross-chain call. This function can be called on the source
    /// chain after calling the gateway to express execute a remote
    /// contract.
    ///
    /// [tx_hash] The hash of the transaction on the destination chain.
    ///
    /// [log_index] The log index of the event.
    ///
    /// [fees] The amount of SOL to pay for gas.
    ///
    /// [refund_address] The address where refunds, if any, should be sent.
    fn add_native_express_gas(
        accounts: &[AccountInfo],
        tx_hash: TxHash,
        log_index: u64,
        fees: u64,
        refund_address: PubkeyWrapper,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();

        // Payer.
        let sender_account = next_account_info(accounts_iter)?;
        let gas_service_root_pda_account = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;

        // Check: System Program Account.
        if !system_program::check_id(system_account.key) {
            return Err(GasServiceError::InvalidSystemAccount.into());
        }

        // Check: Root PDA Account.
        let (addr, bumb) = get_gas_service_root_pda();
        if gas_service_root_pda_account.key != &addr {
            return Err(GasServiceError::InvalidGasServiceRootPDAAccount.into());
        }

        // Check: Sender Account muttable.
        if !sender_account.is_writable {
            return Err(GasServiceError::SenderAccountIsNotWrittable.into());
        }

        // Check: Gas Service Account is signer.
        if !sender_account.is_signer {
            return Err(GasServiceError::SenderAccountIsNotSigner.into());
        }

        invoke_signed(
            &system_instruction::transfer(
                sender_account.key,
                gas_service_root_pda_account.key,
                fees,
            ),
            &[sender_account.clone(), gas_service_root_pda_account.clone()],
            &[&[&[bumb]]],
        )?;

        events::emit_native_express_gas_added_event(tx_hash, log_index, fees, refund_address)?;

        Ok(())
    }

    /// Allows the gas collector to collect accumulated fees from the contract.
    ///
    /// This instruction would success only if the authority key used to
    /// initialize the contract signs the transaction.
    ///
    /// [amount] The amount of SOL to transfer.
    fn collect_fees(accounts: &[AccountInfo], amount: u64) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();

        // Payer/ Initializer / Authority.
        let sender_account = next_account_info(accounts_iter)?;
        let gas_service_root_pda_account = next_account_info(accounts_iter)?;
        let receiver_account = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;

        // Check: System Program Account.
        if !system_program::check_id(system_account.key) {
            return Err(GasServiceError::InvalidSystemAccount.into());
        }

        // Check: Root PDA Account.
        let (addr, _bumb) = get_gas_service_root_pda();
        if gas_service_root_pda_account.key != &addr {
            return Err(GasServiceError::InvalidGasServiceRootPDAAccount.into());
        }

        // Check: Sender Account muttable.
        if !sender_account.is_writable {
            return Err(GasServiceError::SenderAccountIsNotWrittable.into());
        }

        // Check: Gas Service Account is signer.
        if !sender_account.is_signer {
            return Err(GasServiceError::SenderAccountIsNotSigner.into());
        }

        // Check: Receiver Account muttable.
        if !receiver_account.is_writable {
            return Err(GasServiceError::ReceiverAccountIsNotWrittable.into());
        }

        // Check: Sender Account is the expected authority.
        let gas_service_root_pda_deserialized =
            GasServiceRootPDA::try_from_slice(&gas_service_root_pda_account.data.borrow())?;

        if !gas_service_root_pda_deserialized.check_authority((*sender_account.key).into()) {
            return Err(GasServiceError::SenderAccountIsNotExpectedAuthority.into());
        }

        // Check: The requested amount bigger than tresure account saldo minus required
        // rent exempt value.
        let space = gas_service_root_pda_account.data.borrow().len();
        let rent_sysvar = Rent::get()?;
        let rent = rent_sysvar.minimum_balance(space);

        if (**gas_service_root_pda_account.try_borrow_lamports()? - rent) < amount {
            return Err(GasServiceError::InsufficientFundsForTransaction.into());
        }

        // Transfer to collector.
        **gas_service_root_pda_account.try_borrow_mut_lamports()? -= amount;
        **receiver_account.try_borrow_mut_lamports()? += amount;

        Ok(())
    }

    fn refund(
        accounts: &[AccountInfo],
        tx_hash: TxHash,
        log_index: u64,
        fees: u64,
    ) -> Result<(), ProgramError> {
        let accounts_iter = &mut accounts.iter();

        // Payer/ Initializer / Authority.
        let sender_account = next_account_info(accounts_iter)?;
        let gas_service_root_pda_account = next_account_info(accounts_iter)?;
        let receiver_account = next_account_info(accounts_iter)?;
        let system_account = next_account_info(accounts_iter)?;

        // Check: System Program Account.
        if !system_program::check_id(system_account.key) {
            return Err(GasServiceError::InvalidSystemAccount.into());
        }

        // Check: Root PDA Account.
        let (addr, _bumb) = get_gas_service_root_pda();
        if gas_service_root_pda_account.key != &addr {
            return Err(GasServiceError::InvalidGasServiceRootPDAAccount.into());
        }

        // Check: Sender Account muttable.
        if !sender_account.is_writable {
            return Err(GasServiceError::SenderAccountIsNotWrittable.into());
        }

        // Check: Gas Service Account is signer.
        if !sender_account.is_signer {
            return Err(GasServiceError::SenderAccountIsNotSigner.into());
        }

        // Check: Receiver Account muttable.
        if !receiver_account.is_writable {
            return Err(GasServiceError::ReceiverAccountIsNotWrittable.into());
        }

        // Check: Sender Account is the expected authority.
        let gas_service_root_pda_deserialized =
            GasServiceRootPDA::try_from_slice(&gas_service_root_pda_account.data.borrow())?;

        if !gas_service_root_pda_deserialized.check_authority((*sender_account.key).into()) {
            return Err(GasServiceError::SenderAccountIsNotExpectedAuthority.into());
        }

        // Check: The requested amount bigger than tresure account saldo minus required
        // rent exempt value.
        let space = gas_service_root_pda_account.data.borrow().len();
        let rent_sysvar = Rent::get()?;
        let rent = rent_sysvar.minimum_balance(space);

        if (**gas_service_root_pda_account.try_borrow_lamports()? - rent) < fees {
            return Err(GasServiceError::InsufficientFundsForTransaction.into());
        }

        // Transfer to collector.
        **gas_service_root_pda_account.try_borrow_mut_lamports()? -= fees;
        **receiver_account.try_borrow_mut_lamports()? += fees;

        emit_refunded_event(tx_hash, log_index, fees, (*receiver_account.key).into())?;

        Ok(())
    }
}
