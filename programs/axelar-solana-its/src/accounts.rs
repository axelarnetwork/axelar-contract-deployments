use event_cpi_macros::event_cpi;
use program_utils::next_optional_account_info;
use program_utils::validate_mpl_token_metadata_key;
use program_utils::validate_rent_key;
use program_utils::validate_spl_associated_token_account_key;
use program_utils::validate_system_account_key;
use program_utils::validate_sysvar_instructions_key;
use solana_program::account_info::next_account_info;
use solana_program::account_info::AccountInfo;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use spl_associated_token_account::get_associated_token_address_with_program_id;
use spl_token_2022::check_spl_token_program_account;
use spl_token_2022::extension::StateWithExtensions;
use spl_token_2022::state::Account as TokenAccount;

/// Checks if an account is a valid Token account for the given mint and owner.
pub(crate) fn is_valid_token_account(
    account: &AccountInfo,
    token_program: &Pubkey,
    expected_mint: &Pubkey,
) -> bool {
    // Check account owner is the token program
    if account.owner != token_program {
        return false;
    }

    // Try to unpack as TokenAccount and verify mint/owner
    let account_data = account.data.borrow();
    if let Ok(token_account) = StateWithExtensions::<TokenAccount>::unpack(&account_data) {
        return token_account.base.mint == *expected_mint;
    }

    false
}

/// Account validation trait
pub(crate) trait Validate {
    fn validate(&self) -> Result<(), ProgramError>;
}

#[event_cpi]
pub(crate) struct ExecuteAccounts<'a> {
    pub(crate) payer: &'a AccountInfo<'a>,
    pub(crate) gateway_incoming_message: &'a AccountInfo<'a>,
    pub(crate) gateway_message_payload: &'a AccountInfo<'a>,
    pub(crate) gateway_signing: &'a AccountInfo<'a>,
    pub(crate) gateway_root: &'a AccountInfo<'a>,
    pub(crate) gateway_event_authority: &'a AccountInfo<'a>,
    pub(crate) gateway_program: &'a AccountInfo<'a>,
    pub(crate) system_program: &'a AccountInfo<'a>,
    pub(crate) its_root: &'a AccountInfo<'a>,
    pub(crate) token_manager: &'a AccountInfo<'a>,
    pub(crate) mint: &'a AccountInfo<'a>,
    pub(crate) token_manager_ata: &'a AccountInfo<'a>,
    pub(crate) token_program: &'a AccountInfo<'a>,
    pub(crate) ata_program: &'a AccountInfo<'a>,
    pub(crate) rent_sysvar: &'a AccountInfo<'a>,
    pub(crate) remaining_accounts: &'a [AccountInfo<'a>],
}

impl<'a> ExecuteAccounts<'a> {
    pub(crate) fn gateway_validation_accounts(&self) -> Vec<AccountInfo<'a>> {
        vec![
            self.payer.clone(),
            self.gateway_incoming_message.clone(),
            self.gateway_message_payload.clone(),
            self.gateway_signing.clone(),
            self.gateway_root.clone(),
            self.gateway_event_authority.clone(),
            self.gateway_program.clone(),
        ]
    }

    pub(crate) fn its_accounts(&self) -> Vec<AccountInfo<'a>> {
        let mut accounts = vec![
            self.system_program.clone(),
            self.its_root.clone(),
            self.token_manager.clone(),
            self.mint.clone(),
            self.token_manager_ata.clone(),
            self.token_program.clone(),
            self.ata_program.clone(),
            self.rent_sysvar.clone(),
            self.__event_cpi_authority_info.clone(),
            self.__event_cpi_program_account.clone(),
        ];

        accounts.extend(self.remaining_accounts.iter().cloned());

        accounts
    }
}

impl<'a> TryFrom<&'a [AccountInfo<'a>]> for ExecuteAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: &'a [AccountInfo<'a>]) -> Result<Self, Self::Error>
    where
        Self: Sized + Validate,
    {
        let accounts_iter = &mut value.iter();
        let converted = Self {
            payer: next_account_info(accounts_iter)?,
            gateway_incoming_message: next_account_info(accounts_iter)?,
            gateway_message_payload: next_account_info(accounts_iter)?,
            gateway_signing: next_account_info(accounts_iter)?,
            gateway_root: next_account_info(accounts_iter)?,
            gateway_event_authority: next_account_info(accounts_iter)?,
            gateway_program: next_account_info(accounts_iter)?,
            system_program: next_account_info(accounts_iter)?,
            its_root: next_account_info(accounts_iter)?,
            token_manager: next_account_info(accounts_iter)?,
            mint: next_account_info(accounts_iter)?,
            token_manager_ata: next_account_info(accounts_iter)?,
            token_program: next_account_info(accounts_iter)?,
            ata_program: next_account_info(accounts_iter)?,
            rent_sysvar: next_account_info(accounts_iter)?,
            __event_cpi_authority_info: next_account_info(accounts_iter)?,
            __event_cpi_program_account: next_account_info(accounts_iter)?,
            remaining_accounts: accounts_iter.as_slice(),
        };

        converted.validate()?;

        Ok(converted)
    }
}

impl Validate for ExecuteAccounts<'_> {
    fn validate(&self) -> Result<(), ProgramError> {
        validate_system_account_key(self.system_program.key)?;

        Ok(())
    }
}

#[event_cpi]
#[derive(Debug)]
pub(crate) struct CallContractAccounts<'a> {
    pub(crate) payer: &'a AccountInfo<'a>,
    pub(crate) gateway_root: &'a AccountInfo<'a>,
    pub(crate) gateway_event_authority: &'a AccountInfo<'a>,
    pub(crate) gateway_program: &'a AccountInfo<'a>,
    pub(crate) gas_service_root: &'a AccountInfo<'a>,
    pub(crate) gas_service_event_authority: &'a AccountInfo<'a>,
    pub(crate) _gas_service_program: &'a AccountInfo<'a>,
    pub(crate) system_program: &'a AccountInfo<'a>,
    pub(crate) its_root: &'a AccountInfo<'a>,
    pub(crate) call_contract_signing: &'a AccountInfo<'a>,
    pub(crate) program: &'a AccountInfo<'a>,
}

impl Validate for CallContractAccounts<'_> {
    fn validate(&self) -> Result<(), ProgramError> {
        validate_system_account_key(self.system_program.key)?;
        axelar_solana_gateway::check_program_account(*self.gateway_program.key)?;

        Ok(())
    }
}

