pub(crate) mod contract_call;
pub(crate) mod execute;

#[allow(dead_code)]
pub(crate) const PREFIX_COMMAND_EXECUTED: &str =
    "957705a374326b30f4a1069c936d736cc9993ed6c820b4e0e2fd94a8beca0d1d";
#[allow(dead_code)]
pub(crate) const PREFIX_CONTRACT_CALL_APPROVED: &str =
    "07b0d4304f82012bd3b70b1d531c160e326067c90829e2a3d386722ad10b89c3";
#[allow(dead_code)]
pub(crate) const SELECTOR_APPROVE_CONTRACT_CALL: &str =
    "37ac16aabc4d87540e53151b2b716265cfd6b195db96a9daf8e893c829bbd233";
#[allow(dead_code)]
pub(crate) const SELECTOR_TRANSFER_OPERATORSHIP: &str =
    "b460dcb6fd5797fc0e7ea0f13406c80d30702ba7f73a42bd91394775dcbca718";
