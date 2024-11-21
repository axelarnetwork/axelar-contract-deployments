//! State related to role management.
use bitflags::bitflags;
use program_utils::StorableArchive;
use rkyv::{bytecheck, Archive, CheckBytes, Deserialize, Serialize};

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

/// Helper module to add rkyv support for [`Roles`].
pub mod archive {
    use rkyv::ser::Serializer;
    use rkyv::with::{ArchiveWith, DeserializeWith, SerializeWith};
    use rkyv::{Archive, Fallible};

    use super::Roles;

    /// A wrapper to add rkyv support for [`Roles`].
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

/// Roles assigned to a user on a specific resource.
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Clone)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq, CheckBytes))]
#[non_exhaustive]
pub struct UserRoles {
    #[with(archive::ArchivableRoles)]
    roles: Roles,
    bump: u8,
}

impl StorableArchive<0> for UserRoles {}

impl UserRoles {
    /// Creates a new instance of `UserRoles`.
    #[must_use]
    pub const fn new(roles: Roles, bump: u8) -> Self {
        Self { roles, bump }
    }

    /// Checks if the user has the provided role.
    #[must_use]
    pub const fn contains(&self, role: Roles) -> bool {
        self.roles.contains(role)
    }

    /// Adds a role to the user.
    pub fn add(&mut self, role: Roles) {
        self.roles.insert(role);
    }

    /// Removes a role from the user.
    #[allow(clippy::arithmetic_side_effects)]
    pub fn remove(&mut self, role: Roles) {
        self.roles.remove(role);
    }

    /// The bump associated with the PDA where this data is stored.
    #[must_use]
    pub const fn bump(&self) -> u8 {
        self.bump
    }
}

impl ArchivedUserRoles {
    /// Checks if the user has the provided role.
    #[must_use]
    pub const fn contains(&self, role: Roles) -> bool {
        let roles = Roles::from_bits_truncate(self.roles);

        roles.contains(role)
    }

    /// The bump associated with the PDA where this data is stored.
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

/// Proposal to transfer roles to a user.
#[repr(transparent)]
#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq, Copy, Clone)]
pub struct RoleProposal {
    /// The roles to be transferred.
    #[with(archive::ArchivableRoles)]
    pub roles: Roles,
}

impl StorableArchive<0> for RoleProposal {}

#[cfg(test)]
mod tests {
    use core::pin::Pin;

    use rkyv::ser::serializers::WriteSerializer;
    use rkyv::ser::Serializer;
    use rkyv::{check_archived_value, AlignedVec, Infallible};

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