impl<'a> TryFrom<&'a [AccountInfo<'a>]> for CallContractAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: &'a [AccountInfo<'a>]) -> Result<Self, Self::Error>
    where
        Self: Sized + Validate,
    {
        let accounts_iter = &mut value.iter();
        let converted = Self {
            payer: next_account_info(accounts_iter)?,
            gateway_root: next_account_info(accounts_iter)?,
            gateway_event_authority: next_account_info(accounts_iter)?,
            gateway_program: next_account_info(accounts_iter)?,
            gas_service_root: next_account_info(accounts_iter)?,
            gas_service_event_authority: next_account_info(accounts_iter)?,
            _gas_service_program: next_account_info(accounts_iter)?,
            system_program: next_account_info(accounts_iter)?,
            its_root: next_account_info(accounts_iter)?,
            call_contract_signing: next_account_info(accounts_iter)?,
            program: next_account_info(accounts_iter)?,
            __event_cpi_authority_info: next_account_info(accounts_iter)?,
            __event_cpi_program_account: next_account_info(accounts_iter)?,
        };

        converted.validate()?;

        Ok(converted)
    }
}

#[event_cpi]
#[derive(Debug)]
pub(crate) struct DeployCanonicalTokenAccounts<'a> {
    pub(crate) payer: &'a AccountInfo<'a>,
    pub(crate) token_metadata: &'a AccountInfo<'a>,
    pub(crate) system_program: &'a AccountInfo<'a>,
    pub(crate) its_root: &'a AccountInfo<'a>,
    pub(crate) token_manager: &'a AccountInfo<'a>,
    pub(crate) mint: &'a AccountInfo<'a>,
    pub(crate) token_manager_ata: &'a AccountInfo<'a>,
    pub(crate) token_program: &'a AccountInfo<'a>,
    pub(crate) ata_program: &'a AccountInfo<'a>,
    pub(crate) rent_sysvar: &'a AccountInfo<'a>,
}

impl Validate for DeployCanonicalTokenAccounts<'_> {
    fn validate(&self) -> Result<(), ProgramError> {
        Ok(())
    }
}

impl<'a> TryFrom<&'a [AccountInfo<'a>]> for DeployCanonicalTokenAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: &'a [AccountInfo<'a>]) -> Result<Self, Self::Error>
    where
        Self: Sized + Validate,
    {
        let accounts_iter = &mut value.iter();
        let converted = Self {
            payer: next_account_info(accounts_iter)?,
            token_metadata: next_account_info(accounts_iter)?,
            system_program: next_account_info(accounts_iter)?,
            its_root: next_account_info(accounts_iter)?,
            token_manager: next_account_info(accounts_iter)?,
            mint: next_account_info(accounts_iter)?,
            token_manager_ata: next_account_info(accounts_iter)?,
            token_program: next_account_info(accounts_iter)?,
            ata_program: next_account_info(accounts_iter)?,
            rent_sysvar: next_account_info(accounts_iter)?,
            __event_cpi_authority_info: next_account_info(accounts_iter)?,
            __event_cpi_program_account: next_account_info(accounts_iter)?,
        };

        converted.validate()?;

        Ok(converted)
    }
}

impl<'a> TryFrom<DeployCanonicalTokenAccounts<'a>> for DeployTokenManagerAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: DeployCanonicalTokenAccounts<'a>) -> Result<Self, Self::Error> {
        let converted = Self {
            payer: value.payer,
            system_program: value.system_program,
            its_root: value.its_root,
            token_manager: value.token_manager,
            mint: value.mint,
            token_manager_ata: value.token_manager_ata,
            token_program: value.token_program,
            ata_program: value.ata_program,
            rent_sysvar: value.rent_sysvar,
            operator: None,
            operator_roles: None,
            __event_cpi_authority_info: value.__event_cpi_authority_info,
            __event_cpi_program_account: value.__event_cpi_program_account,
        };

        converted.validate()?;

        Ok(converted)
    }
}

#[event_cpi]
#[derive(Debug)]
pub(crate) struct DeployCustomTokenAccounts<'a> {
    pub(crate) payer: &'a AccountInfo<'a>,
    pub(crate) deployer: &'a AccountInfo<'a>,
    pub(crate) system_program: &'a AccountInfo<'a>,
    pub(crate) its_root: &'a AccountInfo<'a>,
    pub(crate) token_manager: &'a AccountInfo<'a>,
    pub(crate) mint: &'a AccountInfo<'a>,
    pub(crate) token_manager_ata: &'a AccountInfo<'a>,
    pub(crate) token_program: &'a AccountInfo<'a>,
    pub(crate) ata_program: &'a AccountInfo<'a>,
    pub(crate) rent_sysvar: &'a AccountInfo<'a>,
    pub(crate) operator: Option<&'a AccountInfo<'a>>,
    pub(crate) operator_roles: Option<&'a AccountInfo<'a>>,
}

impl Validate for DeployCustomTokenAccounts<'_> {
    fn validate(&self) -> Result<(), ProgramError> {
        if !self.deployer.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(())
    }
}

impl<'a> TryFrom<&'a [AccountInfo<'a>]> for DeployCustomTokenAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: &'a [AccountInfo<'a>]) -> Result<Self, Self::Error>
    where
        Self: Sized + Validate,
    {
        let accounts_iter = &mut value.iter();
        let converted = Self {
            payer: next_account_info(accounts_iter)?,
            deployer: next_account_info(accounts_iter)?,
            system_program: next_account_info(accounts_iter)?,
            its_root: next_account_info(accounts_iter)?,
            token_manager: next_account_info(accounts_iter)?,
            mint: next_account_info(accounts_iter)?,
            token_manager_ata: next_account_info(accounts_iter)?,
            token_program: next_account_info(accounts_iter)?,
            ata_program: next_account_info(accounts_iter)?,
            rent_sysvar: next_account_info(accounts_iter)?,
            operator: next_optional_account_info(accounts_iter, &crate::ID)?,
            operator_roles: next_optional_account_info(accounts_iter, &crate::ID)?,
            __event_cpi_authority_info: next_account_info(accounts_iter)?,
            __event_cpi_program_account: next_account_info(accounts_iter)?,
        };

        converted.validate()?;

        Ok(converted)
    }
}

impl<'a> TryFrom<DeployCustomTokenAccounts<'a>> for DeployTokenManagerAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: DeployCustomTokenAccounts<'a>) -> Result<Self, Self::Error> {
        let converted = Self {
            payer: value.payer,
            system_program: value.system_program,
            its_root: value.its_root,
            token_manager: value.token_manager,
            mint: value.mint,
            token_manager_ata: value.token_manager_ata,
            token_program: value.token_program,
            ata_program: value.ata_program,
            rent_sysvar: value.rent_sysvar,
            operator: value.operator,
            operator_roles: value.operator_roles,
            __event_cpi_authority_info: value.__event_cpi_authority_info,
            __event_cpi_program_account: value.__event_cpi_program_account,
        };

        converted.validate()?;

        Ok(converted)
    }
}

