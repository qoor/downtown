// Copyright 2023. The downtown authors all rights reserved.

pub mod config;
pub mod env;

mod handler;

use std::sync::Arc;

use axum::routing::get;
use config::Config;
use sqlx::MySql;

pub(crate) struct AppState {
    config: Config,
    database: sqlx::Pool<MySql>,
}

pub async fn app(config: Config, database: &sqlx::Pool<MySql>) -> axum::Router {
    let state = Arc::new(AppState { config, database: database.clone() });

    let root_routers = axum::Router::new().route("/", get(handler::root));

    axum::Router::new().merge(root_routers).with_state(state)
}

pub fn about() -> String {
    const NAME: &str = env!("CARGO_PKG_NAME");
    const DESCRIPTION: &str = env!("CARGO_PKG_DESCRIPTION");
    const VERSION: &str = env!("CARGO_PKG_VERSION");
    let authors: Vec<&str> = env!("CARGO_PKG_AUTHORS").split(':').collect();
    const HOMEPAGE: &str = env!("CARGO_PKG_HOMEPAGE");
    format!(
        "{NAME} - {DESCRIPTION}
{}

Version: {VERSION}
Authors: {:?}
\n",
        HOMEPAGE, authors
    )
}
