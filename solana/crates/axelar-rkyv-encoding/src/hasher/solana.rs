use super::{AxelarRkyv256Hasher, Hash256};
use crate::visitor::{ArchivedVisitor, Visitor};

#[derive(Clone, Default)]
pub struct SolanaKeccak256Hasher<'a> {
    state: Vec<&'a [u8]>,
}

impl<'a> Visitor<'a> for SolanaKeccak256Hasher<'a> {
    fn visit_bytes(&mut self, bytes: &'a [u8]) {
        self.state.push(bytes)
    }

    fn tag(&mut self, bytes: &'a [u8]) {
        Visitor::visit_bytes(self, bytes)
    }
}

impl<'a> ArchivedVisitor<'a> for SolanaKeccak256Hasher<'a> {
    fn visit_bytes(&mut self, bytes: &'a [u8]) {
        self.state.push(bytes)
    }

    fn tag(&mut self, bytes: &'a [u8]) {
        ArchivedVisitor::visit_bytes(self, bytes)
    }
}

impl<'a> AxelarRkyv256Hasher<'a> for SolanaKeccak256Hasher<'a> {
    fn hash(&mut self, val: &'a [u8]) {
        self.state.push(val);
    }
    fn hashv(&mut self, vals: &'a [&[u8]]) {
        for val in vals {
            self.hash(val);
        }
    }
    fn result(self) -> Hash256 {
        Hash256(solana_program::keccak::hashv(&self.state).to_bytes())
    }
}