#[event_cpi]
#[derive(Debug)]
pub(crate) struct TakeTokenAccounts<'a> {
    pub(crate) payer: &'a AccountInfo<'a>,
    pub(crate) authority: &'a AccountInfo<'a>,
    pub(crate) its_root: &'a AccountInfo<'a>,
    pub(crate) source_ata: &'a AccountInfo<'a>,
    pub(crate) mint: &'a AccountInfo<'a>,
    pub(crate) token_manager: &'a AccountInfo<'a>,
    pub(crate) token_manager_ata: &'a AccountInfo<'a>,
    pub(crate) token_program: &'a AccountInfo<'a>,
    pub(crate) gateway_root: &'a AccountInfo<'a>,
    pub(crate) gateway_event_authority: &'a AccountInfo<'a>,
    pub(crate) gateway_program: &'a AccountInfo<'a>,
    pub(crate) gas_service_root: &'a AccountInfo<'a>,
    pub(crate) gas_service_event_authority: &'a AccountInfo<'a>,
    pub(crate) gas_service_program: &'a AccountInfo<'a>,
    pub(crate) system_program: &'a AccountInfo<'a>,
    pub(crate) call_contract_signing: &'a AccountInfo<'a>,
    pub(crate) its_program: &'a AccountInfo<'a>,
}

impl Validate for TakeTokenAccounts<'_> {
    fn validate(&self) -> Result<(), ProgramError> {
        validate_system_account_key(self.system_program.key)?;
        spl_token_2022::check_spl_token_program_account(self.token_program.key)?;

        if !self.payer.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if !self.authority.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if self.mint.owner != self.token_program.key {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(())
    }
}

impl<'a> TryFrom<&'a [AccountInfo<'a>]> for TakeTokenAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: &'a [AccountInfo<'a>]) -> Result<Self, Self::Error>
    where
        Self: Sized + Validate,
    {
        let accounts_iter = &mut value.iter();

        let converted = Self {
            payer: next_account_info(accounts_iter)?,
            authority: next_account_info(accounts_iter)?,
            its_root: next_account_info(accounts_iter)?,
            source_ata: next_account_info(accounts_iter)?,
            mint: next_account_info(accounts_iter)?,
            token_manager: next_account_info(accounts_iter)?,
            token_manager_ata: next_account_info(accounts_iter)?,
            token_program: next_account_info(accounts_iter)?,
            gateway_root: next_account_info(accounts_iter)?,
            gateway_event_authority: next_account_info(accounts_iter)?,
            gateway_program: next_account_info(accounts_iter)?,
            gas_service_root: next_account_info(accounts_iter)?,
            gas_service_event_authority: next_account_info(accounts_iter)?,
            gas_service_program: next_account_info(accounts_iter)?,
            system_program: next_account_info(accounts_iter)?,
            call_contract_signing: next_account_info(accounts_iter)?,
            its_program: next_account_info(accounts_iter)?,
            __event_cpi_authority_info: next_account_info(accounts_iter)?,
            __event_cpi_program_account: next_account_info(accounts_iter)?,
        };

        converted.validate()?;

        Ok(converted)
    }
}

impl<'a> TryFrom<TakeTokenAccounts<'a>> for CallContractAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: TakeTokenAccounts<'a>) -> Result<Self, Self::Error> {
        let converted = Self {
            payer: value.payer,
            gateway_root: value.gateway_root,
            gateway_event_authority: value.gateway_event_authority,
            gateway_program: value.gateway_program,
            gas_service_root: value.gas_service_root,
            gas_service_event_authority: value.gas_service_event_authority,
            _gas_service_program: value.gas_service_program,
            call_contract_signing: value.call_contract_signing,
            program: value.its_program,
            system_program: value.system_program,
            its_root: value.its_root,
            __event_cpi_authority_info: value.__event_cpi_authority_info,
            __event_cpi_program_account: value.__event_cpi_program_account,
        };

        converted.validate()?;

        Ok(converted)
    }
}

#[event_cpi]
#[derive(Debug)]
pub(crate) struct GiveTokenAccounts<'a> {
    pub(crate) payer: &'a AccountInfo<'a>,
    pub(crate) system_program: &'a AccountInfo<'a>,
    pub(crate) its_root: &'a AccountInfo<'a>,
    pub(crate) gateway_message_payload: &'a AccountInfo<'a>,
    pub(crate) token_manager: &'a AccountInfo<'a>,
    pub(crate) mint: &'a AccountInfo<'a>,
    pub(crate) token_manager_ata: &'a AccountInfo<'a>,
    pub(crate) token_program: &'a AccountInfo<'a>,
    pub(crate) ata_program: &'a AccountInfo<'a>,
    pub(crate) rent_sysvar: &'a AccountInfo<'a>,
    pub(crate) destination: &'a AccountInfo<'a>,
    pub(crate) destination_ata: &'a AccountInfo<'a>,
    pub(crate) interchain_transfer_execute: Option<&'a AccountInfo<'a>>,
    pub(crate) remaining_accounts: &'a [AccountInfo<'a>],
}

impl Validate for GiveTokenAccounts<'_> {
    fn validate(&self) -> Result<(), ProgramError> {
        validate_system_account_key(self.system_program.key)?;
        validate_spl_associated_token_account_key(self.ata_program.key)?;
        validate_rent_key(self.rent_sysvar.key)?;
        spl_token_2022::check_spl_token_program_account(self.token_program.key)?;

        if self.mint.owner != self.token_program.key {
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(())
    }
}

