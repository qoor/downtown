// Copyright 2023. The downtown authors all rights reserved.

use std::sync::Arc;

use axum::{extract::State, response::IntoResponse, Json};
use axum_typed_multipart::TypedMultipart;
use chrono::Duration;
use hyper::StatusCode;

use crate::{
    schema::{
        PhoneVerificationSchema, PhoneVerificationSetupSchema, RegistrationSchema, TokenSchema,
    },
    user::{
        account::{User, UserId},
        jwt::Token,
        verification::PhoneVerification,
    },
    AppState, Result,
};

pub async fn create_user(
    State(state): State<Arc<AppState>>,
    TypedMultipart(payload): TypedMultipart<RegistrationSchema>,
) -> Result<impl IntoResponse> {
    PhoneVerification::verify(&payload.phone, &payload.authorization_code, &state.database).await?;

    let user = User::register(&payload, &state.database).await?;

    PhoneVerification::cancel(&payload.phone, &state.database).await?;

    Ok(Json(create_jwt_token_pairs(user.id(), &state).await?))
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

    Ok(Json(create_jwt_token_pairs(user.id(), &state).await?))
}

async fn create_jwt_token_pairs(user_id: UserId, state: &Arc<AppState>) -> Result<TokenSchema> {
    let access_token = Token::new(
        state.config.private_key(),
        Duration::seconds(state.config.access_token_max_age()),
        user_id,
    )?;
    let refresh_token = Token::new(
        state.config.private_key(),
        Duration::seconds(state.config.refresh_token_max_age()),
        user_id,
    )?;

    Ok(TokenSchema {
        user_id,
        access_token: access_token.encoded_token().to_string(),
        refresh_token: refresh_token.encoded_token().to_string(),
    })
}
