use program_utils::{BytemuckedPda, ValidPDA};
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::bpf_loader_upgradeable::{self, UpgradeableLoaderState};
use solana_program::entrypoint::ProgramResult;
use solana_program::log::sol_log_data;
use solana_program::pubkey::Pubkey;

use super::event_utils::{read_array, EventParseError};
use super::Processor;
use crate::error::GatewayError;
use crate::state::GatewayConfig;
use crate::{assert_valid_gateway_root_pda, event_prefixes};

impl Processor {
    /// Transfer operatorship of the Gateway to a new address.
    /// reference implementation: https://github.com/axelarnetwork/axelar-gmp-sdk-solidity/blob/c290c7337fd447ecbb7426e52ac381175e33f602/contracts/gateway/AxelarAmplifierGateway.sol#L129-L133
    pub fn process_transfer_operatorship(
        program_id: &Pubkey,
        accounts: &[AccountInfo<'_>],
    ) -> ProgramResult {
        let mut accounts_iter = accounts.iter();
        let gateway_root_pda = next_account_info(&mut accounts_iter)?;
        let operator_or_upgrade_authority = next_account_info(&mut accounts_iter)?;
        let programdata_account = next_account_info(&mut accounts_iter)?;
        let new_operator = next_account_info(&mut accounts_iter)?;

        // Check: Gateway Root PDA is initialized.
        gateway_root_pda.check_initialized_pda_without_deserialization(program_id)?;
        let mut data = gateway_root_pda.try_borrow_mut_data()?;
        let gateway_config =
            GatewayConfig::read_mut(&mut data).ok_or(GatewayError::BytemuckDataLenInvalid)?;
        assert_valid_gateway_root_pda(gateway_config.bump, gateway_root_pda.key)?;

        // Check: programdata account derived correctly (it holds the upgrade authority
        // information)
        if *programdata_account.key
            != Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::id()).0
        {
            return Err(GatewayError::InvalidProgramDataDerivation.into());
        }

        // Check: the programda state is valid
        let loader_state = bincode::deserialize::<UpgradeableLoaderState>(
            &programdata_account.data.borrow()
                [0..UpgradeableLoaderState::size_of_programdata_metadata()],
        )
        .map_err(|_err| GatewayError::InvalidLoaderContent)?;
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
            || upgrade_authority_address.map_or(false, |x| x == *operator_or_upgrade_authority.key))
        {
            return Err(GatewayError::InvalidOperatorOrAuthorityAccount.into());
        }

        // Update the opreatorship field
        gateway_config.operator = *new_operator.key;

        // Emit an event
        sol_log_data(&[
            event_prefixes::OPERATORSHIP_TRANSFERRED,
            &new_operator.key.to_bytes(),
        ]);

        Ok(())
    }
}

/// Event for the `TransferOperatorship` instruction
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct OperatorshipTransferredEvent {
    /// The pubkey of the new operator
    pub new_operator: Pubkey,
}

impl OperatorshipTransferredEvent {
    /// Constructs a new `OperatorshipTransferredEvent` with the provided data slice.
    pub fn new<I>(mut data: I) -> Result<Self, EventParseError>
    where
        I: Iterator<Item = Vec<u8>>,
    {
        // Read known-size elements
        let new_operator = data
            .next()
            .ok_or(EventParseError::MissingData("new_operator"))?;
        let new_operator = read_array("new_operator", &new_operator)?;

        Ok(Self {
            new_operator: Pubkey::new_from_array(new_operator),
        })
    }
}
