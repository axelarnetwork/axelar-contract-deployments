use std::error::Error;

use crate::entities::{
    last_processed_block::{self, Chain, Column},
    prelude::LastProcessedBlock,
};
use sea_orm::{sea_query, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter};

#[derive(Clone)]
pub struct PostgresStateTracker {
    connection: DatabaseConnection,
}

impl PostgresStateTracker {
    pub fn new(connection: DatabaseConnection) -> Self {
        Self { connection }
    }

    pub async fn load(&self) -> Result<Option<u64>, Box<dyn Error + Send + Sync>> {
        LastProcessedBlock::find()
            .filter(Column::Chain.eq(Chain::Axelar))
            .one(&self.connection)
            .await
            .map(|query_result| query_result.map(|model| model.height))
            .map_err(|error| Box::new(error) as Box<dyn Error + Send + Sync>)
    }

    pub async fn save(
        &self,
        chain: Self::ChainId,
        height: u64,
    ) -> Result<(), Box<dyn Error + Send + Sync>> {
        let model = last_processed_block::ActiveModel {
            height: ActiveValue::Set(height),
            chain: ActiveValue::Set(chain),
        };
        last_processed_block::Entity::insert(model)
            .on_conflict(
                sea_query::OnConflict::column(last_processed_block::Column::Chain)
                    .update_column(last_processed_block::Column::Height)
                    .to_owned(),
            )
            .exec(&self.connection)
            .await?;
        Ok(())
    }
}
