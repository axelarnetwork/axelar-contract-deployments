use std::str::FromStr;

use solana_sdk::signature::Signature;
use sqlx::pool::Pool;
use sqlx::prelude::FromRow;
use sqlx::Postgres;
use tiny_keccak::{Hasher, Keccak};
use tracing::{debug, info};
use url::Url;

type Result<T> = sqlx::Result<T>;

const SINGLETON_ID: i32 = 1;

#[derive(Clone)]
pub struct State {
    pool: Pool<Postgres>,
}

#[allow(dead_code)]
impl State {
    pub async fn from_url(database_url: Url) -> Result<Self> {
        Pool::connect(database_url.as_str())
            .await
            .map(|pool| Self { pool })
    }

    pub async fn migrate(&self) -> anyhow::Result<()> {
        info!("Running database migrations");
        Ok(sqlx::migrate!().run(&self.pool).await?)
    }

    //
    // Axelar Block
    //

    pub async fn update_axelar_block_height(&self, block_height: i64) -> Result<()> {
        sqlx::query(
            "INSERT INTO axelar_block (id, latest_block, updated_at) \
             VALUES ($1, $2, CURRENT_TIMESTAMP) \
             ON CONFLICT (id) DO UPDATE \
             SET latest_block = EXCLUDED.latest_block, updated_at = EXCLUDED.updated_at;",
        )
        .bind(SINGLETON_ID)
        .bind(block_height)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_axelar_block_height(&self) -> Result<i64> {
        let (block,): (i64,) = sqlx::query_as(
            "SELECT latest_block \
             FROM axelar_block \
             WHERE id = $1;",
        )
        .bind(SINGLETON_ID)
        .fetch_one(&self.pool)
        .await?;
        Ok(block)
    }

    //
    // Solana Transaction
    //

    #[tracing::instrument(skip(self), err)]
    pub async fn update_solana_transaction(&self, signature: Signature) -> Result<()> {
        debug!("updating solana_transaction table");
        let signature = signature.to_string();
        sqlx::query(
            "INSERT INTO solana_transaction (id, latest_signature, updated_at) \
                     VALUES ($1, $2, CURRENT_TIMESTAMP) \
                     ON CONFLICT (id) DO UPDATE \
                     SET signature = EXCLUDED.signature, updated_at = EXCLUDED.updated_at;",
        )
        .bind(SINGLETON_ID)
        .bind(signature)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    #[tracing::instrument(skip(self), err, ret)]
    pub async fn get_solana_transaction(&self) -> Result<Option<Signature>> {
        let optional_signature: Option<(String,)> = sqlx::query_as("")
            .bind(SINGLETON_ID)
            .fetch_optional(&self.pool)
            .await?;

        optional_signature
            .map(|(text,)| Signature::from_str(&text))
            .transpose()
            .map_err(|parse_error| sqlx::Error::Decode(Box::new(parse_error)))
    }

    //
    // Axelar Messages
    //

    pub async fn insert_axelar_message(
        &self,
        solana_transaction_signature: &str,
        source_address: &str,
        destination_address: &str,
        destination_chain: &str,
        payload: &[u8],
        ccid: &str,
    ) -> Result<()> {
        let (solana_transaction_id,): (i32,) =
            sqlx::query_as("SELECT id FROM solana_transactions WHERE signature = $1")
                .bind(solana_transaction_signature)
                .fetch_one(&self.pool)
                .await?;
        let payload_hash = keccak(payload);
        sqlx::query(
            "INSERT INTO axelar_messages \
             (solana_transaction_id, source_address, destination_address, destination_chain, payload, payload_hash, ccid) \
             VALUES ($1, $2, $3, $4, $5, $6, $7);")
            .bind(solana_transaction_id)
            .bind(source_address)
            .bind(destination_address)
            .bind(destination_chain)
            .bind(payload)
            .bind(payload_hash)
            .bind(ccid)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn get_pending_axelar_messages(&self) -> Result<Vec<AxelarMessageRow>> {
        sqlx::query_as(
            "SELECT id, source_address, destination_address, destination_chain, payload, payload_hash, ccid \
             FROM axelar_messages \
             WHERE status = 'pending'"
        ).fetch_all(&self.pool).await
    }

    pub async fn mark_axelar_message_submitted(&self, axelar_message_id: i32) -> Result<()> {
        sqlx::query(
            "UPDATE axelar_messages \
             SET status = 'submitted' \
             WHERE id = $1;",
        )
        .bind(axelar_message_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}

#[derive(FromRow)]
#[allow(dead_code)]
pub struct AxelarMessageRow {
    pub id: i32,
    pub source_address: String,
    pub destination_address: String,
    pub destination_chain: String,
    pub payload: Vec<u8>,
    pub payload_hash: [u8; 32],
    pub ccid: Option<String>,
}

fn keccak(data: &[u8]) -> [u8; 32] {
    let mut output = [0; 32];
    let mut keccak = Keccak::v256();
    keccak.update(data);
    keccak.finalize(&mut output);
    output
}
