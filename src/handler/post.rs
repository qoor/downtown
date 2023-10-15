// Copyright 2023. The downtown authors all rights reserved.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Extension, Json,
};
use axum_typed_multipart::TypedMultipart;

use crate::{
    post::Post,
    schema::{PostCreationSchema, PostEditSchema, PostResultSchema},
    user::account::User,
    AppState, Result,
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
