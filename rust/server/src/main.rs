mod api;
mod config;
mod db;
mod events;
mod models;
mod providers;
mod runner;
mod scheduler;
mod skills;
mod tool_runtime;

use std::{
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
};

use anyhow::Context;
use sqlx::{migrate::Migrator, sqlite::SqliteConnectOptions, SqlitePool};
use tokio::net::TcpListener;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;

use crate::{
    config::ConfigBundle, events::EventBus, providers::ProviderHub, skills::SkillRegistry,
};

static MIGRATOR: Migrator = sqlx::migrate!("./migrations");

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<ConfigBundle>,
    pub db: SqlitePool,
    pub events: EventBus,
    pub providers: Arc<ProviderHub>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    init_tracing();

    let config_dir = std::env::var("HYBRIDTRADE_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("./config"));
    let config = Arc::new(ConfigBundle::load(&config_dir)?);
    let skills_dir = resolve_skills_dir(&config_dir);
    let skills = SkillRegistry::load(&skills_dir)?;

    info!(skills_dir = %skills_dir.display(), "loaded agent skills");

    let database_path = PathBuf::from(&config.database.path);
    if let Some(parent) = database_path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("cannot create {:?}", parent))?;
    }

    let connect_options = SqliteConnectOptions::new()
        .filename(&config.database.path)
        .create_if_missing(true)
        .foreign_keys(true)
        .pragma("journal_mode", "WAL");
    let db = SqlitePool::connect_with(connect_options)
        .await
        .context("cannot connect sqlite")?;

    MIGRATOR.run(&db).await.context("cannot run migrations")?;
    db::bootstrap_schedules(&db, &config.schedules).await?;
    let providers = Arc::new(ProviderHub::new(
        config.providers.clone(),
        config.tooling.clone(),
        skills,
    )?);

    let state = Arc::new(AppState {
        config,
        db,
        events: EventBus::new(256),
        providers,
    });

    scheduler::start_background_workers(state.clone());

    let app = api::router(state.clone())
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive());

    let address = SocketAddr::new(
        state.config.server.host.parse().context("invalid host")?,
        state.config.server.port,
    );
    let listener = TcpListener::bind(address)
        .await
        .context("cannot bind listener")?;

    info!(address = %address, "hybridtrade backend listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .context("server exited with error")?;

    Ok(())
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "hybridtrade_server=info,sqlx=warn,tower_http=info".into()),
        )
        .init();
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}

fn resolve_skills_dir(config_dir: &Path) -> PathBuf {
    if let Ok(value) = std::env::var("HYBRIDTRADE_SKILLS_DIR") {
        return PathBuf::from(value);
    }

    let mut candidates = vec![PathBuf::from(".skills"), PathBuf::from("../.skills")];

    if let Some(config_parent) = config_dir.parent() {
        candidates.push(config_parent.join(".skills"));

        if let Some(project_root) = config_parent.parent() {
            candidates.push(project_root.join(".skills"));
        }
    }

    candidates
        .into_iter()
        .find(|path| path.exists())
        .unwrap_or_else(|| PathBuf::from(".skills"))
}
