use event_cpi_macros::{emit_cpi, event_cpi_accounts};
use program_utils::pda::{BytemuckedPda, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::bpf_loader_upgradeable::{self, UpgradeableLoaderState};
use solana_program::entrypoint::ProgramResult;
use solana_program::pubkey::Pubkey;

use super::Processor;
use crate::assert_valid_gateway_root_pda;
use crate::error::GatewayError;
use crate::events::OperatorshipTransferredEvent;
use crate::state::GatewayConfig;

impl Processor {
    /// Transfers gateway operatorship to a new address, authorized by
    /// either current operator or upgrade authority.
    ///
    /// Reference implementation:
    /// `https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/c290c7337fd447ecbb7426e52ac381175e33f602/contracts/gateway/AxelarAmplifierGateway.sol#L129-L133`
    ///
    /// # Errors
    ///
    /// Returns [`ProgramError`] if:
    /// * Account balance and expected ownership validation fails.
    /// * Required accounts are missing
    ///
    /// Returns [`GatewayError`] if:
    /// * Gateway root PDA is invalid
    /// * `ProgramData` account derivation fails
    /// * Loader state is invalid
    /// * Signer is neither operator nor upgrade authority
    /// * Data serialization fails
    pub fn process_transfer_operatorship(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();
        let gateway_root_pda = next_account_info(accounts_iter)?;
        let operator_or_upgrade_authority = next_account_info(accounts_iter)?;
        let programdata_account = next_account_info(accounts_iter)?;
        let new_operator = next_account_info(accounts_iter)?;
        event_cpi_accounts!(accounts_iter);

        // Check: Gateway Root PDA is initialized and valid.
        gateway_root_pda.check_initialized_pda_without_deserialization(&crate::ID)?;
        let mut gateway_data = gateway_root_pda.try_borrow_mut_data()?;
        let gateway_config = GatewayConfig::read_mut(&mut gateway_data)
            .ok_or(GatewayError::BytemuckDataLenInvalid)?;
        assert_valid_gateway_root_pda(gateway_config.bump, gateway_root_pda.key)?;

        // Check: programdata account derived correctly (it holds the upgrade authority
        // information)
        if *programdata_account.key
            != Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::id()).0
        {
            return Err(GatewayError::InvalidProgramDataDerivation.into());
        }

        // Check: the programdata state is valid
        let loader_state = programdata_account
            .data
            .borrow()
            .get(0..UpgradeableLoaderState::size_of_programdata_metadata())
            .ok_or(GatewayError::InvalidLoaderContent)
            .and_then(|bytes: &[u8]| {
                bincode::deserialize::<UpgradeableLoaderState>(bytes)
                    .map_err(|_err| GatewayError::InvalidLoaderContent)
            })?;

        let UpgradeableLoaderState::ProgramData {
            upgrade_authority_address,
            ..
        } = loader_state
        else {
            return Err(GatewayError::InvalidLoaderState.into());
        };

        // Check: ensure that the operator_or_upgrade_authority is a signer
        if !operator_or_upgrade_authority.is_signer {
            return Err(GatewayError::OperatorOrUpgradeAuthorityMustBeSigner.into());
        }

        // Check: the signer matches either the current operator or the upgrade
        // authority
        if !(gateway_config.operator == *operator_or_upgrade_authority.key
            || upgrade_authority_address == Some(*operator_or_upgrade_authority.key))
        {
            return Err(GatewayError::InvalidOperatorOrAuthorityAccount.into());
        }

        // Update the operator field
        gateway_config.operator = *new_operator.key;

        emit_cpi!(OperatorshipTransferredEvent {
            new_operator: *new_operator.key,
        });

        Ok(())
    }
}
