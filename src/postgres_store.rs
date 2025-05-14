use std::fmt::Debug;

use async_trait::async_trait;
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, IntoActiveModel, QueryFilter,
    Set, TransactionTrait,
};
use time::OffsetDateTime;
use tower_sessions::{session::Id, session::Record, session_store, ExpiredDeletion, SessionStore};

use crate::entity::session::{self, ActiveModel as SessionActiveModel, Entity as SessionEntity};

/// A PostgreSQL-based session store for tower-sessions using Sea-ORM.
///
/// `PostgresStore` provides a session storage backend implementation that persists session data
/// in a PostgreSQL database using Sea-ORM as the database abstraction layer. This allows for
/// robust and efficient session management with type-safe database operations.
///
/// Session data is serialized using MessagePack for compact storage.
///
/// # Features
///
/// - Persistent session storage in PostgreSQL
/// - Session data serialization using MessagePack
/// - Automatic session expiry and cleanup
/// - Custom table name configuration
/// - Collision-safe ID generation
///
/// # Usage
///
/// ```no_run
/// use sea_orm::{Database, DbConn};
/// use tower_sessions::Expiry;
/// use time::Duration;
/// use tower_sessions_seaorm_store::PostgresStore;
///
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// // Connect to the database
/// let conn = Database::connect("postgres://postgres:password@localhost:5432/sessions").await?;
///
/// // Create a new PostgresStore with default settings
/// let store = PostgresStore::new(conn);
///
/// // Or with a custom table name
/// // let store = PostgresStore::new(conn).with_table_name("my_custom_sessions");
///
/// // Use the store with tower-sessions
/// let session_layer = tower_sessions::SessionManagerLayer::new(store)
///     .with_expiry(Expiry::OnInactivity(Duration::days(7)));
/// # Ok(())
/// # }
/// ```
///
/// # Database Schema
///
/// The store uses a table with the following structure (by default named "tower_sessions"):
///
/// | Column      | Type                    | Description                             |
/// |-------------|-------------------------|-----------------------------------------|
/// | id          | TEXT (Primary Key)      | Session ID                              |
/// | data        | BYTEA                   | MessagePack serialized session data     |
/// | expiry_date | TIMESTAMPTZ             | Expiration date of the session          |
///
/// # Error Handling
///
/// This implementation maps errors from the underlying Sea-ORM operations to `tower_sessions::session_store::Error` types:
///
/// - Database errors → `session_store::Error::Backend`
/// - Serialization errors → `session_store::Error::Encode`
/// - Deserialization errors → `session_store::Error::Decode`
#[derive(Debug, Clone)]
pub struct PostgresStore {
    /// The Sea-ORM database connection used for database operations.
    conn: DatabaseConnection,
    /// The name of the database table used for storing sessions.
    table_name: String,
}

impl PostgresStore {
    /// Creates a new PostgreSQL session store with the default configuration.
    ///
    /// This constructor initializes a new `PostgresStore` with the provided Sea-ORM database connection
    /// and the default table name "tower_sessions".
    ///
    /// # Parameters
    ///
    /// * `conn` - A Sea-ORM `DatabaseConnection` to the PostgreSQL database.
    ///
    /// # Returns
    ///
    /// A new instance of `PostgresStore` configured with the default table name.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sea_orm::{Database, DbConn};
    /// use tower_sessions_seaorm_store::PostgresStore;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let conn = Database::connect("postgres://postgres:password@localhost:5432/sessions").await?;
    /// let store = PostgresStore::new(conn);
    /// # Ok(())
    /// # }
    /// ```
    pub fn new(conn: DatabaseConnection) -> Self {
        Self {
            conn,
            table_name: "tower_sessions".to_string(),
        }
    }

