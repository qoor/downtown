// Copyright 2023. The downtown authors all rights reserved.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Extension, Json,
};
use axum_typed_multipart::TypedMultipart;
use serde::Serialize;

use crate::{
    post::{
        comment::{Comment, CommentId},
        Post, PostId,
    },
    schema::{CommentCreationSchema, PostCreationSchema, PostEditSchema, PostResultSchema},
    user::account::{User, UserId},
    AppState, Error, Result,
};

pub(crate) async fn create_post(
    State(state): State<Arc<AppState>>,
    TypedMultipart(PostCreationSchema { author_id, content, images }): TypedMultipart<
        PostCreationSchema,
    >,
) -> Result<impl IntoResponse> {
    let post = Post::create(author_id, &content, images, &state.database, &state.s3).await?;

    Ok(Json(PostResultSchema { post_id: post.id(), author_id: post.author_id() }))
}

pub(crate) async fn edit_post(
    Path(post_id): Path<u64>,
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    TypedMultipart(PostEditSchema { content, images }): TypedMultipart<PostEditSchema>,
) -> Result<impl IntoResponse> {
    let post = Post::from_id(post_id, &state.database).await?;

    post.edit(user.id(), &content, images, &state.database, &state.s3).await?;

    Ok(Json(PostResultSchema { post_id, author_id: user.id() }))
}

pub(crate) async fn delete_post(
    Path(post_id): Path<u64>,
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse> {
    let post = Post::from_id(post_id, &state.database).await?;

    post.delete(user.id(), &state.database, &state.s3).await?;

    Ok(Json(PostResultSchema { post_id, author_id: user.id() }))
}

pub(crate) async fn create_post_comment(
    Path(post_id): Path<u64>,
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    TypedMultipart(CommentCreationSchema { content, parent_comment_id }): TypedMultipart<
        CommentCreationSchema,
    >,
) -> Result<impl IntoResponse> {
    Post::from_id(post_id, &state.database).await?;

    Comment::add(post_id, user.id(), &content, parent_comment_id, &state.database).await.map(
        |comment| {
            #[derive(Serialize)]
            struct CommentCreationResult {
                id: CommentId,
                post_id: PostId,
                author_id: UserId,
            }
            Json(CommentCreationResult { id: comment.id(), post_id, author_id: user.id() })
        },
    )
}

pub(crate) async fn delete_post_comment(
    Path(post_id): Path<u64>,
    Path(comment_id): Path<u64>,
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse> {
    let comment = Comment::from_id(comment_id, &state.database).await?;

    if post_id != comment.post_id() {
        return Err(Error::InvalidRequest);
    }

    match comment.author_id() {
        Some(author_id) if user.id() == author_id => (),
        _ => return Err(Error::InvalidRequest),
    };

    Comment::delete(comment_id, &state.database).await.map(|_| {
        #[derive(Serialize)]
        struct CommentDeletionResult {
            id: CommentId,
            post_id: PostId,
            author_id: UserId,
        }

        Json(CommentDeletionResult { id: comment_id, post_id, author_id: user.id() })
    })
}
