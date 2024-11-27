use bitflags::Flags;
use rkyv::ser::Serializer;
use rkyv::with::{ArchiveWith, DeserializeWith, SerializeWith};
use rkyv::{Archive, Deserialize, Fallible, Serialize};

/// A wrapper to add rkyv support for bitflags.
pub struct ArchivableFlags;

impl<T> ArchiveWith<T> for ArchivableFlags
where
    T: Flags,
    T::Bits: Archive,
{
    type Archived = T::Bits;
    type Resolver = <<T as Flags>::Bits as Archive>::Resolver;

    unsafe fn resolve_with(
        field: &T,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        let bits = field.bits();
        bits.resolve(pos, resolver, out.cast());
    }
}

impl<T, S> SerializeWith<T, S> for ArchivableFlags
where
    S: Serializer + ?Sized,
    T: Flags,
    T::Bits: Archive + Serialize<S>,
{
    fn serialize_with(field: &T, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        let resolver = field.bits().serialize(serializer)?;

        Ok(resolver)
    }
}

impl<T, B, D> DeserializeWith<B, T, D> for ArchivableFlags
where
    T: Flags<Bits = B>,
    T::Bits: Copy,
    B: Archive,
    B::Archived: Deserialize<T::Bits, D>,
    D: Fallible + ?Sized,
{
    fn deserialize_with(field: &T::Bits, _: &mut D) -> Result<T, D::Error> {
        Ok(T::from_bits_truncate(*field))
    }
}