impl<'a> TryFrom<ExecuteAccounts<'a>> for GiveTokenAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: ExecuteAccounts<'a>) -> Result<Self, Self::Error> {
        let remaining_accounts_iter = &mut value.remaining_accounts.iter();
        let mut converted = Self {
            payer: value.payer,
            system_program: value.system_program,
            its_root: value.its_root,
            gateway_message_payload: value.gateway_message_payload,
            token_manager: value.token_manager,
            mint: value.mint,
            token_manager_ata: value.token_manager_ata,
            token_program: value.token_program,
            ata_program: value.ata_program,
            rent_sysvar: value.rent_sysvar,
            destination: next_account_info(remaining_accounts_iter)?,
            destination_ata: next_account_info(remaining_accounts_iter)?,
            interchain_transfer_execute: next_optional_account_info(
                remaining_accounts_iter,
                &crate::ID,
            )?,
            __event_cpi_authority_info: value.__event_cpi_authority_info,
            __event_cpi_program_account: value.__event_cpi_program_account,
            remaining_accounts: remaining_accounts_iter.as_slice(),
        };

        if is_valid_token_account(
            converted.destination,
            converted.token_program.key,
            converted.mint.key,
        ) {
            converted.destination_ata = converted.destination;
        } else {
            crate::create_associated_token_account_idempotent(
                converted.payer,
                converted.mint,
                converted.destination_ata,
                converted.destination,
                converted.system_program,
                converted.token_program,
            )?;
        }

        converted.validate()?;

        Ok(converted)
    }
}

pub(crate) struct AxelarInterchainTokenExecutableAccounts<'a> {
    pub(crate) gateway_message_payload: &'a AccountInfo<'a>,
    pub(crate) token_program: &'a AccountInfo<'a>,
    pub(crate) mint: &'a AccountInfo<'a>,
    pub(crate) destination_program_ata: &'a AccountInfo<'a>,
    pub(crate) interchain_transfer_execute: &'a AccountInfo<'a>,
    pub(crate) destination_program_accounts: &'a [AccountInfo<'a>],
}

impl Validate for AxelarInterchainTokenExecutableAccounts<'_> {
    fn validate(&self) -> Result<(), ProgramError> {
        Ok(())
    }
}

impl<'a> TryFrom<GiveTokenAccounts<'a>> for AxelarInterchainTokenExecutableAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: GiveTokenAccounts<'a>) -> Result<Self, Self::Error> {
        let converted = Self {
            gateway_message_payload: value.gateway_message_payload,
            token_program: value.token_program,
            mint: value.mint,
            destination_program_ata: value.destination_ata,
            destination_program_accounts: value.remaining_accounts,
            interchain_transfer_execute: value
                .interchain_transfer_execute
                .ok_or(ProgramError::NotEnoughAccountKeys)?,
        };

        converted.validate()?;

        Ok(converted)
    }
}

pub(crate) struct FlowTrackingAccounts<'a> {
    pub(crate) system_program: &'a AccountInfo<'a>,
    pub(crate) payer: &'a AccountInfo<'a>,
    pub(crate) token_manager: &'a AccountInfo<'a>,
}

impl<'a> From<&TakeTokenAccounts<'a>> for FlowTrackingAccounts<'a> {
    fn from(value: &TakeTokenAccounts<'a>) -> Self {
        Self {
            system_program: value.system_program,
            payer: value.payer,
            token_manager: value.token_manager,
        }
    }
}

impl<'a> From<&GiveTokenAccounts<'a>> for FlowTrackingAccounts<'a> {
    fn from(value: &GiveTokenAccounts<'a>) -> Self {
        Self {
            system_program: value.system_program,
            payer: value.payer,
            token_manager: value.token_manager,
        }
    }
}

#[event_cpi]
#[derive(Debug)]
pub(crate) struct DeployTokenManagerAccounts<'a> {
    pub(crate) payer: &'a AccountInfo<'a>,
    pub(crate) system_program: &'a AccountInfo<'a>,
    pub(crate) its_root: &'a AccountInfo<'a>,
    pub(crate) token_manager: &'a AccountInfo<'a>,
    pub(crate) mint: &'a AccountInfo<'a>,
    pub(crate) token_manager_ata: &'a AccountInfo<'a>,
    pub(crate) token_program: &'a AccountInfo<'a>,
    pub(crate) ata_program: &'a AccountInfo<'a>,
    pub(crate) rent_sysvar: &'a AccountInfo<'a>,
    pub(crate) operator: Option<&'a AccountInfo<'a>>,
    pub(crate) operator_roles: Option<&'a AccountInfo<'a>>,
}

impl Validate for DeployTokenManagerAccounts<'_> {
    fn validate(&self) -> Result<(), ProgramError> {
        validate_system_account_key(self.system_program.key)?;
        check_spl_token_program_account(self.token_program.key)?;
        validate_spl_associated_token_account_key(self.ata_program.key)?;
        validate_rent_key(self.rent_sysvar.key)?;

        if !self.payer.is_signer {
            return Err(ProgramError::MissingRequiredSignature);
        }

        if self.token_program.key != self.mint.owner {
            msg!("Mint and program account mismatch");
            return Err(ProgramError::IncorrectProgramId);
        }

        if &get_associated_token_address_with_program_id(
            self.token_manager.key,
            self.mint.key,
            self.token_program.key,
        ) != self.token_manager_ata.key
        {
            msg!("Wrong ata account key");
            return Err(ProgramError::InvalidAccountData);
        }

        Ok(())
    }
}

impl<'a> TryFrom<ExecuteAccounts<'a>> for DeployTokenManagerAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: ExecuteAccounts<'a>) -> Result<Self, Self::Error> {
        let accounts_iter = &mut value.remaining_accounts.iter();

        Ok(Self {
            payer: value.payer,
            system_program: value.system_program,
            its_root: value.its_root,
            token_manager: value.token_manager,
            mint: value.mint,
            token_manager_ata: value.token_manager_ata,
            token_program: value.token_program,
            ata_program: value.ata_program,
            rent_sysvar: value.rent_sysvar,
            operator: next_optional_account_info(accounts_iter, &crate::ID)?,
            operator_roles: next_optional_account_info(accounts_iter, &crate::ID)?,
            __event_cpi_authority_info: value.__event_cpi_authority_info,
            __event_cpi_program_account: value.__event_cpi_program_account,
        })
    }
}

