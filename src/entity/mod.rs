//! Database entity models for tower-sessions-seaorm-store.
//!
//! This module contains the Sea-ORM entity definitions used by the PostgreSQL
//! session store implementation. These entities define the database schema
//! and provide the data structures necessary for interacting with the database.
//!
//! The primary entity in this module is the `session` entity, which represents
//! the database table used to store session data.

/// Session entity model for Sea-ORM database interaction.
///
/// Contains the database schema representation and entity model for storing
/// session data in a PostgreSQL database.
pub mod session;
