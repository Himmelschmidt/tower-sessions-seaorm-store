# Tower Sessions SeaORM Store

A [SeaORM](https://www.sea-ql.org/SeaORM/) session store implementation for [tower-sessions](https://github.com/maxcountryman/tower-sessions), providing seamless session storage in PostgreSQL databases.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
tower-sessions-seaorm-store = "0.1.0"
```

### Feature Flags

- `postgres` (default): Enables PostgreSQL support via SeaORM

Future support is planned for SQLite and MySQL databases.

## Usage

### Basic Example

```rust
use sea_orm::{Database, DbConn};
use time::Duration;
use tower_sessions::Expiry;
use tower_sessions_seaorm_store::PostgresStore;

// Connect to the database
let conn = Database::connect("postgres://postgres:postgres@localhost:5432/sessions").await?;

// Create a new PostgresStore
let store = PostgresStore::new(conn);

// Use the store with tower-sessions
let session_layer = tower_sessions::SessionManagerLayer::new(store)
    .with_expiry(Expiry::OnInactivity(Duration::days(7)));
```

### With Axum Framework

```rust
use axum::{
    routing::get,
    Router,
};
use sea_orm::Database;
use time::Duration;
use tower_sessions::{Expiry, SessionLayer};
use tower_sessions_seaorm_store::PostgresStore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Connect to database
    let conn = Database::connect("postgres://postgres:postgres@localhost:5432/sessions").await?;
    
    // Create store
    let store = PostgresStore::new(conn);
    
    // Create session layer
    let session_layer = SessionLayer::new(store)
        .with_secure(false)  // Set to true in production
        .with_expiry(Expiry::OnInactivity(Duration::hours(24)));
    
    // Build app with session layer
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .layer(session_layer);
    
    // Run it
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 3000));
    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .await?;
    
    Ok(())
}
```

## Configuration

The `PostgresStore` can be configured with the following options:

```rust
// Create store with custom table name
let store = PostgresStore::new(conn)
    .with_table_name("custom_sessions");
```

By default, the store uses a table named `tower_sessions`.

## Example Application

This crate includes a complete example application built with Axum, demonstrating how to:

1. Set up a database connection with SeaORM
2. Initialize the PostgresStore
3. Configure the session middleware
4. Use sessions to store and retrieve values

To run the example:

```bash
# Set the DATABASE_URL environment variable
export DATABASE_URL=postgres://postgres:password@localhost:5432/sessions

# Run the example
cargo run --example axum_example
```

The server will start on http://127.0.0.1:3000, and you can test it with:

```bash
# Set a session value
curl -v -c cookies.txt -X POST "http://127.0.0.1:3000/set?name=username&value=john_doe"

# Get the session value
curl -v -b cookies.txt "http://127.0.0.1:3000/get?name=username"

# Clear the session
curl -v -b cookies.txt -c cookies.txt -X POST http://127.0.0.1:3000/clear
```

## License

This project is licensed under the MIT License.