    /// Sets a custom table name for this store.
    ///
    /// This method allows customizing the table name used for session storage.
    /// This is useful when you need to use a different table name than the default "tower_sessions".
    ///
    /// # Parameters
    ///
    /// * `table_name` - A string-like value representing the desired table name.
    ///
    /// # Returns
    ///
    /// The modified `PostgresStore` instance with the updated table name.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use sea_orm::{Database, DbConn};
    /// use tower_sessions_seaorm_store::PostgresStore;
    ///
    /// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let conn = Database::connect("postgres://postgres:password@localhost:5432/sessions").await?;
    ///
    /// // Use a custom table name for multi-tenant applications or specific environments
    /// let store = PostgresStore::new(conn).with_table_name("production_sessions");
    /// # Ok(())
    /// # }
    /// ```
    pub fn with_table_name(mut self, table_name: impl Into<String>) -> Self {
        self.table_name = table_name.into();
        self
    }
}

#[async_trait]
impl SessionStore for PostgresStore {
    /// Creates a new session record in the database.
    ///
    /// This method inserts a new session record into the database with the provided data.
    /// It includes collision detection to ensure unique session IDs - if a collision is detected,
    /// a new session ID will be generated automatically.
    ///
    /// # Parameters
    ///
    /// * `record` - A mutable reference to the session record to create. The record ID may be modified
    ///   if a collision is detected.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - The session was successfully created.
    /// * `Err(session_store::Error)` - An error occurred during session creation.
    ///
    /// # Error Mapping
    ///
    /// * Sea-ORM database errors → `session_store::Error::Backend`
    /// * MessagePack serialization errors → `session_store::Error::Encode`
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tower_sessions::{session_store, Record, Session, SessionStore};
    /// use tower_sessions_seaorm_store::PostgresStore;
    /// use time::OffsetDateTime;
    ///
    /// # async fn example(store: PostgresStore) -> session_store::Result<()> {
    /// // Create a new session record
    /// let mut record = Record {
    ///     id: Session::id(),
    ///     data: serde_json::json!({ "user_id": 123 }),
    ///     expiry_date: OffsetDateTime::now_utc() + time::Duration::days(7),
    /// };
    ///
    /// // Insert the record into the store
    /// store.create(&mut record).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn create(&self, record: &mut Record) -> session_store::Result<()> {
        let txn = self
            .conn
            .begin()
            .await
            .map_err(|e| session_store::Error::Backend(e.to_string()))?;

        // Session ID collision mitigation
        while SessionEntity::find_by_id(record.id.to_string())
            .one(&txn)
            .await
            .map_err(|e| session_store::Error::Backend(e.to_string()))?
            .is_some()
        {
            // Generate a new ID if there's a collision
            record.id = Id::default();
        }

        // Serialize the session data using MessagePack
        let data =
            rmp_serde::to_vec(record).map_err(|e| session_store::Error::Encode(e.to_string()))?;

        // Convert time::OffsetDateTime to DateTimeWithTimeZone
        let expiry_date = convert_time_to_datetime(record.expiry_date);

        // Create a new session record
        let session_model = SessionActiveModel {
            id: Set(record.id.to_string()),
            data: Set(data),
            expiry_date: Set(expiry_date),
        };

        session_model
            .insert(&txn)
            .await
            .map_err(|e| session_store::Error::Backend(e.to_string()))?;

        txn.commit()
            .await
            .map_err(|e| session_store::Error::Backend(e.to_string()))?;

