#![allow(clippy::missing_errors_doc)] // TODO: Remove this
#![allow(missing_docs)] // TODO: Remove this

use std::io::Write;

use bitflags::bitflags;
use program_utils::{check_rkyv_initialized_pda, init_rkyv_pda};
use rkyv::{bytecheck, Archive, CheckBytes, Deserialize, Infallible, Serialize};
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::seed_prefixes;

bitflags! {
    #[derive(Debug, Eq, PartialEq, Clone, Copy)]
    pub struct Roles: u8 {
        const MINTER = 0b0000_0001;
        const OPERATOR = 0b0000_0010;
        const FLOW_LIMITER = 0b0000_0100;
    }
}

impl PartialEq<u8> for Roles {
    fn eq(&self, other: &u8) -> bool {
        self.bits().eq(other)
    }
}

impl PartialEq<Roles> for u8 {
    fn eq(&self, other: &Roles) -> bool {
        self.eq(&other.bits())
    }
}

pub mod archive {
    use rkyv::ser::Serializer;
    use rkyv::with::{ArchiveWith, DeserializeWith, SerializeWith};
    use rkyv::{Archive, Fallible};

    use super::Roles;

    pub struct ArchivableRoles;

    impl ArchiveWith<Roles> for ArchivableRoles {
        type Archived = u8;
        type Resolver = ();

        unsafe fn resolve_with(
            field: &Roles,
            pos: usize,
            resolver: Self::Resolver,
            out: *mut Self::Archived,
        ) {
            let bits = field.bits();
            bits.resolve(pos, resolver, out);
        }
    }

    impl<S: Serializer + ?Sized> SerializeWith<Roles, S> for ArchivableRoles {
        fn serialize_with(field: &Roles, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
            serializer.serialize_value(&field.bits())?;

            Ok(())
        }
    }

    impl<D: Fallible + ?Sized> DeserializeWith<u8, Roles, D> for ArchivableRoles {
        fn deserialize_with(field: &u8, _: &mut D) -> Result<Roles, D::Error> {
            Ok(Roles::from_bits_truncate(*field))
        }
    }
}

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
#[non_exhaustive]
pub struct UserRoles {
    #[with(archive::ArchivableRoles)]
    roles: Roles,
    bump: u8,
}

impl UserRoles {
    #[must_use]
    pub const fn new(roles: Roles, bump: u8) -> Self {
        Self { roles, bump }
    }

    pub fn init<'a>(
        &self,
        program_id: &Pubkey,
        system_account: &AccountInfo<'a>,
        payer: &AccountInfo<'a>,
        resource: &AccountInfo<'a>,
        user: &AccountInfo<'a>,
        into: &AccountInfo<'a>,
    ) -> ProgramResult {
        let (pda, _) = crate::create_user_roles_pda(program_id, resource.key, user.key, self.bump);

        if pda != *into.key {
            msg!("Invalid PDA account or internal bump");
            return Err(ProgramError::InvalidArgument);
        }

        init_rkyv_pda::<0, Self>(
            payer,
            into,
            program_id,
            system_account,
            self.clone(),
            &[
                seed_prefixes::USER_ROLES_SEED,
                resource.key.as_ref(),
                user.key.as_ref(),
                &[self.bump],
            ],
        )
    }

    #[must_use]
    pub const fn contains(&self, role: Roles) -> bool {
        self.roles.contains(role)
    }

    pub fn add(&mut self, role: Roles) {
        self.roles.insert(role);
    }

    #[allow(clippy::arithmetic_side_effects)]
    pub fn remove(&mut self, role: Roles) {
        self.roles.remove(role);
    }

    pub fn store(&self, destination: &AccountInfo<'_>) -> ProgramResult {
        let mut account_data = destination.try_borrow_mut_data()?;
        let data = rkyv::to_bytes::<_, 0>(self).map_err(|_err| ProgramError::InvalidAccountData)?;

        account_data
            .write_all(&data)
            .map_err(|_err| ProgramError::InvalidAccountData)
    }

    pub fn load(
        program_id: &Pubkey,
        source_account: &AccountInfo<'_>,
    ) -> Result<Self, ProgramError> {
        let account_data = source_account.try_borrow_data()?;
        let archived =
            check_rkyv_initialized_pda::<Self>(program_id, source_account, &account_data)?;

        archived
            .deserialize(&mut Infallible)
            .map_err(|_err| ProgramError::InvalidAccountData)
    }

    #[must_use]
    pub const fn bump(&self) -> u8 {
        self.bump
    }
}

impl ArchivedUserRoles {
    #[must_use]
    pub const fn contains(&self, role: Roles) -> bool {
        let roles = Roles::from_bits_truncate(self.roles);

        roles.contains(role)
    }

