//! Program state accounts.

use bimap::BiBTreeMap;
use borsh::io::Error;
use borsh::io::ErrorKind::{Interrupted, InvalidData};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::account_info::AccountInfo;
use solana_program::program::invoke;
use solana_program::program_error::ProgramError;
use solana_program::rent::Rent;
use solana_program::system_instruction;
use solana_program::sysvar::Sysvar;

use crate::error::AuthWeightedError;
use crate::types::u256::U256;

type OperatorsHash = [u8; 32];

/// Wrapper type for implementing borsh traits for `BiBTreeMap`.
#[derive(Clone, Debug, Default, PartialEq)]
struct Map(bimap::BiBTreeMap<OperatorsHash, U256>);

impl BorshSerialize for Map {
    #[inline]
    fn serialize<W: std::io::prelude::Write>(&self, writer: &mut W) -> borsh::io::Result<()> {
        u16::try_from(self.0.len())
            .map_err(|_| InvalidData)?
            .serialize(writer)?;
        for (hash, epoch) in self.0.iter() {
            epoch.to_le_bytes().serialize(writer)?;
            hash.serialize(writer)?;
        }
        Ok(())
    }
}

impl BorshDeserialize for Map {
    #[inline]
    fn deserialize_reader<R: std::io::prelude::Read>(reader: &mut R) -> borsh::io::Result<Self> {
        let mut bimap = BiBTreeMap::new();
        let mut pos = 0;
        let mut epoch_buffer = [0u8; 32];
        let mut hash_buffer = [0u8; 32];
        let len = u16::deserialize_reader(reader)?;
        while pos < len {
            if reader.read(&mut epoch_buffer)? == 0 {
                return Err(Error::new(Interrupted, "Unexpected length of input"));
            };
            let epoch = U256::from_le_bytes(epoch_buffer);
            if reader.read(&mut hash_buffer)? == 0 {
                return Err(Error::new(Interrupted, "Unexpected length of input"));
            };
            bimap.insert_no_overwrite(hash_buffer, epoch).map_err(|_| {
                Error::new(
                    InvalidData,
                    "Can't insert duplicated values in the biject map",
                )
            })?;
            pos += 1;
        }
        Ok(Map(bimap))
    }
}

/// [AuthWeightedStateAccount]; where program keeps operators and epoch.
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct AuthWeightedStateAccount {
    map: Map,
}

impl AuthWeightedStateAccount {
    /// Creates a new value with an empty collection.

    /// Updates the epoch and operators in the state.
    pub fn update_epoch_and_operators(
        &mut self,
        operators_hash: OperatorsHash,
    ) -> Result<(), AuthWeightedError> {
        // We add one so this epoch number matches with the value returned from
        // `Self::current_epoch`
        let epoch = self.map.0.len() as u128 + 1;
        self.map
            .0
            .insert_no_overwrite(operators_hash, epoch.into())
            .map_err(|_| AuthWeightedError::DuplicateOperators)
    }

    /// Returns the epoch associated with the given operator hash
    pub fn epoch_for_operator_hash(&self, operators_hash: &OperatorsHash) -> Option<&U256> {
        self.map.0.get_by_left(operators_hash)
    }

    /// Returns the operator hash associated with the given epoch
    pub fn operator_hash_for_epoch(&self, epoch: &U256) -> Option<&OperatorsHash> {
        self.map.0.get_by_right(epoch)
    }

    /// Reallocate the state account data.
    pub fn reallocate<'a>(
        &self,
        state_account: &AccountInfo<'a>,
        payer: &AccountInfo<'a>,
        system_program: &AccountInfo<'a>,
    ) -> Result<(), ProgramError> {
        let data = borsh::to_vec(self)?;
        let size = data.len();
        let new_minimum_balance = Rent::get()?.minimum_balance(size);
        let lamports_diff = new_minimum_balance.saturating_sub(state_account.lamports());
        invoke(
            &system_instruction::transfer(payer.key, state_account.key, lamports_diff),
            &[payer.clone(), state_account.clone(), system_program.clone()],
        )?;
        state_account.realloc(size, false)?;
        state_account.try_borrow_mut_data()?[..size].copy_from_slice(&data);
        Ok(())
    }

    /// Returns the current epoch.
    pub fn current_epoch(&self) -> U256 {
        (self.map.0.len() as u128).into()
    }
}

#[test]
fn map_type_borsh_traits() {
    let mut bimap = BiBTreeMap::new();
    bimap.insert([1u8; 32], U256::from(5u8));
    bimap.insert([2u8; 32], U256::from(4u8));
    bimap.insert([3u8; 32], U256::from(3u8));
    bimap.insert([4u8; 32], U256::from(2u8));
    bimap.insert([5u8; 32], U256::from(1u8));
    let original = Map(bimap);

    let serialized = borsh::to_vec(&original).expect("can serialize Map");
    let deserialized: Map = borsh::from_slice(&serialized).expect("can serialize Map");

    assert_eq!(deserialized, original)
}
