use sha3::{Digest, Keccak256};

use crate::visitor::{ArchivedVisitor, Visitor};

#[derive(Default)]
pub(crate) struct Hasher {
    state: Keccak256,
}

impl Hasher {
    pub fn finalize(self) -> [u8; 32] {
        self.state.finalize().into()
    }
}

impl Visitor for Hasher {
    fn visit_bytes(&mut self, bytes: &[u8]) {
        self.state.update(bytes)
    }

    fn tag(&mut self, bytes: &[u8]) {
        Visitor::visit_bytes(self, bytes)
    }
}

impl ArchivedVisitor for Hasher {
    fn visit_bytes(&mut self, bytes: &[u8]) {
        self.state.update(bytes)
    }

    fn tag(&mut self, bytes: &[u8]) {
        ArchivedVisitor::visit_bytes(self, bytes)
    }
}
