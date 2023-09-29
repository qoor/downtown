// Copyright 2023. The downtown authors all rights reserved.

use std::sync::Arc;

use axum::{
    extract::State,
    headers::{authorization::Bearer, Authorization},
    http, middleware,
    response::IntoResponse,
    RequestPartsExt, TypedHeader,
};
use axum_extra::extract::CookieJar;
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey};
use serde::{Deserialize, Serialize};

use crate::{AppState, Error, Result};

use super::account::{User, UserId};

pub(crate) const ACCESS_TOKEN_COOKIE: &str = "access_token";
pub(crate) const REFRESH_TOKEN_COOKIE: &str = "refresh_token";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct Claims {
    /// Issuer of the JWT
    iss: String,
    /// Time at which the JWT was issued; can be used to determine age of the
    /// JWT
    iat: i64,
    /// Time after which the JWT expires
    exp: i64,
    /// Subject of the JWT (the user)
    sub: String,
}

impl Claims {
    pub(crate) fn expires_in(&self) -> i64 {
        self.exp - self.iat
    }
}

pub(crate) struct Token {
    claims: Claims,
    encoded_token: String,
    user_id: UserId,
}

impl Token {
    pub(crate) fn new(
        private_key: &EncodingKey,
        expires_in: Duration,
        user_id: UserId,
    ) -> Result<Self> {
        let claims = Claims {
            iss: env!("CARGO_PKG_HOMEPAGE").to_string() + "/api",
            iat: Utc::now().timestamp(),
            exp: (Utc::now() + expires_in).timestamp(),
            sub: user_id.to_string(),
        };

        Ok(jsonwebtoken::encode(&jsonwebtoken::Header::new(Algorithm::RS256), &claims, private_key)
            .map(|token| Token { claims, encoded_token: token, user_id })?)
    }

    pub(crate) fn from_encoded_token(
        encoded_token: Option<&str>,
        public_key: &DecodingKey,
    ) -> Result<Self> {
        let encoded_token =
            encoded_token.ok_or(Error::TokenNotExists).and_then(|encoded_token| {
                if encoded_token.is_empty() {
                    return Err(Error::InvalidToken);
                }

                Ok(encoded_token.to_string())
            })?;

        let claims = jsonwebtoken::decode::<Claims>(
            &encoded_token,
            public_key,
            &jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256),
        )
        .map(|token| token.claims)?;

        let user_id =
            claims.sub.parse::<UserId>().map_err(|err| Error::Unhandled(Box::new(err)))?;

        Ok(Token { claims, encoded_token, user_id })
    }

    pub(crate) fn encoded_token(&self) -> &str {
        &self.encoded_token
    }
}

pub(crate) async fn authorize_user_middleware<B>(
    State(state): State<Arc<AppState>>,
    req: http::Request<B>,
    next: middleware::Next<B>,
) -> Result<impl IntoResponse> {
    let (mut parts, body) = req.into_parts();

    // Find the access token from Authorization header in HTTP headers
    let access_token = parts
        .extract::<TypedHeader<Authorization<Bearer>>>()
        .await
        .map(|header| header.token().to_string())
        .map_err(|_| Error::TokenNotExists)
        .ok();
    let user_id = authorize_user(access_token.as_deref(), state.config.public_key()).await?;

    let mut req = http::Request::from_parts(parts, body);

    // Include the account data to extensions
    req.extensions_mut().insert(User::from_id(user_id, &state.database).await?);

    // Execute the next middleware
    Ok(next.run(req).await)
}

pub(crate) async fn authorize_user(
    access_token: Option<&str>,
    public_key: &DecodingKey,
) -> Result<UserId> {
    Token::from_encoded_token(access_token, public_key).map(|token| token.user_id)
}