use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create the tower_sessions schema if it doesn't exist (PostgreSQL specific)
        if manager.get_database_backend() == sea_orm::DbBackend::Postgres {
            let create_schema_query = r#"CREATE SCHEMA IF NOT EXISTS "tower_sessions""#;
            
            // Concurrent create schema may fail due to duplicate key violations.
            // This works around that by assuming the schema must exist on such an error.
            if let Err(err) = manager.get_connection().execute_unprepared(create_schema_query).await {
                if !err.to_string().contains("duplicate key value violates unique constraint") {
                    return Err(err);
                }
            }
        }

        // Create the session table in the tower_sessions schema
        manager
            .create_table(
                Table::create()
                    .table((Alias::new("tower_sessions"), Session::Table))
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Session::Id)
                            .text()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(Session::Data)
                            .binary()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(Session::ExpiryDate)
                            .timestamp_with_time_zone()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        // Create an index on expiry_date for efficient cleanup
        manager
            .create_index(
                Index::create()
                    .if_not_exists()
                    .name("idx-session-expiry_date")
                    .table((Alias::new("tower_sessions"), Session::Table))
                    .col(Session::ExpiryDate)
                    .to_owned(),
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop the index first
        manager
            .drop_index(Index::drop().name("idx-session-expiry_date").to_owned())
            .await?;

        // Drop the session table from the tower_sessions schema
        manager
            .drop_table(Table::drop().table((Alias::new("tower_sessions"), Session::Table)).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum Session {
    Table,
    Id,
    Data,
    ExpiryDate,
}