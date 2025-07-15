use axum::{
    extract::Query,
    response::Html,
    routing::{get, post},
    Router,
};
use sea_orm::{Database, DbConn};
use serde::Deserialize;
use std::net::SocketAddr;
use time::Duration;
use tower_sessions::{Expiry, Session, SessionManagerLayer};
use tower_sessions_sqlx_store::PostgresStore;

#[derive(Deserialize)]
struct SetQuery {
    name: String,
    value: String,
}

#[derive(Deserialize)]
struct GetQuery {
    name: String,
}

async fn set_session(
    session: Session,
    Query(params): Query<SetQuery>,
) -> Html<String> {
    let _ = session
        .insert(&params.name, params.value.clone())
        .await;

    Html(format!(
        r#"
        <html>
            <body>
                <h1>Session Value Set</h1>
                <p>Set <strong>{}</strong> to <strong>{}</strong></p>
                <p><a href="/get?name={}">Get this value</a></p>
                <p><a href="/">Back to home</a></p>
            </body>
        </html>
        "#,
        params.name, params.value, params.name
    ))
}

async fn get_session(
    session: Session,
    Query(params): Query<GetQuery>,
) -> Html<String> {
    let value: Option<String> = session
        .get(&params.name)
        .await
        .unwrap_or(None);

    match value {
        Some(val) => Html(format!(
            r#"
            <html>
                <body>
                    <h1>Session Value</h1>
                    <p><strong>{}</strong> = <strong>{}</strong></p>
                    <p><a href="/">Back to home</a></p>
                </body>
            </html>
            "#,
            params.name, val
        )),
        None => Html(format!(
            r#"
            <html>
                <body>
                    <h1>Session Value Not Found</h1>
                    <p>No value found for <strong>{}</strong></p>
                    <p><a href="/">Back to home</a></p>
                </body>
            </html>
            "#,
            params.name
        )),
    }
}

async fn clear_session(session: Session) -> Html<&'static str> {
    let _ = session.clear().await;

    Html(
        r#"
        <html>
            <body>
                <h1>Session Cleared</h1>
                <p>All session data has been cleared.</p>
                <p><a href="/">Back to home</a></p>
            </body>
        </html>
        "#,
    )
}

async fn home() -> Html<&'static str> {
    Html(
        r#"
        <html>
            <body>
                <h1>Tower Sessions with SQLx Store and SeaORM</h1>
                <p>This example demonstrates using the tower-sessions-sqlx-store with a SeaORM database connection.</p>
                
                <h2>Try it out:</h2>
                <form action="/set" method="get">
                    <label>Name: <input type="text" name="name" placeholder="username" required></label><br><br>
                    <label>Value: <input type="text" name="value" placeholder="john_doe" required></label><br><br>
                    <button type="submit">Set Session Value</button>
                </form>
                
                <br>
                
                <form action="/get" method="get">
                    <label>Name: <input type="text" name="name" placeholder="username" required></label><br><br>
                    <button type="submit">Get Session Value</button>
                </form>
                
                <br>
                
                <form action="/clear" method="post">
                    <button type="submit">Clear Session</button>
                </form>
            </body>
        </html>
        "#,
    )
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Get database URL from environment
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:password@localhost:5432/sessions".to_string());

    println!("Connecting to database: {}", database_url);

    // Create SeaORM database connection
    let sea_orm_db: DbConn = Database::connect(&database_url).await?;

    // Extract the underlying SQLx connection pool from SeaORM
    let sqlx_pool = sea_orm_db.get_postgres_connection_pool().clone();

    // Create the session store using the SQLx pool
    let session_store = PostgresStore::new(sqlx_pool);

    // Run migrations to create the session table
    session_store.migrate().await?;

    // Create session layer with 7-day expiry
    let session_layer = SessionManagerLayer::new(session_store)
        .with_secure(false) // Set to true in production with HTTPS
        .with_expiry(Expiry::OnInactivity(Duration::days(7)));

    // Build the application with session middleware
    let app = Router::new()
        .route("/", get(home))
        .route("/set", get(set_session))
        .route("/get", get(get_session))
        .route("/clear", post(clear_session))
        .layer(session_layer);

    // Start the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Server running on http://{}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}