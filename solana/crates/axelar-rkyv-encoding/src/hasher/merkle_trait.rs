use rs_merkle::{MerkleProof, MerkleTree};

type Hash = [u8; 32];

/// A trait for constructing and interacting with Merkle Trees.
///
/// `MerkleTrait` is an interface for generating Merkle trees,
/// calculating Merkle roots, and obtaining Merkle proofs for data
/// structures.
///
/// Implementors of this trait must define how to extract Merkle leaves from
/// their data, while the trait provides default implementations for building
/// the tree, calculating the root, and generating proofs.
pub trait Merkle<MerkleHasher>
where
    MerkleHasher: rs_merkle::Hasher<Hash = Hash>,
{
    /// A type that encapsulates all necessary data with sufficient entropy to
    /// uniquely represent `Self` as a leaf node within a Merkle tree.
    type LeafNode: Into<Hash>;

    /// Extracts the Merkle leaf nodes from the implementor.
    fn merkle_leaves(&self) -> impl Iterator<Item = Self::LeafNode>;

    /// Builds a Merkle tree using the leaves collected from
    /// [`Self::merkle_leaves`]].
    fn build_merkle_tree(&self) -> MerkleTree<MerkleHasher> {
        let leaves: Vec<_> = self.merkle_leaves().map(Into::into).collect();
        MerkleTree::from_leaves(&leaves)
    }

    /// Calculates the Merkle root of the constructed Merkle tree.
    fn calculate_merkle_root(&self) -> Option<Hash> {
        self.build_merkle_tree().root()
    }

    /// Generates Merkle proofs for each leaf in the tree.
    ///
    /// This method builds a new Merkle tree and generates a corresponding
    /// `MerkleProof` for each of its leaf elements.
    fn merkle_proofs(&self) -> impl Iterator<Item = MerkleProof<MerkleHasher>> {
        let tree = self.build_merkle_tree();
        let num_leaves = tree.leaves_len();
        let mut count = 0;

        std::iter::from_fn(move || {
            if count < num_leaves {
                let proof = tree.proof(&[count]);
                count += 1;
                Some(proof)
            } else {
                None
            }
        })
    }
}

#[cfg(test)]
pub mod tests {

    use itertools::{EitherOrBoth, Itertools};
    use sha3::{Digest, Keccak256};

    use super::*;
    use crate::hasher::merkle_tree::{NativeHasher, SolanaSyscallHasher};

    struct Stub(&'static str);
    struct StubLeaf(char);

    impl From<StubLeaf> for Hash {
        fn from(value: StubLeaf) -> Self {
            let mut buffer = [0; 4];
            value.0.encode_utf8(&mut buffer);
            Keccak256::digest(buffer).into()
        }
    }

    impl<M: rs_merkle::Hasher<Hash = [u8; 32]>> Merkle<M> for Stub {
        type LeafNode = StubLeaf;

        fn merkle_leaves(&self) -> impl Iterator<Item = Self::LeafNode> {
            self.0.chars().map(StubLeaf)
        }
    }

    /// Asserts that the Merkle inclusion proofs for a given value are valid.
    /// Returns the calculated Merkle root as a 32-byte array.
    pub(crate) fn assert_merkle_inclusion_proof<H, T>(thing: &T) -> [u8; 32]
    where
        T: Merkle<H>,
        H: rs_merkle::Hasher<Hash = [u8; 32]>,
    {
        let merkle_tree = thing.build_merkle_tree();
        let merkle_root = thing
            .calculate_merkle_root()
            .expect("expected a non-empty merkle tree");
        let proofs = thing.merkle_proofs();
        let leaves = thing.merkle_leaves();

        // Verify the inclusion proof for every leaf node.
        for (idx, pair) in proofs.zip_longest(leaves).enumerate() {
            let EitherOrBoth::Both(proof, leaf) = pair else {
                panic!("proof and leaf iterators must yield the same number of items")
            };
            assert!(
                proof.verify(
                    merkle_root,
                    &[idx],
                    &[leaf.into()],
                    merkle_tree.leaves_len()
                ),
                "Merkle proof should be valid for leaf index {}",
                idx
            );
        }
        merkle_root
    }

    #[test]
    fn test_stub_implementation() {
        let thing = &Stub("It's the job that's never started as takes longest to finish");
        assert_eq!(
            assert_merkle_inclusion_proof::<SolanaSyscallHasher, _>(thing),
            assert_merkle_inclusion_proof::<NativeHasher, _>(thing),
            "different hasher implementations should produce the same merkle root"
        );
    }
}
