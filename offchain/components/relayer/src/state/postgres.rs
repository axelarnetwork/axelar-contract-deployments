use std::str::FromStr;

use solana_sdk::signature::Signature;
use sqlx::pool::Pool;
use sqlx::Postgres;
use tracing::{debug, info};
use url::Url;

use super::interface::State as StateTrait;

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

    /// Updates the stored Axelar block height if the new value is greater.
    pub async fn update_axelar_block_height(&self, block_height: i64) -> Result<()> {
        sqlx::query!(
            "INSERT INTO axelar_block (id, latest_block, updated_at) \
             VALUES ($1, $2, CURRENT_TIMESTAMP) \
             ON CONFLICT (id) DO UPDATE \
             SET latest_block = EXCLUDED.latest_block, updated_at = EXCLUDED.updated_at \
             WHERE EXCLUDED.latest_block > axelar_block.latest_block;",
            SINGLETON_ID,
            block_height
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn get_axelar_block_height(&self) -> Result<i64> {
        sqlx::query!(
            "SELECT latest_block \
             FROM axelar_block \
             WHERE id = $1;",
            SINGLETON_ID
        )
        .fetch_one(&self.pool)
        .await
        .map(|record| record.latest_block)
    }

    //
    // Solana Transaction
    //

    #[tracing::instrument(skip(self), err)]
    pub async fn update_solana_transaction(&self, signature: Signature) -> Result<()> {
        debug!("updating solana_transaction table");
        let signature = signature.to_string();
        sqlx::query!(
            "INSERT INTO solana_transaction (id, latest_signature, updated_at) \
                     VALUES ($1, $2, CURRENT_TIMESTAMP) \
                     ON CONFLICT (id) DO UPDATE \
                     SET latest_signature = EXCLUDED.latest_signature, updated_at = EXCLUDED.updated_at;",
            SINGLETON_ID,
            signature
        )
        .execute(&self.pool).await?;
        Ok(())
    }

    #[tracing::instrument(skip(self), err, ret)]
    pub async fn get_solana_transaction(&self) -> Result<Option<Signature>> {
        let optional_signature = sqlx::query!(
            "SELECT latest_signature from solana_transaction where id = $1;",
            SINGLETON_ID
        )
        .fetch_optional(&self.pool)
        .await?;

        optional_signature
            .map(|record| Signature::from_str(&record.latest_signature))
            .transpose()
            .map_err(|parse_error| sqlx::Error::Decode(Box::new(parse_error)))
    }
}

impl StateTrait<Signature> for State {
    type Error = sqlx::Error;

    async fn get(&self) -> Result<Option<Signature>> {
        self.get_solana_transaction().await
    }

    async fn set(&self, signature: Signature) -> Result<()> {
        self.update_solana_transaction(signature).await
    }
}
