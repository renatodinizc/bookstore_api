use bookstore_api::{configuration, startup::run};
use sqlx::{Connection, Executor, PgConnection, PgPool};
use std::net::TcpListener;
use uuid::Uuid;

struct TestApp {
    address: String,
    db_pool: PgPool,
    db_name: String,
    db_url: String,
}

async fn spawn_app() -> TestApp {
    let tcp_listener = TcpListener::bind("localhost:0").expect("Failed to bind random port");
    let address = tcp_listener
        .local_addr()
        .expect("Failed to get local address")
        .to_string();

    let (db_pool, db_name, db_url) = setup_db().await;

    let server = run(tcp_listener, db_pool.clone()).expect("Failed to bind address");
    tokio::spawn(server);

    TestApp {
        address,
        db_pool,
        db_name,
        db_url,
    }
}

async fn setup_db() -> (PgPool, String, String) {
    let config = configuration::get_configuration().expect("Failed to read configuration.");
    let db_url = format!(
        "postgres://{}:{}@{}:{}",
        config.database.username,
        config.database.password,
        config.database.host,
        config.database.port,
    );
    let test_db_name = Uuid::new_v4().to_string();
    let test_db_url = format!("{}/{}", db_url, test_db_name);

    let mut db_connection = PgConnection::connect(&db_url)
        .await
        .expect("Failed to connect to Postgres.");
    db_connection
        .execute(format!(r#"CREATE DATABASE "{}";"#, test_db_name).as_str())
        .await
        .expect("Failed to create database.");

    let db_pool = PgPool::connect(&test_db_url)
        .await
        .expect("Failed to connect to Postgres.");

    sqlx::migrate!("./migrations")
        .run(&db_pool)
        .await
        .expect("Failed to migrate the database");

    (db_pool, test_db_name, db_url)
}

async fn drop_db(name: String, db_url: String) {
    // Connect to the default or system database, not the target database
    let system_db_url = format!("{}/postgres", db_url);
    let mut connection = PgConnection::connect(&system_db_url)
        .await
        .expect("Failed to connect to system database");

    // Terminate all connections to the target database
    let terminate_connections_query = format!(
        "SELECT pg_terminate_backend(pg_stat_activity.pid) FROM pg_stat_activity WHERE pg_stat_activity.datname = '{}'",
        name
    );
    connection
        .execute(terminate_connections_query.as_str())
        .await
        .expect("Failed to terminate connections");

    // Now attempt to drop the database
    let drop_db_query = format!("DROP DATABASE \"{}\"", name);
    connection
        .execute(drop_db_query.as_str())
        .await
        .expect("Failed to drop database");
}

#[tokio::test]
async fn user_creation() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("http://{}/users/create", app.address))
        .header("Content-Type", "application/json")
        .body(r#"{"name":"renato", "email":"example@email.com"}"#)
        .send()
        .await
        .expect("Failed to execute request.");

    let record = sqlx::query!("SELECT * FROM users")
        .fetch_one(&app.db_pool)
        .await
        .expect("Failed to fetch saved user.");

    assert!(response.status().is_success());
    assert_eq!(record.name, "renato");
    assert_eq!(record.email, "example@email.com");

    drop_db(app.db_name, app.db_url).await;
}

#[tokio::test]
async fn user_creation_with_invalid_data() {
    let app = spawn_app().await;
    let client = reqwest::Client::new();

    let response = client
        .post(format!("http://{}/users/create", app.address))
        .header("Content-Type", "application/json")
        .body(r#"{"name":"renato", "email":"example.com}"#)
        .send()
        .await
        .expect("Failed to execute request.");
    let record = sqlx::query!("SELECT * FROM users")
        .fetch_optional(&app.db_pool)
        .await
        .expect("Failed to fetch saved user.");

    assert!(response.status().is_client_error());
    assert!(record.is_none());

    drop_db(app.db_name, app.db_url).await;
}
