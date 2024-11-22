use borsh::{BorshDeserialize, BorshSerialize};

use crate::{AxelarMessagePayload, PayloadError, SolanaAccountRepr};

impl<'payload> AxelarMessagePayload<'payload> {
    pub(super) fn encode_borsh(&self) -> Result<Vec<u8>, PayloadError> {
        let mut writer_vec = self.encoding_scheme_prefixed_array()?;
        borsh::to_writer(
            &mut writer_vec,
            &(
                self.payload_without_accounts.to_vec(),
                self.solana_accounts.clone(),
            ),
        )
        .map_err(|_err| PayloadError::BorshSerializeError)?;
        Ok(writer_vec)
    }

    pub(super) fn decode_borsh(
        data: &'payload [u8],
    ) -> Result<(Vec<u8>, Vec<SolanaAccountRepr>), PayloadError> {
        borsh::from_slice::<(Vec<u8>, Vec<SolanaAccountRepr>)>(data)
            .map_err(|_err| PayloadError::BorshDeserializeError)
    }
}

impl BorshSerialize for SolanaAccountRepr {
    fn serialize<W: std::io::prelude::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        writer.write_all(self.pubkey.as_ref())?;
        writer.write_all(&[u8::from(self.is_signer) | u8::from(self.is_writable) << 1_u8])?;
        Ok(())
    }
}

impl BorshDeserialize for SolanaAccountRepr {
    fn deserialize_reader<R: std::io::prelude::Read>(reader: &mut R) -> std::io::Result<Self> {
        let mut key = [0_u8; 32];
        reader.read_exact(&mut key)?;

        let mut flags = [0_u8; 1];
        reader.read_exact(&mut flags)?;

        let is_signer = flags[0] & 1 == 1;
        let is_writable = flags[0] >> 1_u8 == 1;

        Ok(Self {
            pubkey: key.into(),
            is_signer,
            is_writable,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::axelar_payload::encoding::tests::{account_fixture, account_fixture_2};

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
        let encoded = borsh::to_vec::<Vec<SolanaAccountRepr>>(&accounts).unwrap();
        let decoded = borsh::from_slice::<Vec<SolanaAccountRepr>>(&encoded).unwrap();

        assert_eq!(accounts, decoded.as_slice());
    }
}
