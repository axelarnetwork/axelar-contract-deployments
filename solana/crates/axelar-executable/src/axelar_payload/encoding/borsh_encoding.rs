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

    /// Decodes a Borsh-encoded byte slice into two parts: a payload reference and Solana accounts.
    ///
    /// The function performs custom deserialization to return a reference to the original payload
    /// bytes while deserializing the Solana accounts into a new vector. This approach is used
    /// instead of `borsh::from_slice` to prevent copying payload bytes, which could exhaust the
    /// limited program memory and cause out-of-memory errors.
    ///
    /// Since this is a helper crate used in Solana programs, which typically use a bump allocator
    /// without free operations, we minimize heap allocations to reduce memory impact on the end
    /// user's program.
    pub(super) fn decode_borsh(
        raw_payload: &'payload [u8],
    ) -> Result<(&'payload [u8], Vec<SolanaAccountRepr>), PayloadError> {
        // Borsh stores the length of a serialized vector (the payload in this case)as a
        // little-endian u32.
        let payload_length = raw_payload
            .get(..4)
            .and_then(|bytes| bytes.try_into().ok())
            .map(u32::from_le_bytes)
            .and_then(|len| len.try_into().ok())
            .ok_or(PayloadError::BorshDeserializeError)?;

        // Split into payload and accounts data
        let (payload_slice, accounts_slice) = raw_payload
            .get(4..)
            .and_then(|bytes| bytes.get(..payload_length).zip(bytes.get(payload_length..)))
            .ok_or(PayloadError::BorshDeserializeError)?;

        // Deserialize accounts data using Borsh
        let solana_accounts = borsh::from_slice(accounts_slice)
            .map_err(|_err| PayloadError::BorshDeserializeError)?;

        Ok((payload_slice, solana_accounts))
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
    use rand::{thread_rng, Rng, RngCore};

    #[test]
    fn solana_account_repr_round_trip_borsh() {
        let repr = account_fixture_2();
        let repr_encoded = borsh::to_vec(&repr).unwrap();
        let _repr2: SolanaAccountRepr = borsh::from_slice(&repr_encoded).unwrap();
    }

    #[test]
    fn account_serialization_borsh() {
        let accounts = account_fixture().to_vec();
        let encoded = borsh::to_vec::<Vec<SolanaAccountRepr>>(&accounts).unwrap();
        let decoded = borsh::from_slice::<Vec<SolanaAccountRepr>>(&encoded).unwrap();

        assert_eq!(accounts, decoded.as_slice());
    }

    fn random_payload_bytes(len: usize) -> Vec<u8> {
        let mut bytes = vec![0_u8; len];
        thread_rng().fill_bytes(&mut bytes);
        bytes
    }

    fn create_test_account() -> SolanaAccountRepr {
        let pubkey: [u8; 32] = random_payload_bytes(32).try_into().unwrap();
        SolanaAccountRepr {
            pubkey: pubkey.into(),
            is_signer: thread_rng().gen_bool(0.5),
            is_writable: thread_rng().gen_bool(0.5),
        }
    }

    #[test]
    fn test_empty_payload() {
        let payload: &[u8] = &[];
        let accounts = vec![create_test_account()];

        let encoded = borsh::to_vec(&(payload, accounts.clone())).unwrap();
        let (payload_ref, decoded_accounts) = AxelarMessagePayload::decode_borsh(&encoded).unwrap();

        assert!(payload_ref.is_empty());
        assert_eq!(decoded_accounts, accounts);
    }

    #[test]
    fn test_decode_empty_accounts() {
        let payload = random_payload_bytes(100);
        let accounts: Vec<SolanaAccountRepr> = vec![];

        let encoded = borsh::to_vec(&(payload.clone(), accounts)).unwrap();

        let (payload_ref, decoded_accounts) = AxelarMessagePayload::decode_borsh(&encoded).unwrap();

        assert_eq!(payload_ref, &payload);
        assert!(decoded_accounts.is_empty());
    }

    #[test]
    fn test_decode_multiple_accounts() {
        let payload = random_payload_bytes(100);
        let accounts: Vec<SolanaAccountRepr> = (0_u8..12).map(|_| create_test_account()).collect();

        let encoded = borsh::to_vec(&(payload.clone(), accounts.clone())).unwrap();
        let (payload_ref, decoded_accounts) = AxelarMessagePayload::decode_borsh(&encoded).unwrap();

        assert_eq!(payload_ref, &payload);
        assert_eq!(decoded_accounts, accounts);
    }

    #[test]
    fn test_decode_large_payload() {
        let payload = random_payload_bytes(0x0010_0000); // 1MB payload
        let accounts = vec![create_test_account()];

        let encoded = borsh::to_vec(&(payload.clone(), accounts.clone())).unwrap();
        let (payload_ref, decoded_accounts) = AxelarMessagePayload::decode_borsh(&encoded).unwrap();

        assert_eq!(payload_ref, &payload);
        assert_eq!(decoded_accounts, accounts);
    }

    #[test]
    fn test_decode_invalid_input() {
        // Test: input too short: less than 4 bytes, can't read payload size (u32)
        assert!(AxelarMessagePayload::decode_borsh(&[1, 2, 3]).is_err());

        // Test: invalid/garbage data
        assert!(AxelarMessagePayload::decode_borsh(&random_payload_bytes(256)).is_err());
    }
}
