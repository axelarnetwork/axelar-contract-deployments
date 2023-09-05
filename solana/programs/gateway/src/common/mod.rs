use borsh::{BorshDeserialize, BorshSerialize};

pub(crate) mod contract_call;
pub(crate) mod execute;

pub(crate) const PREFIX_COMMAND_EXECUTED: &'static str =
    "957705a374326b30f4a1069c936d736cc9993ed6c820b4e0e2fd94a8beca0d1d";
pub(crate) const PREFIX_CONTRACT_CALL_APPROVED: &'static str =
    "07b0d4304f82012bd3b70b1d531c160e326067c90829e2a3d386722ad10b89c3";
pub(crate) const SELECTOR_APPROVE_CONTRACT_CALL: &'static str =
    "37ac16aabc4d87540e53151b2b716265cfd6b195db96a9daf8e893c829bbd233";
pub(crate) const SELECTOR_TRANSFER_OPERATORSHIP: &'static str =
    "b460dcb6fd5797fc0e7ea0f13406c80d30702ba7f73a42bd91394775dcbca718";
