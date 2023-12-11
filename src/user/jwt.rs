// Copyright 2023. The downtown authors all rights reserved.

use std::sync::Arc;

use axum::{
    extract::{self, State},
    middleware,
    response::IntoResponse,
    RequestPartsExt,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey};
use serde::{Deserialize, Serialize};

use crate::{AppState, Error, Result};

use super::account::{User, UserId};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Claims {
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

pub(crate) struct Token {
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
            .map(|token| Token { encoded_token: token, user_id })?)
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

        Ok(Token { encoded_token, user_id })
    }

    pub(crate) fn encoded_token(&self) -> &str {
        &self.encoded_token
    }

    pub(crate) fn user_id(&self) -> UserId {
        self.user_id
    }
}

pub(crate) async fn authorize_user_middleware(
    State(state): State<Arc<AppState>>,
    req: extract::Request,
    next: middleware::Next,
) -> Result<impl IntoResponse> {
    let (mut parts, body) = req.into_parts();

    // Find the access token from Authorization header in HTTP headers
    let access_token = parts
        .extract::<TypedHeader<Authorization<Bearer>>>()
        .await
        .map(|header| header.token().to_string())
        .map_err(|_| Error::TokenNotExists)
        .ok();
    let user_id = authorize_user(access_token.as_deref(), state.config.public_key())
        .await
        .map(|token| token.user_id)?;

    let mut req = extract::Request::from_parts(parts, body);

    // Include the account data to extensions
    req.extensions_mut().insert(User::from_id(user_id, &state.database).await?);

    // Execute the next middleware
    Ok(next.run(req).await)
}

pub(crate) async fn authorize_user(token: Option<&str>, public_key: &DecodingKey) -> Result<Token> {
    Token::from_encoded_token(token, public_key)
}
