use program_utils::ValidPDA;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::bpf_loader_upgradeable::{self, UpgradeableLoaderState};
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::program_pack::Pack;
use solana_program::pubkey::Pubkey;

use super::Processor;
use crate::events::GatewayEvent;
use crate::state::GatewayConfig;

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

        // Check: Config account uses the canonical bump.
        // Unpack Gateway configuration data.
        let mut gateway_config =
            gateway_root_pda.check_initialized_pda::<GatewayConfig>(program_id)?;

        // Check: programdata account derived correctly (it holds the upgrade authority
        // information)
        if *programdata_account.key
            != Pubkey::find_program_address(&[program_id.as_ref()], &bpf_loader_upgradeable::id()).0
        {
            msg!("invalid programdata account provided");
            return Err(ProgramError::InvalidArgument);
        }

        // Check: the programda state is valid
        let loader_state = bincode::deserialize::<UpgradeableLoaderState>(
            &programdata_account.data.borrow()
                [0..UpgradeableLoaderState::size_of_programdata_metadata()],
        )
        .map_err(|err| {
            msg!("upradeable loader state deserialisatoin error {:?}", err);
            ProgramError::InvalidAccountData
        })?;
        let UpgradeableLoaderState::ProgramData {
            slot: _,
            upgrade_authority_address,
        } = loader_state
        else {
            msg!("upgradeable loader state is not programdata state");
            return Err(ProgramError::InvalidAccountData);
        };

        // Check: ensure that the operator_or_upgrade_authority is a signer
        if !operator_or_upgrade_authority.is_signer {
            msg!("Operator or owner account must be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        // Check: the signer matches either the current operator or the upgrade
        // authority
        if !(gateway_config.operator == *operator_or_upgrade_authority.key
            || upgrade_authority_address.map_or(false, |x| x == *operator_or_upgrade_authority.key))
        {
            msg!(
                "Operator or owner account is not the factual operator or the owner of the Gateway"
            );
            return Err(ProgramError::InvalidArgument);
        }

        // Update the opreatorship field
        gateway_config.operator = *new_operator.key;
        msg!(
            "Operatorship transferred to: {:?}",
            new_operator.key.to_string()
        );

        // Store the gateway data back to the account.
        let mut data = gateway_root_pda.try_borrow_mut_data()?;
        gateway_config.pack_into_slice(&mut data);

        // Emit an event
        GatewayEvent::OperatorshipTransferred(crate::events::OperatorshipTransferred {
            operator: new_operator.key.to_bytes(),
        })
        .emit()?;

        Ok(())
    }
}
