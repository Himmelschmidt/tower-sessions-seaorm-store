//! Axum Example for tower-sessions-seaorm-store
//!
//! This example demonstrates how to use the tower-sessions-seaorm-store with an Axum application.
//! It shows how to set up a database connection, initialize the PostgresStore, and use sessions
//! to store and retrieve values across requests.
//!
//! # Running the example
//!
//! 1. Make sure you have a PostgreSQL server running
//! 2. Set the DATABASE_URL environment variable to point to your PostgreSQL database:
//!    ```bash
//!    export DATABASE_URL=postgres://postgres:password@localhost:5432/sessions
//!    ```
//! 3. Run the example:
//!    ```bash
//!    cargo run --example axum_example
//!    ```
//! 4. The server will start on http://127.0.0.1:3000
//!
//! # Testing the example
//!
//! Once the server is running, you can test it with curl:
//!
//! ```bash
//! # Set a session value
//! curl -v -c cookies.txt -X POST "http://127.0.0.1:3000/set?name=username&value=john_doe"
//!
//! # Get the session value
//! curl -v -b cookies.txt "http://127.0.0.1:3000/get?name=username"
//!
//! # Clear the session
//! curl -v -b cookies.txt -c cookies.txt -X POST http://127.0.0.1:3000/clear
//! ```

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
use dotenvy::dotenv;
use sea_orm::{ConnectOptions, Database, DatabaseConnection};
use std::{collections::HashMap, env, net::SocketAddr, time::Duration};
use time::Duration as TimeDuration;
use tower_sessions::{Expiry, Session, SessionManagerLayer};
use tower_sessions_seaorm_store::PostgresStore;
use tracing::{info, Level};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

// Application state that will be shared across handlers
#[derive(Clone)]
struct AppState {
    db: DatabaseConnection,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for better logging
    tracing_subscriber::registry()
        .with(fmt::layer())
        .with(EnvFilter::from_default_env().add_directive(Level::INFO.into()))
        .init();

    // Load environment variables from .env file if present
    dotenv().ok();

    // Get the database URL from the environment
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    info!("Connecting to database: {}", database_url);

    // Configure database connection
    let mut opt = ConnectOptions::new(database_url);
    opt.max_connections(10)
        .min_connections(2)
        .connect_timeout(Duration::from_secs(10))
        .acquire_timeout(Duration::from_secs(10))
        .idle_timeout(Duration::from_secs(10))
        .max_lifetime(Duration::from_secs(10 * 60));

    // Establish a connection to the database
    let db = Database::connect(opt).await?;

    info!("Connected to database");

    // Create the session store using our PostgresStore
    let store = PostgresStore::new(db.clone());

    // Create the app state
    let state = AppState { db };

    // Session expiry - sessions will expire after 24 hours of inactivity
    let session_expiry = Expiry::OnInactivity(TimeDuration::hours(24));

    // Create the session layer with our store and configuration
    let session_layer = SessionManagerLayer::new(store)
        .with_secure(false) // Allow non-HTTPS for development
        .with_expiry(session_expiry);

    // Set up routes with middleware
    let app = Router::new()
        .route("/", get(index))
        .route("/set", post(set_session_value))
        .route("/get", get(get_session_value))
        .route("/clear", post(clear_session))
        .with_state(state)
        .layer(session_layer);

    // Run the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    info!("Server starting on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
}

// Route handlers

/// Index route that shows basic usage information
async fn index() -> impl IntoResponse {
    Html(
        r#"
        <html>
            <head><title>Tower Sessions SeaORM Example</title></head>
            <body>
                <h1>Tower Sessions SeaORM Example</h1>
                <p>This example demonstrates the use of tower-sessions with the SeaORM PostgresStore.</p>
                
                <h2>Available Routes:</h2>
                <ul>
                    <li><code>POST /set?name=key&value=some_value</code> - Set a session value</li>
                    <li><code>GET /get?name=key</code> - Get a session value</li>
                    <li><code>POST /clear</code> - Clear the session</li>
                </ul>
            </body>
        </html>
        "#,
    )
}

/// Set a value in the session
///
/// Example: POST /set?name=username&value=john_doe
async fn set_session_value(
    session: Session,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let name = params.get("name");
    let value = params.get("value");

    match (name, value) {
        (Some(name), Some(value)) => {
            // Set the value in the session
            if let Err(e) = session.insert(name, value).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to set session value: {}", e),
                );
            }

            (
                StatusCode::OK,
                format!("Successfully set session value '{}' to '{}'", name, value),
            )
        }
        _ => (
            StatusCode::BAD_REQUEST,
            "Missing name or value parameter".to_string(),
        ),
    }
}

/// Get a value from the session
///
/// Example: GET /get?name=username
async fn get_session_value(
    session: Session,
    Query(params): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    let name = params.get("name");

    match name {
        Some(name) => {
            // Try to get the value from the session
            match session.get::<String>(name).await {
                Ok(Some(value)) => (
                    StatusCode::OK,
                    format!("Session value '{}' = '{}'", name, value),
                ),
                Ok(None) => (
                    StatusCode::NOT_FOUND,
                    format!("No session value found for '{}'", name),
                ),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to get session value: {}", e),
                ),
            }
        }
        None => (
            StatusCode::BAD_REQUEST,
            "Missing name parameter".to_string(),
        ),
    }
}

/// Clear the session by destroying it
///
/// This will remove all session data and invalidate the session cookie
async fn clear_session(session: Session) -> impl IntoResponse {
    match session.flush().await {
        Ok(_) => (StatusCode::OK, "Session cleared successfully".to_string()),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to clear session: {}", e),
        ),
    }
}
