// Copyright 2023. The downtown authors all rights reserved.

use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Extension, Json};
use axum_typed_multipart::TypedMultipart;
use chrono::Duration;
use hyper::StatusCode;

use crate::{
    schema::{
        PhoneVerificationSchema, PhoneVerificationSetupSchema, RegistrationSchema, TokenSchema,
    },
    user::{account::User, jwt::Token, verification::PhoneVerification},
    AppState, Result,
};

pub async fn create_user(
    State(state): State<Arc<AppState>>,
    TypedMultipart(payload): TypedMultipart<RegistrationSchema>,
) -> Result<impl IntoResponse> {
    PhoneVerification::verify(&payload.phone, &payload.authorization_code, &state.database).await?;

    let user = User::register(&payload, &state.database).await?;

    PhoneVerification::cancel(&payload.phone, &state.database).await?;

    Ok(Json(create_jwt_token_pairs(&user, &state).await?))
}

pub(crate) async fn get_user_info(
    Extension(user): Extension<User>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse> {
    Ok(Json(user.to_schema(&state.database).await?))
}

pub(crate) async fn refresh_verification(
    Extension(user): Extension<User>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse> {
    let refresh_token = Token::new(
        state.config.private_key(),
        Duration::seconds(state.config.refresh_token_max_age()),
        user.id(),
    )
    .map(|token| token.encoded_token().to_string())?;

    user.update_refresh_token(&refresh_token, &state.database).await?;

    Ok(StatusCode::OK)
}

pub async fn setup_phone_verification(
    State(state): State<Arc<AppState>>,
    TypedMultipart(PhoneVerificationSetupSchema { phone }): TypedMultipart<
        PhoneVerificationSetupSchema,
    >,
) -> Result<impl IntoResponse> {
    PhoneVerification::send(&phone, &state.database).await?;

    Ok(StatusCode::CREATED)
}

pub async fn verify_phone(
    State(state): State<Arc<AppState>>,
    TypedMultipart(PhoneVerificationSchema { phone, code }): TypedMultipart<
        PhoneVerificationSchema,
    >,
) -> Result<impl IntoResponse> {
    PhoneVerification::verify(&phone, &code, &state.database).await?;

    let user = User::from_phone(&phone, &state.database).await?;

    PhoneVerification::cancel(&phone, &state.database).await?;

    Ok(Json(create_jwt_token_pairs(&user, &state).await?))
}

async fn create_jwt_token_pairs(user: &User, state: &Arc<AppState>) -> Result<TokenSchema> {
    let access_token = Token::new(
        state.config.private_key(),
        Duration::seconds(state.config.access_token_max_age()),
        user.id(),
    )
    .map(|token| token.encoded_token().to_string())?;
    let refresh_token = Token::new(
        state.config.private_key(),
        Duration::seconds(state.config.refresh_token_max_age()),
        user.id(),
    )
    .map(|token| token.encoded_token().to_string())?;

    user.update_refresh_token(&refresh_token, &state.database).await?;

    Ok(TokenSchema { user_id: user.id(), access_token, refresh_token })
}
