use anyhow::{anyhow, Context};
use borsh::BorshDeserialize;
use solana_program::pubkey::Pubkey;
use solana_sdk::account::{Account, ReadableAccount};

/// Extension trait for AccountInfo to check if the account is an initialized
/// PDA
pub trait CheckValidPDAInTests {
    /// Check if the account is an initialized PDA
    fn check_initialized_pda<T: solana_program::program_pack::Pack + BorshDeserialize>(
        &self,
        expected_program_id: &Pubkey,
    ) -> anyhow::Result<T>;

    /// Check if the account is an initialized PDA
    fn check_uninitialized_pda(&self) -> anyhow::Result<()>;
}

impl CheckValidPDAInTests for Account {
    fn check_initialized_pda<T: solana_program::program_pack::Pack + BorshDeserialize>(
        &self,
        expected_program_id: &Pubkey,
    ) -> anyhow::Result<T> {
        let has_lamports = self.lamports > 0;
        if !has_lamports {
            return Err(anyhow!("Account has no lamports"));
        }
        let has_correct_owner = &self.owner == expected_program_id;
        if !has_correct_owner {
            return Err(anyhow!("Account owner does not match expected program id"));
        }
        let data = self.data();

        // TODO use T::unpack(data) instead, but we need T: IsInitialized for that
        T::unpack_from_slice(data).context("Failed to deserialize account data")
    }

    fn check_uninitialized_pda(&self) -> anyhow::Result<()> {
        let data_is_empty = self.data.is_empty();
        if !data_is_empty {
            return Err(anyhow!("Account data is not empty"));
        }
        let owner_is_system = self.owner == solana_program::system_program::id();
        if !owner_is_system {
            return Err(anyhow!("Account owner is not the system program"));
        }
        Ok(())
    }
}
