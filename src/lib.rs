// Copyright 2023. The downtown authors all rights reserved.

pub mod config;
pub mod env;
pub mod error;

mod aws;
mod handler;
mod post;
mod schema;
mod town;
mod user;

use std::sync::Arc;

pub use error::{Error, Result};

use axum::{
    extract::DefaultBodyLimit,
    middleware,
    routing::{delete, get, patch, post, put},
};
use config::Config;
use sqlx::MySql;

pub struct AppState {
    config: Config,
    database: sqlx::Pool<MySql>,
    s3: aws::S3Client,
}

pub async fn app(config: Config, database: &sqlx::Pool<MySql>) -> axum::Router {
    let state = Arc::new(AppState {
        config,
        database: database.clone(),
        s3: aws::S3Client::from_env().await,
    });

    let auth_layer =
        middleware::from_fn_with_state(state.clone(), user::jwt::authorize_user_middleware);

    let root_routers = axum::Router::new().route("/", get(handler::root));
    let user_routers = axum::Router::new()
        .route("/user", post(handler::user::create_user))
        .route("/user/me", get(handler::user::get_user_info).route_layer(auth_layer.clone()))
        .route(
            "/user/me/picture",
            patch(handler::user::update_profile_picture).route_layer(auth_layer.clone()),
        )
        .route(
            "/user/me/bio",
            patch(handler::user::update_profile_bio).route_layer(auth_layer.clone()),
        )
        .route("/user/verification", patch(handler::user::refresh_verification))
        .route("/user/verification/phone", post(handler::user::setup_phone_verification))
        .route("/user/verification/phone", put(handler::user::verify_phone));
    let post_routers = axum::Router::new()
        .route("/post", post(handler::post::create_post))
        .route("/post/:id", patch(handler::post::edit_post))
        .route("/post/:id", delete(handler::post::delete_post))
        .route(
            "/post/:id/comment",
            post(handler::post::create_post_comment).route_layer(auth_layer.clone()),
        )
        .route(
            "/post/:id/comment/:id",
            delete(handler::post::delete_post_comment).route_layer(auth_layer.clone()),
        );

    axum::Router::new()
        .merge(root_routers)
        .merge(user_routers)
        .merge(post_routers)
        .layer(DefaultBodyLimit::max(1024 * 1024 * 10))
        .with_state(state)
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