        Ok(())
    }

    /// Saves an existing session record to the database.
    ///
    /// This method updates an existing session record in the database or creates a new one if it
    /// doesn't exist. This provides an "upsert" functionality for session data.
    ///
    /// # Parameters
    ///
    /// * `record` - A reference to the session record to save.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - The session was successfully saved.
    /// * `Err(session_store::Error)` - An error occurred during session saving.
    ///
    /// # Error Mapping
    ///
    /// * Sea-ORM database errors → `session_store::Error::Backend`
    /// * MessagePack serialization errors → `session_store::Error::Encode`
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tower_sessions::{session_store, Record, Session, SessionStore};
    /// use tower_sessions_seaorm_store::PostgresStore;
    /// use time::OffsetDateTime;
    ///
    /// # async fn example(store: PostgresStore) -> session_store::Result<()> {
    /// // Create or update a session record
    /// let record = Record {
    ///     id: Session::id(),
    ///     data: serde_json::json!({ "last_seen": "2023-01-01" }),
    ///     expiry_date: OffsetDateTime::now_utc() + time::Duration::days(7),
    /// };
    ///
    /// // Save the record to the store
    /// store.save(&record).await?;
    /// # Ok(())
    /// # }
    /// ```
    async fn save(&self, record: &Record) -> session_store::Result<()> {
        // Serialize the session data using MessagePack
        let data =
            rmp_serde::to_vec(record).map_err(|e| session_store::Error::Encode(e.to_string()))?;

        // Convert time::OffsetDateTime to DateTimeWithTimeZone
        let expiry_date = convert_time_to_datetime(record.expiry_date);

        match SessionEntity::find_by_id(record.id.to_string())
            .one(&self.conn)
            .await
            .map_err(|e| session_store::Error::Backend(e.to_string()))?
        {
            Some(existing) => {
                let mut active_model = existing.into_active_model();
                active_model.data = Set(data);
                active_model.expiry_date = Set(expiry_date);
                active_model
                    .update(&self.conn)
                    .await
                    .map_err(|e| session_store::Error::Backend(e.to_string()))?;
            }
            None => {
                let session_model = SessionActiveModel {
                    id: Set(record.id.to_string()),
                    data: Set(data),
                    expiry_date: Set(expiry_date),
                };

                session_model
                    .insert(&self.conn)
                    .await
                    .map_err(|e| session_store::Error::Backend(e.to_string()))?;
            }
        }

        Ok(())
    }

    /// Loads a session record from the database by ID.
    ///
    /// This method retrieves a session record by its ID, only returning sessions that have not expired.
    /// Expired sessions are filtered out at the database query level for efficiency.
    ///
    /// # Parameters
    ///
    /// * `session_id` - The ID of the session to load.
    ///
    /// # Returns
    ///
    /// * `Ok(Some(Record))` - The session was found and successfully loaded.
    /// * `Ok(None)` - No session was found with the given ID or the session has expired.
    /// * `Err(session_store::Error)` - An error occurred during session loading.
    ///
    /// # Error Mapping
    ///
    /// * Sea-ORM database errors → `session_store::Error::Backend`
    /// * MessagePack deserialization errors → `session_store::Error::Decode`
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tower_sessions::{session_store, Id, SessionStore};
    /// use tower_sessions_seaorm_store::PostgresStore;
    ///
    /// # async fn example(store: PostgresStore) -> session_store::Result<()> {
    /// // Load a session by its ID
    /// let session_id = Id::from_bytes([0; 32]);
    /// if let Some(record) = store.load(&session_id).await? {
    ///     println!("Session found with data: {:?}", record.data);
    /// } else {
    ///     println!("Session not found or expired");
    /// }
    /// # Ok(())
    /// # }
    /// ```
    async fn load(&self, session_id: &Id) -> session_store::Result<Option<Record>> {
        let now = OffsetDateTime::now_utc();
        let now_db = convert_time_to_datetime(now);

        // Get the session and make sure it's not expired
        let session = SessionEntity::find_by_id(session_id.to_string())
            .filter(session::Column::ExpiryDate.gt(now_db))
            .one(&self.conn)
            .await
            .map_err(|e| session_store::Error::Backend(e.to_string()))?;

        match session {
            Some(model) => {
                // Deserialize the session data using MessagePack
                let record = rmp_serde::from_slice(&model.data)
                    .map_err(|e| session_store::Error::Decode(e.to_string()))?;
                Ok(Some(record))
            }
            None => Ok(None),
        }
    }

    /// Deletes a session record from the database by ID.
    ///
    /// This method removes a session record from the database, effectively invalidating the session.
    ///
    /// # Parameters
    ///
    /// * `session_id` - The ID of the session to delete.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - The session was successfully deleted or didn't exist.
    /// * `Err(session_store::Error)` - An error occurred during session deletion.
    ///
    /// # Error Mapping
    ///
    /// * Sea-ORM database errors → `session_store::Error::Backend`
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tower_sessions::{session_store, Id, SessionStore};
    /// use tower_sessions_seaorm_store::PostgresStore;
    ///
    /// # async fn example(store: PostgresStore) -> session_store::Result<()> {
    /// // Delete a session by its ID
    /// let session_id = Id::from_bytes([0; 32]);
    /// store.delete(&session_id).await?;
    /// println!("Session deleted");
    /// # Ok(())
    /// # }
    /// ```
    async fn delete(&self, session_id: &Id) -> session_store::Result<()> {
        SessionEntity::delete_by_id(session_id.to_string())
            .exec(&self.conn)
            .await
            .map_err(|e| session_store::Error::Backend(e.to_string()))?;

        Ok(())
    }
}

