//! Session entity model for Sea-ORM database interaction.
//!
//! This module defines the database schema representation for session storage.
//! It provides the Sea-ORM entity definition that maps to the "session" table
//! in the configured schema.

use sea_orm::entity::prelude::*;

/// Sea-ORM entity model representing a session in the database.
///
/// This model defines the structure of the database table used to store sessions.
/// It maps to the "session" table in the configured schema (by default "tower_sessions")
/// and is used by the `PostgresStore` to interact with the database.
///
/// # Database Schema
///
/// The database schema for this model contains the following columns:
///
/// | Column      | Type                    | Description                       |
/// |-------------|-------------------------|-----------------------------------|
/// | id          | TEXT (Primary Key)      | Session ID                        |
/// | data        | BYTEA                   | Serialized session data           |
/// | expiry_date | TIMESTAMPTZ             | Session expiration timestamp      |
///
/// # Usage
///
/// This entity is primarily used internally by the `PostgresStore` implementation
/// and you typically won't need to interact with it directly when using this crate.
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
#[sea_orm(table_name = "session", schema_name = "tower_sessions")]
pub struct Model {
    /// The unique session identifier stored as a string.
    ///
    /// This field serves as the primary key for the table and is typically a
    /// UUID or a similarly unique value. It corresponds to the `tower_sessions::Id`
    /// type which is converted to a string for database storage.
    #[sea_orm(primary_key, column_type = "Text")]
    pub id: String,

    /// The serialized session data stored as a binary blob.
    ///
    /// This field contains the MessagePack-serialized representation of the
    /// session record, including all user-specific session data. It's stored
    /// as a binary array (`BYTEA` in PostgreSQL) for efficient storage.
    pub data: Vec<u8>,

    /// The session expiration timestamp with timezone information.
    ///
    /// This field determines when the session becomes invalid. The `PostgresStore`
    /// implementation uses this field to:
    /// 1. Filter out expired sessions when loading
    /// 2. Automatically delete expired sessions during cleanup
    ///
    /// It's stored as a `TIMESTAMPTZ` in PostgreSQL.
    pub expiry_date: DateTimeWithTimeZone,
}

/// Required enum for Sea-ORM entity relations.
///
/// This entity doesn't have any relations to other entities, so this enum is empty.
#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

/// Default behavior implementation for session entity active models.
///
/// This implementation uses the default behavior for active model operations.
impl ActiveModelBehavior for ActiveModel {}
