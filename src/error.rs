// Copyright 2023. The downtown authors all rights reserved.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use tracing::error;

pub type Result<T, E = Error> = core::result::Result<T, E>;

pub(crate) type BoxDynError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("cannot parse value {value} to {type_name} type")]
    CannotParse { value: String, type_name: String },
    #[error("an error occurred with the database")]
    Database(#[from] sqlx::Error),
    #[error("verification failed")]
    Verification,
    #[error("the verification code has been expired")]
    VerificationExpired,
    #[error("user with phone number {0} not found")]
    UserNotFound(String),
    #[error("an error occurred with the JWT token")]
    Token(jsonwebtoken::errors::Error),
    #[error("invalid token")]
    InvalidToken,
    #[error("token does not exist")]
    TokenNotExists,
    #[error("the token has been expired")]
    TokenExpired,
    #[error("unhandled exception")]
    Unhandled(BoxDynError),
}

impl Error {
    pub(crate) fn status(&self) -> StatusCode {
        match self {
            Error::CannotParse { value: _, type_name: _ } => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Database(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Verification => StatusCode::UNAUTHORIZED,
            Error::VerificationExpired => StatusCode::UNAUTHORIZED,
            Error::UserNotFound(_) => StatusCode::NOT_FOUND,
            Error::Token(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Error::InvalidToken => StatusCode::BAD_REQUEST,
            Error::TokenNotExists => StatusCode::NOT_FOUND,
            Error::TokenExpired => StatusCode::UNAUTHORIZED,
            Error::Unhandled(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        match self {
            Error::CannotParse { value: _, type_name: _ } => {
                error!("an error occurred while handling data")
            }
            Error::Database(ref err) => error!("database error: {err}"),
            Error::Token(ref err) => error!("jsonwebtoken error: {err}"),
            Error::Unhandled(ref err) => error!("unhandled error: {err}"),

            _ => (),
        }

        #[derive(Serialize)]
        struct ErrorResponse {
            message: String,
        }

        (self.status(), Json(ErrorResponse { message: self.to_string() })).into_response()
    }
}

impl From<jsonwebtoken::errors::Error> for Error {
    fn from(value: jsonwebtoken::errors::Error) -> Self {
        match value.kind() {
            jsonwebtoken::errors::ErrorKind::InvalidToken => Error::InvalidToken,
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => Error::TokenExpired,
            _ => Error::Token(value),
        }
    }
}