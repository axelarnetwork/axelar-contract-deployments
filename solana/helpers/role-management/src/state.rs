//! State related to role management.
use core::fmt::Debug;

use axelar_rkyv_encoding::types::ArchivableFlags;
use bitflags::Flags;
use program_utils::StorableArchive;
use rkyv::de::deserializers::SharedDeserializeMap;
use rkyv::ser::serializers::{
    AlignedSerializer, AllocScratch, CompositeSerializer, FallbackScratch, HeapScratch,
    SharedSerializeMap,
};
use rkyv::validation::validators::DefaultValidator;
use rkyv::{bytecheck, AlignedVec, Archive, CheckBytes, Deserialize, Infallible, Serialize};

/// Flags representing the roles that can be assigned to a user. Users shouldn't
/// need to implement this manually as we have a blanket implementation for
/// `Flags`.
pub trait RolesFlags: Flags<Bits = Self::RawBits> + Debug + Clone + PartialEq + Eq + Copy {
    /// The archived version of the flags.
    type ArchivedBits: Deserialize<Self::RawBits, SharedDeserializeMap>
        + Deserialize<Self::RawBits, Infallible>
        + for<'a> CheckBytes<DefaultValidator<'a>>;

    /// The raw bits representing the flags.
    type RawBits: Serialize<
            CompositeSerializer<
                AlignedSerializer<AlignedVec>,
                FallbackScratch<HeapScratch<0>, AllocScratch>,
                SharedSerializeMap,
            >,
        > + Archive<Archived = Self::ArchivedBits>
        + Debug
        + Eq
        + PartialEq
        + Clone
        + Copy;
}

impl<T> RolesFlags for T
where
    T: Flags + Debug + Clone + PartialEq + Eq + Copy,
    T::Bits: Serialize<
            CompositeSerializer<
                AlignedSerializer<AlignedVec>,
                FallbackScratch<HeapScratch<0>, AllocScratch>,
                SharedSerializeMap,
            >,
        > + Archive
        + Debug
        + Eq
        + PartialEq
        + Clone
        + Copy,

    <T::Bits as Archive>::Archived: Deserialize<Self::Bits, SharedDeserializeMap>
        + Deserialize<Self::Bits, Infallible>
        + Debug
        + Eq
        + PartialEq
        + Clone
        + Copy
        + for<'a> CheckBytes<DefaultValidator<'a>>,
{
    type ArchivedBits = <T::Bits as Archive>::Archived;
    type RawBits = T::Bits;
}

/// Roles assigned to a user on a specific resource.
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(CheckBytes))]
#[non_exhaustive]
pub struct UserRoles<F: RolesFlags> {
    #[with(ArchivableFlags)]
    roles: F,
    bump: u8,
}

impl<F> StorableArchive<0> for UserRoles<F> where F: RolesFlags {}

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

impl<F> ArchivedUserRoles<F>
where
    F: RolesFlags,
{
    /// Checks if the user has the provided role.
    #[must_use]
    pub fn contains(&self, role: F) -> bool {
        let roles = F::from_bits_truncate(self.roles);

        roles.contains(role)
    }

    /// The bump associated with the PDA where this data is stored.
    #[must_use]
    pub const fn bump(&self) -> u8 {
        self.bump
    }
}

impl<F> From<&ArchivedUserRoles<F>> for UserRoles<F>
where
    F: RolesFlags,
{
    fn from(value: &ArchivedUserRoles<F>) -> Self {
        Self {
            roles: F::from_bits_truncate(value.roles),
            bump: value.bump,
        }
    }
}

/// Proposal to transfer roles to a user.
#[repr(transparent)]
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Copy, Clone)]
pub struct RoleProposal<F: RolesFlags> {
    /// The roles to be transferred.
    #[with(ArchivableFlags)]
    pub roles: F,
}

impl<F> StorableArchive<0> for RoleProposal<F> where F: RolesFlags {}

#[cfg(test)]
mod tests {
    use core::pin::Pin;

    use bitflags::bitflags;
    use rkyv::check_archived_value;
    use rkyv::ser::serializers::WriteSerializer;
    use rkyv::ser::Serializer;

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

    #[test]
    fn test_user_roles_round_trip() {
        let original = UserRoles {
            roles: Roles::MINTER | Roles::OPERATOR,
            bump: 42,
        };

        let mut serializer = WriteSerializer::new(AlignedVec::new());
        let pos = serializer.serialize_value(&original).unwrap();
        let bytes = serializer.into_inner();

        check_archived_value::<UserRoles<Roles>>(&bytes, pos).unwrap();

        // SAFETY: The archived data is valid
        let archived = unsafe { rkyv::archived_root::<UserRoles<Roles>>(&bytes) };

        let deserialized: UserRoles<Roles> = archived.deserialize(&mut Infallible).unwrap();

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

            check_archived_value::<UserRoles<Roles>>(&bytes, pos).unwrap();

            // SAFETY: The archived data is valid
            let archived = unsafe { rkyv::archived_root::<UserRoles<Roles>>(&bytes) };

            let deserialized: UserRoles<Roles> = archived.deserialize(&mut Infallible).unwrap();

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

        check_archived_value::<UserRoles<Roles>>(&bytes, pos).unwrap();

        // SAFETY: The archived data is valid
        let mut archived =
            unsafe { rkyv::archived_root_mut::<UserRoles<Roles>>(Pin::new(bytes.as_mut_slice())) };

        // Force the invalid bits into the archived data. Since when archived roles is
        // just an u8, the invalid bits will be accepted, but when
        // deserializing, the invalid bits should be truncated.
        archived.roles = invalid_bits;

        let deserialized: UserRoles<Roles> = (*archived).deserialize(&mut Infallible).unwrap();

        // Since from_bits_truncate ignores invalid bits, the deserialized roles should
        // contain only the valid bits, which are equivalent to MINTER in this
        // case.
        assert_eq!(deserialized.roles, Roles::MINTER);
    }
}
