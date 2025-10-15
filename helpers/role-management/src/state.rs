//! State related to role management.
use core::fmt::Debug;

use anchor_discriminators::Discriminator;
use anchor_discriminators_macros::account;

use bitflags::Flags;
use borsh::{BorshDeserialize, BorshSerialize};
use program_utils::pda::BorshPda;

/// Flags representing the roles that can be assigned to a user. Users shouldn't
/// need to implement this manually as we have a blanket implementation for
/// `Flags`.
pub trait RolesFlags:
    Flags<Bits = Self::RawBits>
    + Debug
    + Clone
    + PartialEq
    + Eq
    + Copy
    + BorshSerialize
    + BorshDeserialize
{
    /// The raw bits representing the flags.
    type RawBits: Debug + Eq + PartialEq + Clone + Copy + BorshSerialize + BorshDeserialize;
}

impl<T> RolesFlags for T
where
    T: Flags + Debug + Clone + PartialEq + Eq + Copy + BorshSerialize + BorshDeserialize,
    T::Bits: Debug + Eq + PartialEq + Clone + Copy + BorshSerialize + BorshDeserialize,
{
    type RawBits = T::Bits;
}

/// Roles assigned to a user on a specific resource.
#[account]
#[derive(Debug, Eq, PartialEq, Clone)]
pub struct UserRoles<F: RolesFlags> {
    roles: F,
    bump: u8,
}

impl<F> UserRoles<F>
where
    F: RolesFlags,
{
    /// Creates a new instance of `UserRoles`.
    #[must_use]
    pub const fn new(roles: F, bump: u8) -> Self {
        Self { roles, bump }
    }

    /// Checks if the user has the provided role.
    #[must_use]
    pub fn contains(&self, role: F) -> bool {
        self.roles.contains(role)
    }

    /// Adds a role to the user.
    pub fn add(&mut self, role: F) {
        self.roles.insert(role);
    }

    /// Removes a role from the user.
    #[allow(clippy::arithmetic_side_effects)]
    pub fn remove(&mut self, role: F) {
        self.roles.remove(role);
    }

    /// The bump associated with the PDA where this data is stored.
    #[must_use]
    pub const fn bump(&self) -> u8 {
        self.bump
    }
}

impl<F> BorshPda for UserRoles<F> where F: RolesFlags {}

/// Proposal to transfer roles to a user.
#[account]
#[derive(Debug, Eq, PartialEq, Clone, Copy)]
pub struct RoleProposal<F: RolesFlags> {
    /// The roles to be transferred.
    pub roles: F,

    /// The bump associated with the PDA where this data is stored.
    pub bump: u8,
}

impl<F> BorshPda for RoleProposal<F> where F: RolesFlags {}

#[cfg(test)]
mod tests {
    use bitflags::bitflags;
    use borsh::to_vec;

    use super::*;

    bitflags! {
        /// Roles that can be assigned to a user.
        #[derive(Debug, Eq, PartialEq, Clone, Copy)]
        pub struct Roles: u8 {
            /// Can mint new tokens.
            const MINTER = 0b0000_0001;

            /// Can perform operations on the resource.
            const OPERATOR = 0b0000_0010;

            /// Can change the limit to the flow of tokens.
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

    impl BorshSerialize for Roles {
        fn serialize<W: std::io::prelude::Write>(&self, writer: &mut W) -> std::io::Result<()> {
            self.bits().serialize(writer)
        }
    }

    impl BorshDeserialize for Roles {
        fn deserialize_reader<R: std::io::prelude::Read>(reader: &mut R) -> std::io::Result<Self> {
            let byte = u8::deserialize_reader(reader)?;
            Ok(Self::from_bits_truncate(byte))
        }
    }

    #[test]
    #[allow(clippy::indexing_slicing)]
    fn test_user_roles_round_trip() {
        let original = UserRoles {
            roles: Roles::MINTER | Roles::OPERATOR,
            bump: 42,
        };

        let serialized = to_vec(&original).unwrap();
        assert_eq!(
            &serialized[..UserRoles::<Roles>::DISCRIMINATOR.len()],
            UserRoles::<Roles>::DISCRIMINATOR
        );
        let deserialized = UserRoles::<Roles>::try_from_slice(&serialized).unwrap();

        assert_eq!(original, deserialized);
        assert!(original.contains(Roles::MINTER));
        assert!(original.contains(Roles::OPERATOR));
        assert!(deserialized.contains(Roles::MINTER | Roles::OPERATOR));
    }

    #[test]
    #[allow(clippy::indexing_slicing)]
    fn test_role_proposal_round_trip() {
        let original = RoleProposal {
            roles: Roles::MINTER | Roles::FLOW_LIMITER,
            bump: 24,
        };

        let serialized = to_vec(&original).unwrap();
        assert_eq!(
            &serialized[..RoleProposal::<Roles>::DISCRIMINATOR.len()],
            RoleProposal::<Roles>::DISCRIMINATOR
        );
        let deserialized = RoleProposal::<Roles>::try_from_slice(&serialized).unwrap();

        assert_eq!(original, deserialized);
        assert!(original.roles.contains(Roles::MINTER));
        assert!(original.roles.contains(Roles::FLOW_LIMITER));
        assert!(deserialized
            .roles
            .contains(Roles::MINTER | Roles::FLOW_LIMITER));
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

            let serialized = to_vec(&original).unwrap();
            let deserialized = UserRoles::<Roles>::try_from_slice(&serialized).unwrap();

            assert_eq!(original, deserialized);
        }
    }
}
