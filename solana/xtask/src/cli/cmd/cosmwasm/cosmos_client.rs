pub(crate) mod network {
    use std::time::Duration;

    use cosmrs::abci::GasInfo;
    use cosmrs::auth::BaseAccount;
    use cosmrs::proto::cosmos::auth::v1beta1::{query_client, QueryAccountRequest};
    use cosmrs::proto::cosmos::tx::v1beta1::SimulateRequest;
    use cosmrs::proto::prost::Message;
    use eyre::{ContextCompat, OptionExt};

    #[derive(Debug, Clone)]
    pub(crate) struct Network {
        pub(crate) chain_id: &'static str,
        pub(crate) grpc_endpoint: &'static str,
        pub(crate) rpc_endpoint: &'static str,
    }

    impl Network {
        pub(crate) async fn rpc(&self) -> eyre::Result<cosmrs::rpc::HttpClient> {
            use cosmrs::rpc::Client;
            tracing::debug!("attempting to create a http client");
            let rpc_client = cosmrs::rpc::HttpClient::new(self.rpc_endpoint)?;

            rpc_client
                .wait_until_healthy(Duration::from_secs(5))
                .await
                .expect("error waiting for RPC to return healthy responses");

            tracing::debug!("cosmos rpc client created");

            Ok(rpc_client)
        }
        pub(crate) async fn account(&self, address: &str) -> eyre::Result<BaseAccount> {
            let mut c = query_client::QueryClient::connect(self.grpc_endpoint.to_string()).await?;

            let res = c
                .account(QueryAccountRequest {
                    address: address.into(),
                })
                .await?
                .into_inner()
                .account
                .ok_or_eyre("account query returned None - account might not be intialised")?;

            let account =
                cosmrs::proto::cosmos::auth::v1beta1::BaseAccount::decode(res.value.as_slice())?;

            let res = BaseAccount::try_from(account)?;
            Ok(res)
        }

        pub(crate) async fn simulate(&self, tx_bytes: Vec<u8>) -> eyre::Result<GasInfo> {
            let mut c = cosmrs::proto::cosmos::tx::v1beta1::service_client::ServiceClient::connect(
                self.grpc_endpoint,
            )
            .await?;

            let res = c
                .simulate(
                    #[allow(deprecated)]
                    SimulateRequest { tx: None, tx_bytes },
                )
                .await?
                .into_inner()
                .gas_info;

            let gas_info = res.with_context(|| "Unable to extract gas info")?;

            gas_info
                .try_into()
                .map_err(|e: cosmrs::ErrorReport| eyre::eyre!(e))
        }
    }
}

pub(crate) mod signer {

    use cosmrs::auth::BaseAccount;
    use cosmrs::crypto::secp256k1::SigningKey;
    use cosmrs::rpc::{self};
    use cosmrs::tx::{Fee, SignDoc, SignerInfo};
    use cosmrs::{tx, AccountId, Any, Coin};

    use super::gas::Gas;
    use super::network::Network;

    pub(crate) struct SigningClient {
        pub(crate) network: Network,
        pub(crate) account_prefix: String,
        pub(crate) signing_key: SigningKey,
    }

    impl SigningClient {
        pub(crate) fn signer_account_id(&self) -> eyre::Result<AccountId> {
            let signer_pub = self.signing_key.public_key();
            signer_pub.account_id(self.account_prefix.as_str())
        }

        async fn estimate_fee(
            &self,
            gas: Gas,
            account: &BaseAccount,
            tx_body: &tx::Body,
        ) -> eyre::Result<Fee> {
            const GAS_LIMIT: u64 = 0;
            const AMOUNT: u128 = 0;
            let tx_raw = self.sign_tx(
                account,
                Fee::from_amount_and_gas(
                    Coin {
                        denom: gas.gas_price.denom.clone(),
                        amount: AMOUNT,
                    },
                    GAS_LIMIT,
                ),
                tx_body,
            )?;
            let gas_info = &self.network.simulate(tx_raw.to_bytes()?).await?;
            tracing::debug!(?gas_info, "simulated gas");
            Ok(gas.fee_from_gas_simulation(gas_info))
        }

