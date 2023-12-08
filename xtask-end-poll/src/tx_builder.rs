use cosmrs::tx::SignDoc;

use crate::account::AxelarAccount;

pub struct AxelarTx {
    fee: cosmrs::tx::Fee,
    memo: String,
    body: cosmrs::tx::Body,
    timeout_height: tendermint::block::Height,
    /// Describes the public key and signing mode of a single top-level signer.
    signer_info: cosmrs::tx::SignerInfo,
    /// Describes the fee and signer modes that are used to sign a transaction.
    auth_info: cosmrs::tx::AuthInfo,
}

impl AxelarTx {
    pub fn sign(self, account: &AxelarAccount, chain_id: tendermint::chain::Id) -> cosmrs::tx::Raw {
        let sign_doc =
            SignDoc::new(&self.body, &self.auth_info, &chain_id, account.number).unwrap();

        let tx_signed = sign_doc.sign(&account.private_key).unwrap();

        tx_signed
    }
}

pub struct AxelarTxBuilder {
    fee: Option<cosmrs::tx::Fee>,
    memo: Option<String>,
    body: Option<Vec<cosmrs::Any>>,
    timeout_height: Option<tendermint::block::Height>,
    /// Describes the public key and signing mode of a single top-level signer.
    signer_info: cosmrs::tx::SignerInfo,
}

impl AxelarTxBuilder {
    pub fn new(account: &AxelarAccount) -> AxelarTxBuilder {
        AxelarTxBuilder {
            signer_info: cosmrs::tx::SignerInfo::single_direct(
                Some(account.public_key),
                account.sequence,
            ),
            fee: None,
            memo: None,
            body: None,
            timeout_height: None,
        }
    }

    pub fn set_memo(mut self, memo: String) -> AxelarTxBuilder {
        self.memo = Some(memo);
        self
    }

    pub fn set_timeout_height(
        mut self,
        timeout_height: tendermint::block::Height,
    ) -> AxelarTxBuilder {
        self.timeout_height = Some(timeout_height);
        self
    }

    pub fn set_fee(mut self, fee: cosmrs::tx::Fee) -> AxelarTxBuilder {
        self.fee = Some(fee);
        self
    }

    pub fn set_body<I>(mut self, body: I) -> AxelarTxBuilder
    where
        I: IntoIterator<Item = cosmrs::Any>,
    {
        self.body = Some(body.into_iter().map(Into::into).collect());
        self
    }

    pub fn build(self) -> AxelarTx {
        let timeout_height = self
            .timeout_height
            .unwrap_or(tendermint::block::Height::from(0 as u32));
        let memo = self.memo.unwrap_or_default();
        let fee = self.fee.expect("TX fee is required");
        let auth_info = self.signer_info.clone().auth_info(fee.clone());

        AxelarTx {
            fee,
            memo: memo.clone(),
            body: cosmrs::tx::Body::new(
                self.body.expect("TX body is required"),
                memo,
                timeout_height,
            ),
            timeout_height,
            signer_info: self.signer_info,
            auth_info,
        }
    }
}
