pub use sea_orm_migration::prelude::*;

mod m20240208_151704_create_last_processed_blocks;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![Box::new(
            m20240208_151704_create_last_processed_blocks::Migration,
        )]
    }
}