#[event_cpi]
#[derive(Debug)]
pub(crate) struct DeployInterchainTokenAccounts<'a> {
    pub(crate) payer: &'a AccountInfo<'a>,
    pub(crate) deployer: &'a AccountInfo<'a>,
    pub(crate) system_program: &'a AccountInfo<'a>,
    pub(crate) its_root: &'a AccountInfo<'a>,
    pub(crate) token_manager: &'a AccountInfo<'a>,
    pub(crate) mint: &'a AccountInfo<'a>,
    pub(crate) token_manager_ata: &'a AccountInfo<'a>,
    pub(crate) token_program: &'a AccountInfo<'a>,
    pub(crate) ata_program: &'a AccountInfo<'a>,
    pub(crate) rent_sysvar: &'a AccountInfo<'a>,
    pub(crate) sysvar_instructions: &'a AccountInfo<'a>,
    pub(crate) mpl_token_metadata_program: &'a AccountInfo<'a>,
    pub(crate) mpl_token_metadata: &'a AccountInfo<'a>,
    pub(crate) deployer_ata: &'a AccountInfo<'a>,
    pub(crate) minter: Option<&'a AccountInfo<'a>>,
    pub(crate) minter_roles: Option<&'a AccountInfo<'a>>,
}

impl Validate for DeployInterchainTokenAccounts<'_> {
    fn validate(&self) -> Result<(), ProgramError> {
        validate_system_account_key(self.system_program.key)?;
        validate_spl_associated_token_account_key(self.ata_program.key)?;
        validate_rent_key(self.rent_sysvar.key)?;
        validate_sysvar_instructions_key(self.sysvar_instructions.key)?;
        validate_mpl_token_metadata_key(self.mpl_token_metadata_program.key)?;
        spl_token_2022::check_program_account(self.token_program.key)?;

        if !self.payer.is_signer {
            msg!("Payer should be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        if !self.deployer.is_signer {
            msg!("Deployer should be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        // If it's a cross-chain message, payer_ata is not set (i.e., is set to program id)
        if *self.deployer_ata.key != crate::id() {
            crate::assert_valid_ata(
                self.deployer_ata.key,
                self.token_program.key,
                self.mint.key,
                self.deployer.key,
            )?;
        }

        crate::assert_valid_ata(
            self.token_manager_ata.key,
            self.token_program.key,
            self.mint.key,
            self.token_manager.key,
        )?;

        Ok(())
    }
}

impl<'a> TryFrom<&'a [AccountInfo<'a>]> for DeployInterchainTokenAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: &'a [AccountInfo<'a>]) -> Result<Self, Self::Error>
    where
        Self: Sized + Validate,
    {
        let accounts_iter = &mut value.iter();
        let converted = Self {
            payer: next_account_info(accounts_iter)?,
            deployer: next_account_info(accounts_iter)?,
            system_program: next_account_info(accounts_iter)?,
            its_root: next_account_info(accounts_iter)?,
            token_manager: next_account_info(accounts_iter)?,
            mint: next_account_info(accounts_iter)?,
            token_manager_ata: next_account_info(accounts_iter)?,
            token_program: next_account_info(accounts_iter)?,
            ata_program: next_account_info(accounts_iter)?,
            rent_sysvar: next_account_info(accounts_iter)?,
            sysvar_instructions: next_account_info(accounts_iter)?,
            mpl_token_metadata_program: next_account_info(accounts_iter)?,
            mpl_token_metadata: next_account_info(accounts_iter)?,
            deployer_ata: next_account_info(accounts_iter)?,
            minter: next_optional_account_info(accounts_iter, &crate::ID)?,
            minter_roles: next_optional_account_info(accounts_iter, &crate::ID)?,
            __event_cpi_authority_info: next_account_info(accounts_iter)?,
            __event_cpi_program_account: next_account_info(accounts_iter)?,
        };

        converted.validate()?;

        Ok(converted)
    }
}

impl<'a> From<DeployInterchainTokenAccounts<'a>> for DeployTokenManagerAccounts<'a> {
    fn from(value: DeployInterchainTokenAccounts<'a>) -> Self {
        Self {
            payer: value.payer,
            system_program: value.system_program,
            its_root: value.its_root,
            token_manager: value.token_manager,
            mint: value.mint,
            token_manager_ata: value.token_manager_ata,
            token_program: value.token_program,
            ata_program: value.ata_program,
            rent_sysvar: value.rent_sysvar,
            operator: value.minter,
            operator_roles: value.minter_roles,
            __event_cpi_authority_info: value.__event_cpi_authority_info,
            __event_cpi_program_account: value.__event_cpi_program_account,
        }
    }
}

impl<'a> TryFrom<ExecuteAccounts<'a>> for DeployInterchainTokenAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: ExecuteAccounts<'a>) -> Result<Self, Self::Error> {
        let accounts_iter = &mut value.remaining_accounts.iter();

        Ok(Self {
            payer: value.payer,
            deployer: value.payer,
            system_program: value.system_program,
            its_root: value.its_root,
            token_manager: value.token_manager,
            mint: value.mint,
            token_manager_ata: value.token_manager_ata,
            token_program: value.token_program,
            ata_program: value.ata_program,
            rent_sysvar: value.rent_sysvar,
            sysvar_instructions: next_account_info(accounts_iter)?,
            mpl_token_metadata_program: next_account_info(accounts_iter)?,
            mpl_token_metadata: next_account_info(accounts_iter)?,
            deployer_ata: next_account_info(accounts_iter)?,
            minter: next_optional_account_info(accounts_iter, &crate::ID)?,
            minter_roles: next_optional_account_info(accounts_iter, &crate::ID)?,
            __event_cpi_authority_info: value.__event_cpi_authority_info,
            __event_cpi_program_account: value.__event_cpi_program_account,
        })
    }
}

#[event_cpi]
#[derive(Debug)]
pub(crate) struct DeployRemoteInterchainTokenAccounts<'a> {
    pub(crate) payer: &'a AccountInfo<'a>,
    pub(crate) deployer: &'a AccountInfo<'a>,
    pub(crate) mint: &'a AccountInfo<'a>,
    pub(crate) mpl_token_metadata: &'a AccountInfo<'a>,
    pub(crate) its_root: &'a AccountInfo<'a>,
    pub(crate) token_manager: &'a AccountInfo<'a>,
    pub(crate) gateway_root: &'a AccountInfo<'a>,
    pub(crate) gateway_event_authority: &'a AccountInfo<'a>,
    pub(crate) gateway_program: &'a AccountInfo<'a>,
    pub(crate) gas_service_root: &'a AccountInfo<'a>,
    pub(crate) gas_service_event_authority: &'a AccountInfo<'a>,
    pub(crate) gas_service_program: &'a AccountInfo<'a>,
    pub(crate) system_program: &'a AccountInfo<'a>,
    pub(crate) call_contract_signing: &'a AccountInfo<'a>,
    pub(crate) its_program: &'a AccountInfo<'a>,
}

