//! # Tower Sessions Store for Sea-ORM with PostgreSQL
//!
//! A robust and efficient session store implementation for [`tower-sessions`](https://crates.io/crates/tower-sessions)
//! using [Sea-ORM](https://crates.io/crates/sea-orm) as the database abstraction layer.
//!
//! This crate provides a PostgreSQL backend implementation for session storage in web applications
//! built with Tower-compatible frameworks like Axum, Hyper, or any Tower-based service.
//!
//! ## Features
//!
//! - Persistent session storage in PostgreSQL databases
//! - Sea-ORM integration for type-safe database operations
//! - Automatic session expiration and cleanup
//! - Automatic database migration support (with `migration` feature)
//! - Optimized upsert operations for better performance
//! - Comprehensive error handling with dedicated error types
//! - Serialization of session data using MessagePack for compact storage
//!
//! ## Quick Start
//!
//! ```no_run
//! use sea_orm::{Database, DbConn};
//! use time::Duration;
//! use tower_sessions::Expiry;
//! use tower_sessions_seaorm_store::PostgresStore;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Connect to the database
//! let conn = Database::connect("postgres://postgres:postgres@localhost:5432/sessions").await?;
//!
//! // Create a new PostgresStore
//! let store = PostgresStore::new(conn);
//!
//! // Run migrations to set up the database schema (requires "migration" feature)
//! store.migrate().await?;
//!
//! // Use the store with tower-sessions
//! let session_layer = tower_sessions::SessionManagerLayer::new(store)
//!     .with_expiry(Expiry::OnInactivity(Duration::days(7)));
//! # Ok(())
//! # }
//! ```
//!
//! ## Axum Integration Example
//!
//! ```no_run
//! use axum::{Router, routing::get};
//! use sea_orm::{Database, DbConn};
//! use time::Duration;
//! use tower_sessions::{Expiry, Session, SessionManagerLayer};
//! use tower_sessions_seaorm_store::PostgresStore;
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! // Connect to the database
//! let conn = Database::connect("postgres://postgres:postgres@localhost:5432/sessions").await?;
//!
//! // Create the store
//! let store = PostgresStore::new(conn);
//!
//! // Run migrations to set up the database schema
//! store.migrate().await?;
//!
//! // Configure session layer
//! let session_layer = SessionManagerLayer::new(store)
//!     .with_expiry(Expiry::OnInactivity(Duration::days(1)))
//!     .with_secure(true);
//!
//! // Build your Axum application with session support
//! let app = Router::new()
//!     .route("/", get(|| async { "Hello, world!" }))
//!     .layer(session_layer);
//! # Ok(())
//! # }
//! ```
//!
//! ## Session Management
//!
//! Once your application is set up with the session layer, you can use the session in your handlers:
//!
//! ```no_run
//! use axum::extract::State;
//! use tower_sessions::Session;
//!
//! # async fn example(session: Session) -> Result<String, &'static str> {
//! // Set a value
//! session.insert("user_id", 123).await.map_err(|_| "Failed to insert")?;
//!
//! // Get a value
//! let user_id: Option<u32> = session.get("user_id").await.map_err(|_| "Failed to get")?;
//!
//! // Remove a value
//! session.remove("user_id").await.map_err(|_| "Failed to remove")?;
//!
//! // Clear the entire session
//! session.flush().await.map_err(|_| "Failed to flush")?;
//! # Ok("Success".to_string())
//! # }
//! ```

pub mod entity;
#[cfg(feature = "migration")]
pub mod migration;
mod postgres_store;

pub use sea_orm;

/// An error type for SeaORM stores.
#[derive(thiserror::Error, Debug)]
pub enum SeaOrmStoreError {
    /// A variant to map Sea-ORM errors.
    #[error(transparent)]
    SeaOrm(#[from] sea_orm::DbErr),

    /// A variant to map `rmp_serde` encode errors.
    #[error(transparent)]
    Encode(#[from] rmp_serde::encode::Error),

    /// A variant to map `rmp_serde` decode errors.
    #[error(transparent)]
    Decode(#[from] rmp_serde::decode::Error),
}

impl From<SeaOrmStoreError> for tower_sessions::session_store::Error {
    fn from(err: SeaOrmStoreError) -> Self {
        match err {
            SeaOrmStoreError::SeaOrm(inner) => tower_sessions::session_store::Error::Backend(inner.to_string()),
            SeaOrmStoreError::Decode(inner) => tower_sessions::session_store::Error::Decode(inner.to_string()),
            SeaOrmStoreError::Encode(inner) => tower_sessions::session_store::Error::Encode(inner.to_string()),
        }
    }
}

// Re-export our PostgreSQL store implementation
/// The main PostgreSQL store implementation for tower-sessions
///
/// This is the primary type you'll use from this crate.
/// See [`PostgresStore`] documentation for usage details.
pub use postgres_store::PostgresStore;

// Re-export necessary types from tower-sessions for convenience
/// Session storage error types and results
///
/// These are re-exported from the `tower-sessions` crate for convenience.
pub use tower_sessions::session_store;

/// Trait for implementing session store expiration cleanup
///
/// Implementation provided by `PostgresStore` allows for efficient deletion of
/// expired sessions from the database.
pub use tower_sessions::ExpiredDeletion;

/// Session identifier type
///
/// Re-exported from `tower-sessions` for convenience.
pub use tower_sessions::session::Id;

/// Session record type
///
/// Contains the session data and metadata that gets stored in the database.
pub use tower_sessions::session::Record;

/// Session type for manipulating the current session
///
/// This is the type you'll use in your request handlers to access session data.
pub use tower_sessions::Session;

/// Trait for implementing session storage backends
///
/// Implemented by `PostgresStore` to provide the required storage functionality.
pub use tower_sessions::SessionStore;

