use horasculo_api::{
    app,
    config::{Config, read_secret},
    state::AppState,
};
use openidconnect::reqwest;
use sqlx::postgres::PgPoolOptions;
use tokio::net::TcpListener;
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "horasculo_api=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .init();

    if std::env::args().nth(1).as_deref() == Some("migrate") {
        let url = read_secret("MIGRATION_DATABASE_URL")?;
        let db = PgPoolOptions::new()
            .max_connections(2)
            .connect(&url)
            .await?;
        sqlx::migrate!("./migrations").run(&db).await?;
        horasculo_api::bootstrap::run(&db).await?;
        info!("migrações concluídas");
        return Ok(());
    }
    let config = Config::from_env()?;
    let db = PgPoolOptions::new()
        .max_connections(10)
        .connect(&config.database_url)
        .await?;
    let http = reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(8))
        .build()?;
    let listener = TcpListener::bind(config.address()).await?;

    info!(address = %config.address(), "API iniciada");
    axum::serve(listener, app(AppState::new(db, config, http)))
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    info!("Encerrando a API");
}
