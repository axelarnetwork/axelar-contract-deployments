use sha3::{Digest, Keccak256};

use crate::types::U256;
use crate::visitor::Visitor;

#[derive(Default)]
pub(crate) struct PayloadHasher {
    state: Keccak256,
}

impl PayloadHasher {
    pub fn finalize(self) -> [u8; 32] {
        self.state.finalize().into()
    }
}

impl Visitor for PayloadHasher {
    fn visit_u64(&mut self, number: &u64) {
        self.state.update(number.to_be_bytes())
    }

    fn visit_u256(&mut self, number: &U256) {
        self.state.update(number.to_be())
    }

    fn visit_bytes(&mut self, bytes: &[u8]) {
        self.state.update(bytes)
    }

    fn tag(&mut self, bytes: &[u8]) {
        self.visit_bytes(bytes)
    }
}