impl Validate for DeployRemoteInterchainTokenAccounts<'_> {
    fn validate(&self) -> Result<(), ProgramError> {
        if !self.payer.is_signer {
            msg!("Payer should be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        if !self.deployer.is_signer {
            msg!("Deployer should be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }
        Ok(())
    }
}

impl<'a> TryFrom<&'a [AccountInfo<'a>]> for DeployRemoteInterchainTokenAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: &'a [AccountInfo<'a>]) -> Result<Self, Self::Error>
    where
        Self: Sized + Validate,
    {
        let accounts_iter = &mut value.iter();

        let converted = Self {
            payer: next_account_info(accounts_iter)?,
            deployer: next_account_info(accounts_iter)?,
            mint: next_account_info(accounts_iter)?,
            mpl_token_metadata: next_account_info(accounts_iter)?,
            its_root: next_account_info(accounts_iter)?,
            token_manager: next_account_info(accounts_iter)?,
            gateway_root: next_account_info(accounts_iter)?,
            gateway_event_authority: next_account_info(accounts_iter)?,
            gateway_program: next_account_info(accounts_iter)?,
            gas_service_root: next_account_info(accounts_iter)?,
            gas_service_event_authority: next_account_info(accounts_iter)?,
            gas_service_program: next_account_info(accounts_iter)?,
            system_program: next_account_info(accounts_iter)?,
            call_contract_signing: next_account_info(accounts_iter)?,
            its_program: next_account_info(accounts_iter)?,
            __event_cpi_authority_info: next_account_info(accounts_iter)?,
            __event_cpi_program_account: next_account_info(accounts_iter)?,
        };

        converted.validate()?;

        Ok(converted)
    }
}

#[event_cpi]
#[derive(Debug, Clone)]
pub(crate) struct DeployRemoteInterchainTokenWithMinterAccounts<'a> {
    pub(crate) payer: &'a AccountInfo<'a>,
    pub(crate) deployer: &'a AccountInfo<'a>,
    pub(crate) mint: &'a AccountInfo<'a>,
    pub(crate) mpl_token_metadata: &'a AccountInfo<'a>,
    pub(crate) its_root: &'a AccountInfo<'a>,
    pub(crate) token_manager: &'a AccountInfo<'a>,
    pub(crate) minter: &'a AccountInfo<'a>,
    pub(crate) deployment_approval: &'a AccountInfo<'a>,
    pub(crate) minter_roles: &'a AccountInfo<'a>,
    pub(crate) gateway_root: &'a AccountInfo<'a>,
    pub(crate) gateway_event_authority: &'a AccountInfo<'a>,
    pub(crate) gateway_program: &'a AccountInfo<'a>,
    pub(crate) gas_service_root: &'a AccountInfo<'a>,
    pub(crate) gas_service_event_authority: &'a AccountInfo<'a>,
    pub(crate) gas_service_program: &'a AccountInfo<'a>,
    pub(crate) system_program: &'a AccountInfo<'a>,
    pub(crate) call_contract_signing: &'a AccountInfo<'a>,
    pub(crate) its_program: &'a AccountInfo<'a>,
}

impl Validate for DeployRemoteInterchainTokenWithMinterAccounts<'_> {
    fn validate(&self) -> Result<(), ProgramError> {
        if !self.payer.is_signer {
            msg!("Payer should be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        if !self.deployer.is_signer {
            msg!("Deployer should be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(())
    }
}

impl<'a> TryFrom<&'a [AccountInfo<'a>]> for DeployRemoteInterchainTokenWithMinterAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: &'a [AccountInfo<'a>]) -> Result<Self, Self::Error>
    where
        Self: Sized + Validate,
    {
        let accounts_iter = &mut value.iter();

        let converted = Self {
            payer: next_account_info(accounts_iter)?,
            deployer: next_account_info(accounts_iter)?,
            mint: next_account_info(accounts_iter)?,
            mpl_token_metadata: next_account_info(accounts_iter)?,
            its_root: next_account_info(accounts_iter)?,
            token_manager: next_account_info(accounts_iter)?,
            minter: next_account_info(accounts_iter)?,
            deployment_approval: next_account_info(accounts_iter)?,
            minter_roles: next_account_info(accounts_iter)?,
            gateway_root: next_account_info(accounts_iter)?,
            gateway_event_authority: next_account_info(accounts_iter)?,
            gateway_program: next_account_info(accounts_iter)?,
            gas_service_root: next_account_info(accounts_iter)?,
            gas_service_event_authority: next_account_info(accounts_iter)?,
            gas_service_program: next_account_info(accounts_iter)?,
            system_program: next_account_info(accounts_iter)?,
            call_contract_signing: next_account_info(accounts_iter)?,
            its_program: next_account_info(accounts_iter)?,
            __event_cpi_authority_info: next_account_info(accounts_iter)?,
            __event_cpi_program_account: next_account_info(accounts_iter)?,
        };

        converted.validate()?;

        Ok(converted)
    }
}

#[event_cpi]
#[derive(Debug)]
pub(crate) struct DeployRemoteCanonicalInterchainTokenAccounts<'a> {
    pub(crate) payer: &'a AccountInfo<'a>,
    pub(crate) mint: &'a AccountInfo<'a>,
    pub(crate) mpl_token_metadata: &'a AccountInfo<'a>,
    pub(crate) its_root: &'a AccountInfo<'a>,
    pub(crate) token_manager: &'a AccountInfo<'a>,
    pub(crate) gateway_root: &'a AccountInfo<'a>,
    pub(crate) gateway_event_authority: &'a AccountInfo<'a>,
    pub(crate) gateway_program: &'a AccountInfo<'a>,
    pub(crate) gas_service_config: &'a AccountInfo<'a>,
    pub(crate) gas_service_event_authority: &'a AccountInfo<'a>,
    pub(crate) gas_service_program: &'a AccountInfo<'a>,
    pub(crate) system_program: &'a AccountInfo<'a>,
    pub(crate) call_contract_signing: &'a AccountInfo<'a>,
    pub(crate) its_program: &'a AccountInfo<'a>,
}

impl Validate for DeployRemoteCanonicalInterchainTokenAccounts<'_> {
    fn validate(&self) -> Result<(), ProgramError> {
        if !self.payer.is_signer {
            msg!("Payer should be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(())
    }
}

impl<'a> TryFrom<&'a [AccountInfo<'a>]> for DeployRemoteCanonicalInterchainTokenAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: &'a [AccountInfo<'a>]) -> Result<Self, Self::Error>
    where
        Self: Sized + Validate,
    {
        let accounts_iter = &mut value.iter();

        let converted = Self {
            payer: next_account_info(accounts_iter)?,
            mint: next_account_info(accounts_iter)?,
            mpl_token_metadata: next_account_info(accounts_iter)?,
            its_root: next_account_info(accounts_iter)?,
            token_manager: next_account_info(accounts_iter)?,
            gateway_root: next_account_info(accounts_iter)?,
            gateway_event_authority: next_account_info(accounts_iter)?,
            gateway_program: next_account_info(accounts_iter)?,
            gas_service_config: next_account_info(accounts_iter)?,
            gas_service_event_authority: next_account_info(accounts_iter)?,
            gas_service_program: next_account_info(accounts_iter)?,
            system_program: next_account_info(accounts_iter)?,
            call_contract_signing: next_account_info(accounts_iter)?,
            its_program: next_account_info(accounts_iter)?,
            __event_cpi_authority_info: next_account_info(accounts_iter)?,
            __event_cpi_program_account: next_account_info(accounts_iter)?,
        };

        converted.validate()?;

        Ok(converted)
    }
}

impl<'a> TryFrom<DeployRemoteCanonicalInterchainTokenAccounts<'a>> for CallContractAccounts<'a> {
    type Error = ProgramError;

    fn try_from(
        value: DeployRemoteCanonicalInterchainTokenAccounts<'a>,
    ) -> Result<Self, Self::Error> {
        let converted = Self {
            payer: value.payer,
            gateway_root: value.gateway_root,
            gateway_event_authority: value.gateway_event_authority,
            gateway_program: value.gateway_program,
            gas_service_root: value.gas_service_config,
            gas_service_event_authority: value.gas_service_event_authority,
            _gas_service_program: value.gas_service_program,
            system_program: value.system_program,
            its_root: value.its_root,
            call_contract_signing: value.call_contract_signing,
            program: value.its_program,
            __event_cpi_authority_info: value.__event_cpi_authority_info,
            __event_cpi_program_account: value.__event_cpi_program_account,
        };

        converted.validate()?;

        Ok(converted)
    }
}

pub(crate) type CommonDeployRemoteInterchainTokenAccounts<'a> =
    DeployRemoteCanonicalInterchainTokenAccounts<'a>;

impl<'a> TryFrom<DeployRemoteInterchainTokenAccounts<'a>>
    for CommonDeployRemoteInterchainTokenAccounts<'a>
{
    type Error = ProgramError;

    fn try_from(value: DeployRemoteInterchainTokenAccounts<'a>) -> Result<Self, Self::Error> {
        Ok(Self {
            payer: value.payer,
            mint: value.mint,
            mpl_token_metadata: value.mpl_token_metadata,
            its_root: value.its_root,
            token_manager: value.token_manager,
            gateway_root: value.gateway_root,
            gateway_event_authority: value.gateway_event_authority,
            gateway_program: value.gateway_program,
            gas_service_config: value.gas_service_root,
            gas_service_event_authority: value.gas_service_event_authority,
            gas_service_program: value.gas_service_program,
            system_program: value.system_program,
            call_contract_signing: value.call_contract_signing,
            its_program: value.its_program,
            __event_cpi_authority_info: value.__event_cpi_authority_info,
            __event_cpi_program_account: value.__event_cpi_program_account,
        })
    }
}

impl<'a> TryFrom<DeployRemoteInterchainTokenWithMinterAccounts<'a>>
    for CommonDeployRemoteInterchainTokenAccounts<'a>
{
    type Error = ProgramError;

    fn try_from(
        value: DeployRemoteInterchainTokenWithMinterAccounts<'a>,
    ) -> Result<Self, Self::Error> {
        Ok(Self {
            payer: value.payer,
            mint: value.mint,
            mpl_token_metadata: value.mpl_token_metadata,
            its_root: value.its_root,
            token_manager: value.token_manager,
            gateway_root: value.gateway_root,
            gateway_event_authority: value.gateway_event_authority,
            gateway_program: value.gateway_program,
            gas_service_config: value.gas_service_root,
            gas_service_event_authority: value.gas_service_event_authority,
            gas_service_program: value.gas_service_program,
            system_program: value.system_program,
            call_contract_signing: value.call_contract_signing,
            its_program: value.its_program,
            __event_cpi_authority_info: value.__event_cpi_authority_info,
            __event_cpi_program_account: value.__event_cpi_program_account,
        })
    }
}

#[event_cpi]
#[derive(Debug)]
pub(crate) struct LinkTokenAccounts<'a> {
    pub(crate) payer: &'a AccountInfo<'a>,
    pub(crate) deployer: &'a AccountInfo<'a>,
    pub(crate) its_root: &'a AccountInfo<'a>,
    pub(crate) token_manager: &'a AccountInfo<'a>,
    pub(crate) gateway_root: &'a AccountInfo<'a>,
    pub(crate) gateway_event_authority: &'a AccountInfo<'a>,
    pub(crate) gateway_program: &'a AccountInfo<'a>,
    pub(crate) gas_service_root: &'a AccountInfo<'a>,
    pub(crate) gas_service_event_authority: &'a AccountInfo<'a>,
    pub(crate) gas_service_program: &'a AccountInfo<'a>,
    pub(crate) system_program: &'a AccountInfo<'a>,
    pub(crate) call_contract_signing: &'a AccountInfo<'a>,
    pub(crate) its_program: &'a AccountInfo<'a>,
}

impl Validate for LinkTokenAccounts<'_> {
    fn validate(&self) -> Result<(), ProgramError> {
        if !self.payer.is_signer {
            msg!("Payer should be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        if !self.deployer.is_signer {
            msg!("Deployer should be signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(())
    }
}

impl<'a> TryFrom<&'a [AccountInfo<'a>]> for LinkTokenAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: &'a [AccountInfo<'a>]) -> Result<Self, Self::Error>
    where
        Self: Sized + Validate,
    {
        let accounts_iter = &mut value.iter();

        let converted = Self {
            payer: next_account_info(accounts_iter)?,
            deployer: next_account_info(accounts_iter)?,
            its_root: next_account_info(accounts_iter)?,
            token_manager: next_account_info(accounts_iter)?,
            gateway_root: next_account_info(accounts_iter)?,
            gateway_event_authority: next_account_info(accounts_iter)?,
            gateway_program: next_account_info(accounts_iter)?,
            gas_service_root: next_account_info(accounts_iter)?,
            gas_service_event_authority: next_account_info(accounts_iter)?,
            gas_service_program: next_account_info(accounts_iter)?,
            system_program: next_account_info(accounts_iter)?,
            call_contract_signing: next_account_info(accounts_iter)?,
            its_program: next_account_info(accounts_iter)?,
            __event_cpi_authority_info: next_account_info(accounts_iter)?,
            __event_cpi_program_account: next_account_info(accounts_iter)?,
        };

        converted.validate()?;

        Ok(converted)
    }
}

impl<'a> TryFrom<LinkTokenAccounts<'a>> for CallContractAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: LinkTokenAccounts<'a>) -> Result<Self, Self::Error> {
        let converted = Self {
            payer: value.payer,
            gateway_root: value.gateway_root,
            gateway_event_authority: value.gateway_event_authority,
            gateway_program: value.gateway_program,
            gas_service_root: value.gas_service_root,
            gas_service_event_authority: value.gas_service_event_authority,
            _gas_service_program: value.gas_service_program,
            system_program: value.system_program,
            its_root: value.its_root,
            call_contract_signing: value.call_contract_signing,
            program: value.its_program,
            __event_cpi_authority_info: value.__event_cpi_authority_info,
            __event_cpi_program_account: value.__event_cpi_program_account,
        };

        converted.validate()?;

        Ok(converted)
    }
}

#[event_cpi]
#[derive(Debug)]
pub(crate) struct RegisterTokenMetadataAccounts<'a> {
    pub(crate) payer: &'a AccountInfo<'a>,
    pub(crate) mint: &'a AccountInfo<'a>,
    pub(crate) its_root: &'a AccountInfo<'a>,
    pub(crate) gateway_root: &'a AccountInfo<'a>,
    pub(crate) gateway_event_authority: &'a AccountInfo<'a>,
    pub(crate) gateway_program: &'a AccountInfo<'a>,
    pub(crate) gas_service_root: &'a AccountInfo<'a>,
    pub(crate) gas_service_event_authority: &'a AccountInfo<'a>,
    pub(crate) gas_service_program: &'a AccountInfo<'a>,
    pub(crate) system_program: &'a AccountInfo<'a>,
    pub(crate) call_contract_signing: &'a AccountInfo<'a>,
    pub(crate) its_program: &'a AccountInfo<'a>,
}

impl Validate for RegisterTokenMetadataAccounts<'_> {
    fn validate(&self) -> Result<(), ProgramError> {
        if !self.payer.is_signer {
            msg!("Payer should be a signer");
            return Err(ProgramError::MissingRequiredSignature);
        }

        Ok(())
    }
}

impl<'a> TryFrom<&'a [AccountInfo<'a>]> for RegisterTokenMetadataAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: &'a [AccountInfo<'a>]) -> Result<Self, Self::Error>
    where
        Self: Sized + Validate,
    {
        let accounts_iter = &mut value.iter();

        let converted = Self {
            payer: next_account_info(accounts_iter)?,
            mint: next_account_info(accounts_iter)?,
            its_root: next_account_info(accounts_iter)?,
            gateway_root: next_account_info(accounts_iter)?,
            gateway_event_authority: next_account_info(accounts_iter)?,
            gateway_program: next_account_info(accounts_iter)?,
            gas_service_root: next_account_info(accounts_iter)?,
            gas_service_event_authority: next_account_info(accounts_iter)?,
            gas_service_program: next_account_info(accounts_iter)?,
            system_program: next_account_info(accounts_iter)?,
            call_contract_signing: next_account_info(accounts_iter)?,
            its_program: next_account_info(accounts_iter)?,
            __event_cpi_authority_info: next_account_info(accounts_iter)?,
            __event_cpi_program_account: next_account_info(accounts_iter)?,
        };

        converted.validate()?;

        Ok(converted)
    }
}

impl<'a> TryFrom<RegisterTokenMetadataAccounts<'a>> for CallContractAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: RegisterTokenMetadataAccounts<'a>) -> Result<Self, Self::Error> {
        let converted = Self {
            payer: value.payer,
            gateway_root: value.gateway_root,
            gateway_event_authority: value.gateway_event_authority,
            gateway_program: value.gateway_program,
            gas_service_root: value.gas_service_root,
            gas_service_event_authority: value.gas_service_event_authority,
            _gas_service_program: value.gas_service_program,
            system_program: value.system_program,
            its_root: value.its_root,
            call_contract_signing: value.call_contract_signing,
            program: value.its_program,
            __event_cpi_authority_info: value.__event_cpi_authority_info,
            __event_cpi_program_account: value.__event_cpi_program_account,
        };

        converted.validate()?;

        Ok(converted)
    }
}

#[event_cpi]
#[derive(Debug)]
pub(crate) struct SetTrustedChainAccounts<'a> {
    pub(crate) payer: &'a AccountInfo<'a>,
    pub(crate) authority: &'a AccountInfo<'a>,
    pub(crate) authority_roles: &'a AccountInfo<'a>,
    pub(crate) program_data: &'a AccountInfo<'a>,
    pub(crate) its_root: &'a AccountInfo<'a>,
    pub(crate) system_program: &'a AccountInfo<'a>,
}

impl<'a> Validate for SetTrustedChainAccounts<'a> {
    fn validate(&self) -> Result<(), ProgramError> {
        validate_system_account_key(self.system_program.key)?;

        Ok(())
    }
}

impl<'a> TryFrom<&'a [AccountInfo<'a>]> for SetTrustedChainAccounts<'a> {
    type Error = ProgramError;

    fn try_from(value: &'a [AccountInfo<'a>]) -> Result<Self, Self::Error> {
        let accounts_iter = &mut value.iter();
        let converted = Self {
            payer: next_account_info(accounts_iter)?,
            authority: next_account_info(accounts_iter)?,
            authority_roles: next_account_info(accounts_iter)?,
            program_data: next_account_info(accounts_iter)?,
            its_root: next_account_info(accounts_iter)?,
            system_program: next_account_info(accounts_iter)?,
            __event_cpi_authority_info: next_account_info(accounts_iter)?,
            __event_cpi_program_account: next_account_info(accounts_iter)?,
        };

        converted.validate()?;

        Ok(converted)
    }
}

pub(crate) type RemoveTrustedChainAccounts<'a> = SetTrustedChainAccounts<'a>;
