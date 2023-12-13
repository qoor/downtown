// Copyright 2023. The downtown authors all rights reserved.

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;
use tracing::error;

use crate::post::{comment::CommentId, PostId};

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
    #[error("failed to upload file")]
    Upload { path: std::path::PathBuf, source: BoxDynError },
    #[error("failed to delete uploaded file")]
    DeleteUploaded { path: String, source: BoxDynError },
    #[error("an error occurred while processing the file to be uploaded")]
    FileToStream { path: std::path::PathBuf, source: BoxDynError },
    #[error("an error occurred while processing the file to be uploaded")]
    PersistFile { path: std::path::PathBuf, source: BoxDynError },
    #[error("an error occurred while processing I/O")]
    Io { path: std::path::PathBuf, source: std::io::Error },
    #[error("post id {0} not found")]
    PostNotFound(PostId),
    #[error("comment id {0} not found")]
    CommentNotFound(CommentId),
    #[error("invalid request")]
    InvalidRequest,
    #[error("the content has blocked")]
    BlockedContent,
    #[error("an error occurred with internal connection")]
    Reqwest {
        #[from]
        source: reqwest::Error,
    },
    #[error("an error occurred with url for internal connection")]
    UrlParse {
        #[from]
        source: url::ParseError,
    },
    #[error("an error occurred while sending message ({0})")]
    MessageSend(i32),
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
            Error::Upload { path: _, source: _ } => StatusCode::INTERNAL_SERVER_ERROR,
            Error::DeleteUploaded { path: _, source: _ } => StatusCode::INTERNAL_SERVER_ERROR,
            Error::FileToStream { path: _, source: _ } => StatusCode::INTERNAL_SERVER_ERROR,
            Error::PersistFile { path: _, source: _ } => StatusCode::INTERNAL_SERVER_ERROR,
            Error::Io { path: _, source: _ } => StatusCode::INTERNAL_SERVER_ERROR,
            Error::PostNotFound(_) => StatusCode::NOT_FOUND,
            Error::CommentNotFound(_) => StatusCode::NOT_FOUND,
            Error::InvalidRequest => StatusCode::BAD_REQUEST,
            Error::BlockedContent => StatusCode::FORBIDDEN,
            Error::Reqwest { source: _ } => StatusCode::INTERNAL_SERVER_ERROR,
            Error::UrlParse { source: _ } => StatusCode::INTERNAL_SERVER_ERROR,
            Error::MessageSend(_) => StatusCode::INTERNAL_SERVER_ERROR,
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
            Error::Upload { ref path, ref source } => {
                error!("failed to upload file {}: {source}", path.to_string_lossy())
            }
            Error::FileToStream { ref path, ref source } => {
                error!(
                    "failed to create byte stream from file {}: {source}",
                    path.to_string_lossy()
                )
            }
            Error::PersistFile { ref path, ref source } => {
                error!("failed to persist the file {}: {source}", path.to_string_lossy())
            }
            Error::Io { ref path, ref source } => {
                error!("{} I/O error: {source}", path.to_string_lossy())
            }
            Error::Reqwest { ref source } => error!("failed to request http: {source:?}"),
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
