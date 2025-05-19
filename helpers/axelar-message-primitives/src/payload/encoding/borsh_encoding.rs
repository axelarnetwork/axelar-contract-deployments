use borsh::{BorshDeserialize, BorshSerialize};

use crate::{DataPayload, PayloadError, SolanaAccountRepr};

impl<'payload> DataPayload<'payload> {
    pub(super) fn encode_borsh(&self) -> Result<Vec<u8>, PayloadError> {
        let mut writer_vec = self.encoding_scheme_prefixed_array();
        borsh::to_writer(
            &mut writer_vec,
            &(
                self.payload_without_accounts.to_vec(),
                self.solana_accounts.to_vec(),
            ),
        )
        .map_err(|_| PayloadError::BorshSerializeError)?;
        Ok(writer_vec)
    }

    pub(super) fn decode_borsh(
        data: &'payload [u8],
    ) -> Result<(Vec<u8>, Vec<SolanaAccountRepr>), PayloadError> {
        borsh::from_slice::<(Vec<u8>, Vec<SolanaAccountRepr>)>(data)
            .map_err(|_| PayloadError::BorshDeserializeError)
    }
}

impl BorshSerialize for SolanaAccountRepr {
    fn serialize<W: std::io::prelude::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(self.pubkey.as_ref())?;
        writer.write_all(&[self.is_signer as u8 | (self.is_writable as u8) << 1])?;
        Ok(())
    }
}

impl BorshDeserialize for SolanaAccountRepr {
    fn deserialize_reader<R: std::io::prelude::Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut key = [0u8; 32];
        reader.read_exact(&mut key)?;

        let mut flags = [0u8; 1];
        reader.read_exact(&mut flags)?;

        let is_signer = flags[0] & 1 == 1;
        let is_writable = flags[0] >> 1 == 1;

        Ok(SolanaAccountRepr {
            pubkey: key.into(),
            is_signer,
            is_writable,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payload::encoding::tests::{account_fixture, account_fixture_2};

    #[test]
    fn solana_account_repr_round_trip_borsh() {
        let repr = account_fixture_2();
        let repr_encoded = borsh::to_vec(&repr).unwrap();
        let repr2 = borsh::from_slice(&repr_encoded).unwrap();
        assert_eq!(repr, repr2);
    }
    #[test]
    fn account_serialization_borsh() {
        let accounts = account_fixture().to_vec();
        let encoded = borsh::to_vec::<Vec<SolanaAccountRepr>>(&accounts.to_vec()).unwrap();
        let decoded = borsh::from_slice::<Vec<SolanaAccountRepr>>(&encoded).unwrap();

        assert_eq!(accounts, decoded.as_slice());
    }
}