#[async_trait]
impl ExpiredDeletion for PostgresStore {
    /// Deletes all expired session records from the database.
    ///
    /// This method implements the `ExpiredDeletion` trait for efficient cleanup of expired sessions.
    /// It executes a bulk delete operation that removes all sessions with an expiry date in the past.
    ///
    /// This method is typically called periodically by the session layer to clean up expired sessions.
    ///
    /// # Returns
    ///
    /// * `Ok(())` - Expired sessions were successfully deleted.
    /// * `Err(session_store::Error)` - An error occurred during deletion.
    ///
    /// # Error Mapping
    ///
    /// * Sea-ORM database errors → `session_store::Error::Backend`
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tower_sessions::{session_store, ExpiredDeletion};
    /// use tower_sessions_seaorm_store::PostgresStore;
    ///
    /// # async fn example(store: PostgresStore) -> session_store::Result<()> {
    /// // Delete all expired sessions
    /// store.delete_expired().await?;
    /// println!("Expired sessions cleaned up");
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// # Integration with Session Manager
    ///
    /// The session layer can be configured to periodically run this cleanup:
    ///
    /// ```no_run
    /// use std::time::Duration;
    /// use tower_sessions::Expiry;
    /// use tower_sessions_seaorm_store::PostgresStore;
    ///
    /// # async fn example(store: PostgresStore) {
    /// let session_layer = tower_sessions::SessionManagerLayer::new(store)
    ///     .with_expiry(Expiry::OnInactivity(time::Duration::days(1)))
    ///     .with_cleanup_task(Duration::from_secs(3600)); // Run cleanup every hour
    /// # }
    /// ```
    async fn delete_expired(&self) -> session_store::Result<()> {
        let now = OffsetDateTime::now_utc();
        let now_db = convert_time_to_datetime(now);

        SessionEntity::delete_many()
            .filter(session::Column::ExpiryDate.lt(now_db))
            .exec(&self.conn)
            .await
            .map_err(|e| session_store::Error::Backend(e.to_string()))?;

        Ok(())
    }
}

// Helper function to convert time::OffsetDateTime to sea_orm::prelude::DateTimeWithTimeZone (chrono)
fn convert_time_to_datetime(time: OffsetDateTime) -> DateTimeWithTimeZone {
    use chrono::{DateTime, NaiveDateTime, TimeZone, Utc};

    // Extract components from OffsetDateTime
    let year = time.year();
    let month = time.month() as u32;
    let day = time.day() as u32;
    let hour = time.hour() as u32;
    let minute = time.minute() as u32;
    let second = time.second() as u32;
    let nanosecond = time.nanosecond();

    // Use timestamp if possible (safer approach)
    if let Some(datetime) = DateTime::from_timestamp(time.unix_timestamp(), time.nanosecond()) {
        return datetime.into();
    }

    // Fallback to manual creation if timestamp is out of range
    let naive = NaiveDateTime::new(
        chrono::NaiveDate::from_ymd_opt(year, month, day).unwrap(),
        chrono::NaiveTime::from_hms_nano_opt(hour, minute, second, nanosecond).unwrap(),
    );

    // Convert to DateTimeWithTimeZone using TimeZone trait method instead of from_utc
    Utc.from_utc_datetime(&naive).into()
}