        fn sign_tx(
            &self,
            account: &BaseAccount,
            fee: Fee,
            tx_body: &tx::Body,
        ) -> Result<tx::Raw, eyre::Error> {
            let signer_info =
                SignerInfo::single_direct(Some(self.signing_key.public_key()), account.sequence);
            let auth_info = signer_info.auth_info(fee);
            let sign_doc = SignDoc::new(
                tx_body,
                &auth_info,
                &self.network.chain_id.parse()?,
                account.account_number,
            )?;
            let tx_raw = sign_doc.sign(&self.signing_key)?;
            Ok(tx_raw)
        }

        pub(crate) async fn sign_and_broadcast(
            &self,
            msgs: Vec<Any>,
            gas: &Gas,
        ) -> eyre::Result<rpc::endpoint::broadcast::tx_commit::Response> {
            const TIMEOUT_HEIGHT: u8 = 0;

            let acc = self
                .network
                .account(self.signer_account_id()?.as_ref())
                .await?;
            tracing::info!(acc = ?acc.address, "found account");

            let tx_body = tx::Body::new(msgs, "", TIMEOUT_HEIGHT);
            let fee = self.estimate_fee(gas.clone(), &acc, &tx_body).await?;

            let tx_raw = self.sign_tx(&acc, fee, &tx_body)?;
            tracing::debug!("tx signed");

            let rpc_client = self.network.rpc().await?;
            let tx_commit_response = tx_raw.broadcast_commit(&rpc_client).await?;
            tracing::debug!("tx broadcasted");

            tracing::info!(hash = ?tx_commit_response.hash, tx_result = ?tx_commit_response.tx_result.log, "raw respones log");
            Ok(tx_commit_response)
        }

        pub(crate) async fn query<T: serde::de::DeserializeOwned>(
            &self,
            address: AccountId,
            query_data: Vec<u8>,
        ) -> eyre::Result<T> {
            use cosmrs::proto::cosmwasm::wasm::v1::query_client;

            let mut c = query_client::QueryClient::connect(self.network.grpc_endpoint).await?;

            let res = c
                .smart_contract_state(
                    cosmrs::proto::cosmwasm::wasm::v1::QuerySmartContractStateRequest {
                        address: address.to_string(),
                        query_data,
                    },
                )
                .await?
                .into_inner()
                .data;

            let result = serde_json::from_slice::<T>(res.as_ref())?;

            Ok(result)
        }
    }
}

pub(crate) mod gas {
    use cosmrs::tx::Fee;
    use cosmrs::{Coin, Denom};
    use rust_decimal::prelude::*;

    #[derive(Debug, Clone)]
    pub(crate) struct GasPrice {
        pub(crate) amount: Decimal,
        pub(crate) denom: Denom,
    }

    #[derive(Debug, Clone)]
    pub(crate) struct Gas {
        pub(crate) gas_price: GasPrice,
        pub(crate) gas_adjustment: Decimal,
    }

    impl Gas {
        pub(crate) fn fee_from_gas_simulation(&self, gas_info: &cosmrs::abci::GasInfo) -> Fee {
            let gas_limit = Decimal::from_u64(gas_info.gas_used).expect("always succeeds");
            let gas_limit = gas_limit.saturating_mul(self.gas_adjustment).ceil();

            let amount = Coin {
                denom: self.gas_price.denom.clone(),
                amount: gas_limit
                    .saturating_mul(self.gas_price.amount)
                    .ceil()
                    .to_u128()
                    .expect("the multiplication resulted in a negative number"),
            };

            Fee::from_amount_and_gas(
                amount,
                gas_limit
                    .to_u64()
                    .expect("gas limit does not map into a valid u64"),
            )
        }
    }
}
