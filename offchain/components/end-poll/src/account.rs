use cosmos_sdk_proto::{
    cosmos::auth::v1beta1::{
        query_client::QueryClient, BaseAccount, QueryAccountRequest, QueryAccountResponse,
    },
    traits::Message,
};
use cosmrs::{
    crypto::{secp256k1, PublicKey},
    AccountId,
};
use tonic::transport::Channel;
use tonic::{Response, Status};

pub struct AxelarAccount {
    pub id: AccountId,
    pub private_key: secp256k1::SigningKey,
    pub public_key: PublicKey,
    /// A number that uniquely identifies an account.
    /// It’s an incremented value that is assigned to an account the first time it receives funds.
    /// Once assigned, the account number doesn’t change.
    pub number: u64,
    /// The sequence is a value that represents the number of transactions sent from an account.
    /// It is therefore initially set to zero.
    /// The sequence must be included in each transaction and incremented accordingly.
    pub sequence: cosmrs::tx::SequenceNumber,
}

impl AxelarAccount {
    pub async fn new(
        mut query_client: QueryClient<Channel>,
        fixed_seed: [u8; 32],
    ) -> AxelarAccount {
        let sender_private_key = secp256k1::SigningKey::from_slice(&fixed_seed).unwrap();
        let sender_public_key = sender_private_key.public_key();
        let sender_account_id = sender_public_key.account_id("axelar").unwrap();

        let resp = query_client
            .account(QueryAccountRequest {
                address: sender_account_id.to_string(),
            })
            .await
            .map(Response::into_inner)
            .unwrap();

        let account: AxelarAccount;
        if resp.account.is_some() {
            let base_acc = BaseAccount::decode(&resp.account.unwrap().value[..]).unwrap();
            account = AxelarAccount {
                id: sender_account_id,
                private_key: sender_private_key,
                public_key: sender_public_key,
                number: base_acc.account_number,
                sequence: base_acc.sequence,
            }
        } else {
            account = AxelarAccount {
                id: sender_account_id,
                private_key: sender_private_key,
                public_key: sender_public_key,
                number: 0,
                sequence: cosmrs::tx::SequenceNumber::default(), // 0
            }
        }

        account
    }
}
