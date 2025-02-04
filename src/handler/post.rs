// Copyright 2023. The downtown authors all rights reserved.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
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
    schema::{
        CommentCreationSchema, CommentGetResult, PostCreationSchema, PostEditSchema, PostGetResult,
        PostListSchema, PostResultSchema,
    },
    user::account::{User, UserId},
    AppState, Error, Result,
};

pub(crate) async fn create_post(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    TypedMultipart(payload): TypedMultipart<PostCreationSchema>,
) -> Result<impl IntoResponse> {
    let post = Post::create(&user, payload, &state.database, &state.s3).await?;

    Ok(Json(PostResultSchema { post_id: post.id(), author_id: post.author_id() }))
}

pub(crate) async fn get_post(
    Path(post_id): Path<u64>,
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse> {
    let post = Post::from_id(post_id, &user, &state.database).await?;

    Ok(Json(PostGetResult::from_post(&post, &user, &state.database).await?))
}

pub(crate) async fn edit_post(
    Path(post_id): Path<u64>,
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
    TypedMultipart(PostEditSchema { content, images }): TypedMultipart<PostEditSchema>,
) -> Result<impl IntoResponse> {
    let post = Post::from_id(post_id, &user, &state.database).await?;

    post.edit(user.id(), &content, images, &state.database, &state.s3).await?;

    Ok(Json(PostResultSchema { post_id, author_id: user.id() }))
}

pub(crate) async fn delete_post(
    Path(post_id): Path<u64>,
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse> {
    let post = Post::from_id(post_id, &user, &state.database).await?;

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
    Post::from_id(post_id, &user, &state.database).await?;

    Comment::add(post_id, &user, &content, parent_comment_id, &state.database).await.map(
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

pub(crate) async fn get_post_comments(
    Path(post_id): Path<u64>,
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse> {
    CommentGetResult::from_comment_nodes(
        Comment::from_post_id(post_id, &user, &state.database).await?,
        &state.database,
    )
    .await
    .map(Json)
}

pub(crate) async fn delete_post_comment(
    Path((post_id, comment_id)): Path<(u64, u64)>,
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse> {
    let comment = Comment::from_id(comment_id, &user, &state.database).await?;

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

pub(crate) async fn get_post_list(
    Query(params): Query<PostListSchema>,
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse> {
    let posts = Post::get(&user, params.last_id(), params.limit(), &state.database).await?;

    Ok(Json(PostGetResult::from_posts(posts, &user, &state.database).await?))
}
