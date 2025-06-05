pub use sea_orm_migration::prelude::*;

mod m20240101_000001_create_session_table;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    // Override the name of migration table to avoid conflicts
    fn migration_table_name() -> sea_orm::DynIden {
        Alias::new("tower_sessions_seaorm_migrations").into_iden()
    }

    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20240101_000001_create_session_table::Migration),
        ]
    }
}