    #[must_use]
    pub const fn bump(&self) -> u8 {
        self.bump
    }
}

impl From<&ArchivedUserRoles> for UserRoles {
    fn from(value: &ArchivedUserRoles) -> Self {
        Self {
            roles: Roles::from_bits_truncate(value.roles),
            bump: value.bump,
        }
    }
}

#[repr(transparent)]
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Copy, Clone)]
pub struct RoleProposal {
    #[with(archive::ArchivableRoles)]
    pub roles: Roles,
}

impl RoleProposal {
    pub fn store(&self, destination: &AccountInfo<'_>) -> ProgramResult {
        let mut account_data = destination.try_borrow_mut_data()?;
        let data = rkyv::to_bytes::<_, 0>(self).map_err(|_err| ProgramError::InvalidAccountData)?;

        account_data
            .write_all(&data)
            .map_err(|_err| ProgramError::InvalidAccountData)
    }

    pub fn load(
        program_id: &Pubkey,
        source_account: &AccountInfo<'_>,
    ) -> Result<Self, ProgramError> {
        let account_data = source_account.try_borrow_data()?;
        let archived =
            check_rkyv_initialized_pda::<Self>(program_id, source_account, &account_data)?;

        archived
            .deserialize(&mut Infallible)
            .map_err(|_err| ProgramError::InvalidAccountData)
    }
}

#[cfg(test)]
mod tests {
    use core::pin::Pin;

    use rkyv::ser::serializers::WriteSerializer;
    use rkyv::ser::Serializer;
    use rkyv::{check_archived_value, AlignedVec};

    use super::*;

    #[test]
    fn test_user_roles_round_trip() {
        let original = UserRoles {
            roles: Roles::MINTER | Roles::OPERATOR,
            bump: 42,
        };

        let mut serializer = WriteSerializer::new(AlignedVec::new());
        let pos = serializer.serialize_value(&original).unwrap();
        let bytes = serializer.into_inner();

        check_archived_value::<UserRoles>(&bytes, pos).unwrap();

        // SAFETY: The archived data is valid
        let archived = unsafe { rkyv::archived_root::<UserRoles>(&bytes) };

        let deserialized: UserRoles = archived.deserialize(&mut Infallible).unwrap();

        assert_eq!(original, deserialized);
        assert!(original.contains(Roles::MINTER));
        assert!(original.contains(Roles::OPERATOR));
        assert!(deserialized.contains(Roles::MINTER | Roles::OPERATOR));
    }

    #[test]
    fn test_roles_bitflags() {
        let roles_list = vec![
            Roles::MINTER,
            Roles::OPERATOR,
            Roles::FLOW_LIMITER,
            Roles::MINTER | Roles::OPERATOR,
            Roles::OPERATOR | Roles::FLOW_LIMITER,
            Roles::MINTER | Roles::FLOW_LIMITER,
            Roles::MINTER | Roles::OPERATOR | Roles::FLOW_LIMITER,
        ];

        for roles in roles_list {
            let original = UserRoles { roles, bump: 0 };

            let mut serializer = WriteSerializer::new(AlignedVec::new());
            let pos = serializer.serialize_value(&original).unwrap();
            let bytes = serializer.into_inner();

            check_archived_value::<UserRoles>(&bytes, pos).unwrap();

            // SAFETY: The archived data is valid
            let archived = unsafe { rkyv::archived_root::<UserRoles>(&bytes) };

            let deserialized: UserRoles = archived.deserialize(&mut Infallible).unwrap();

            assert_eq!(original, deserialized);
        }
    }

    #[test]
    fn test_invalid_roles_bits() {
        let invalid_bits = 0b1111_0001;
        let truncated_roles = Roles::from_bits_truncate(invalid_bits);

        let original = UserRoles {
            roles: truncated_roles,
            bump: 0,
        };

        let mut serializer = WriteSerializer::new(AlignedVec::new());
        let pos = serializer.serialize_value(&original).unwrap();
        let mut bytes = serializer.into_inner();

        check_archived_value::<UserRoles>(&bytes, pos).unwrap();

        // SAFETY: The archived data is valid
        let mut archived =
            unsafe { rkyv::archived_root_mut::<UserRoles>(Pin::new(bytes.as_mut_slice())) };

        // Force the invalid bits into the archived data. Since when archived roles is
        // just an u8, the invalid bits will be accepted, but when
        // deserializing, the invalid bits should be truncated.
        archived.roles = invalid_bits;

        let deserialized: UserRoles = (*archived).deserialize(&mut Infallible).unwrap();

        // Since from_bits_truncate ignores invalid bits, the deserialized roles should
        // contain only the valid bits, which are equivalent to MINTER in this
        // case.
        assert_eq!(deserialized.roles, Roles::MINTER);
    }
}
