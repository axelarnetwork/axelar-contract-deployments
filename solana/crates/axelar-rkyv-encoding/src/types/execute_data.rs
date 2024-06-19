use rkyv::{Archive, Deserialize, Serialize};

use crate::hasher::Hasher;
use crate::types::{Payload, Proof, VerifierSet};

#[derive(Archive, Deserialize, Serialize, Debug, Eq, PartialEq)]
#[archive(compare(PartialEq))]
#[archive_attr(derive(Debug, PartialEq, Eq))]
pub struct ExecuteData {
    pub(crate) payload: Payload,
    pub(crate) proof: Proof,
}

impl ExecuteData {
    pub(crate) fn new(payload: Payload, proof: Proof) -> Self {
        Self { payload, proof }
    }
}

impl ArchivedExecuteData {
    pub fn hash_payload_for_verifier_set(
        &self,
        domain_separator: &[u8; 32],
        verifier_set: &VerifierSet,
    ) -> [u8; 32] {
        use crate::visitor::{ArchivedVisitor, Visitor};
        let mut hasher = Hasher::default();
        Visitor::visit_bytes(&mut hasher, domain_separator);
        Visitor::visit_verifier_set(&mut hasher, verifier_set);
        ArchivedVisitor::visit_payload(&mut hasher, &self.payload);
        hasher.finalize()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::fixtures::random_execute_data;
    use crate::visitor::{ArchivedVisitor, Visitor};

    #[test]
    fn test_serialize_deserialize_execute_data() {
        let mut rng = rand::thread_rng();
        let execute_data = random_execute_data(&mut rng);

        let serialized = rkyv::to_bytes::<_, 1024>(&execute_data).unwrap().to_vec();
        let archived = unsafe { rkyv::archived_root::<ExecuteData>(&serialized) };

        assert_eq!(*archived, execute_data);
    }

    #[test]
    fn archived_and_unarchived_values_have_the_same_hash() {
        let mut rng = rand::thread_rng();
        let execute_data = random_execute_data(&mut rng);

        let serialized = rkyv::to_bytes::<_, 1024>(&execute_data).unwrap().to_vec();
        let archived = unsafe { rkyv::archived_root::<ExecuteData>(&serialized) };

        let mut archived_hasher = Hasher::default();
        let mut unarchived_hasher = Hasher::default();

        Visitor::visit_execute_data(&mut unarchived_hasher, &execute_data);
        ArchivedVisitor::visit_execute_data(&mut archived_hasher, archived);

        assert_eq!(archived_hasher.finalize(), unarchived_hasher.finalize());
    }
}
