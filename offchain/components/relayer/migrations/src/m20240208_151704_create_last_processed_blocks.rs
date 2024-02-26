use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(LastProcessedBlock::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(LastProcessedBlock::Height)
                            .big_unsigned()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(LastProcessedBlock::Chain)
                            .string()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                sea_query::Index::create()
                    .table(LastProcessedBlock::Table)
                    .name("chain_index")
                    .col(LastProcessedBlock::Chain)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(Table::drop().table(LastProcessedBlock::Table).to_owned())
            .await
    }
}

#[derive(DeriveIden)]
enum LastProcessedBlock {
    Table,
    Height,
    Chain,
}
