use borsh::BorshSerialize;

pub mod hash;

/// Namespace for calculating instruction sighash signatures for any instruction
/// not affecting program state.
// https://github.com/solana-foundation/anchor/blob/18d0ca0ce9b78c03ef370406c6ba86e28e4591ab/lang/syn/src/codegen/program/common.rs#L5-L7
pub const SIGHASH_GLOBAL_NAMESPACE: &str = "global";

/// Returns the first 8 bytes of the SHA256 hash of the string
/// "{namespace}:{name}".
// https://github.com/solana-foundation/anchor/blob/56b21edd1f4c1865e5f943537fb7f89a0ffe5ede/lang/syn/src/codegen/program/common.rs#L13
pub fn sighash(namespace: &str, name: &str) -> [u8; 8] {
    let preimage = format!("{namespace}:{name}");

    let mut sighash = [0u8; 8];
    sighash.copy_from_slice(&hash::hash(preimage.as_bytes()).to_bytes()[..8]);
    sighash
}

/// Unique identifier for a type.
pub trait Discriminator {
    /// Discriminator slice.
    ///
    /// See [`Discriminator`] trait documentation for more information.
    const DISCRIMINATOR: &'static [u8];
}

/// Calculates the data for an instruction invocation, where the data is
/// `Discriminator + BorshSerialize(args)`. `args` is a borsh serialized
/// struct of named fields for each argument given to an instruction.
pub trait InstructionData: Discriminator + BorshSerialize {
    fn data(&self) -> Vec<u8> {
        let mut data = Vec::with_capacity(256);
        data.extend_from_slice(Self::DISCRIMINATOR);
        self.serialize(&mut data).unwrap();
        data
    }

    /// Clears `data` and writes instruction data to it.
    ///
    /// We use a `Vec<u8>` here because of the additional flexibility of re-allocation (only if
    /// necessary), and because the data field in `Instruction` expects a `Vec<u8>`.
    fn write_to(&self, mut data: &mut Vec<u8>) {
        data.clear();
        data.extend_from_slice(Self::DISCRIMINATOR);
        self.serialize(&mut data).unwrap();
    }
}
