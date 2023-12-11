// Copyright 2023. The downtown authors all rights reserved.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    response::IntoResponse,
    Extension, Json,
};
use axum_extra::{
    headers::{authorization::Bearer, Authorization},
    TypedHeader,
};
use axum_typed_multipart::TypedMultipart;
use chrono::{Datelike, Duration};
use serde::Serialize;

use crate::{
    post::{
        comment::{Comment, CommentId},
        Post, PostId,
    },
    schema::{
        PhoneVerificationSchema, PhoneVerificationSetupSchema, PostGetResult, PostLikeResult,
        PostListSchema, ProfileBioUpdateSchema, ProfilePictureUpdateSchema, RegistrationSchema,
        TokenSchema, UserLikeResult,
    },
    user::{
        account::{User, UserId},
        authentication::PhoneAuthentication,
        jwt::{authorize_user, Token},
    },
    AppState, Error, Result,
};

pub async fn create_user(
    State(state): State<Arc<AppState>>,
    TypedMultipart(payload): TypedMultipart<RegistrationSchema>,
) -> Result<impl IntoResponse> {
    let phone = payload.phone.clone();
    let authorization_code = payload.authorization_code.clone();

    PhoneAuthentication::authorize(&phone, &authorization_code, &state.database).await?;

    let user = User::register(payload, &state.database, &state.s3).await?;

    PhoneAuthentication::cancel(&phone, &state.database).await?;

    Ok(Json(create_jwt_token_pairs(&user, &state).await?))
}

pub(crate) async fn get_other_user_info(
    Path(target_id): Path<UserId>,
    Extension(user): Extension<User>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse> {
    let target = User::from_id(target_id, &state.database).await?;
    let blocked = target.is_blocked(&user, &state.database).await?;

    if blocked {
        return Err(Error::BlockedContent);
    }

    Ok(Json(target.to_other_user_schema(&user, &state.database).await?))
}

pub(crate) async fn get_user_info(
    Extension(user): Extension<User>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse> {
    Ok(Json(user.to_schema(&state.database).await?))
}

pub(crate) async fn refresh_verification(
    TypedHeader(Authorization(refresh_token)): TypedHeader<Authorization<Bearer>>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse> {
    let refresh_token = refresh_token.token();
    let token = authorize_user(Some(refresh_token), state.config.public_key()).await?;
    let user = User::from_id(token.user_id(), &state.database).await?;

    user.verify_refresh_token(refresh_token)?;

    Ok(Json(create_jwt_token_pairs(&user, &state).await?))
}

pub async fn setup_phone_verification(
    State(state): State<Arc<AppState>>,
    TypedMultipart(PhoneVerificationSetupSchema { phone }): TypedMultipart<
        PhoneVerificationSetupSchema,
    >,
) -> Result<impl IntoResponse> {
    let user = User::from_phone(&phone, &state.database).await?;
    let bypass = user.created_at().year() == 1970;
    let result = PhoneAuthentication::send(&phone, &state.database).await?;

    #[derive(Serialize)]
    struct PhoneAuthenticationSetupResult {
        #[serde(skip_serializing_if = "Option::is_none")]
        code: Option<String>,
    }
    Ok(Json(PhoneAuthenticationSetupResult {
        code: {
            if !bypass {
                None
            } else {
                Some(result.code().to_string())
            }
        },
    }))
}

pub async fn verify_phone(
    State(state): State<Arc<AppState>>,
    TypedMultipart(PhoneVerificationSchema { phone, code }): TypedMultipart<
        PhoneVerificationSchema,
    >,
) -> Result<impl IntoResponse> {
    PhoneAuthentication::authorize(&phone, &code, &state.database).await?;

    let user = User::from_phone(&phone, &state.database).await?;

    PhoneAuthentication::cancel(&phone, &state.database).await?;

    Ok(Json(create_jwt_token_pairs(&user, &state).await?))
}

pub(crate) async fn update_profile_picture(
    Extension(mut user): Extension<User>,
    State(state): State<Arc<AppState>>,
    TypedMultipart(ProfilePictureUpdateSchema { picture }): TypedMultipart<
        ProfilePictureUpdateSchema,
    >,
) -> Result<impl IntoResponse> {
    let picture_url = user.update_picture(picture, &state.s3, &state.database).await?;

    #[derive(Serialize)]
    struct PictureUpdateResult {
        id: UserId,
        picture: String,
    }

    Ok(Json(PictureUpdateResult { id: user.id(), picture: picture_url }))
}

pub(crate) async fn update_profile_bio(
    Extension(mut user): Extension<User>,
    State(state): State<Arc<AppState>>,
    TypedMultipart(ProfileBioUpdateSchema { bio }): TypedMultipart<ProfileBioUpdateSchema>,
) -> Result<impl IntoResponse> {
    user.update_bio(&bio, &state.database).await?;

    #[derive(Serialize)]
    struct BioUpdateResult {
        id: UserId,
        bio: String,
    }

    Ok(Json(BioUpdateResult { id: user.id(), bio }))
}

pub(crate) async fn like_user(
    Path(target_id): Path<UserId>,
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse> {
    user.like_user(&User::from_id(target_id, &state.database).await?, &state.database).await?;

    Ok(Json(UserLikeResult { issuer_id: user.id(), target_id }))
}

pub(crate) async fn cancel_like_user(
    Path(target_id): Path<UserId>,
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse> {
    user.cancel_like_user(&User::from_id(target_id, &state.database).await?, &state.database)
        .await?;

    Ok(Json(UserLikeResult { issuer_id: user.id(), target_id }))
}

pub(crate) async fn like_post(
    Path(post_id): Path<PostId>,
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse> {
    user.like_post(&Post::from_id(post_id, &user, &state.database).await?, &state.database).await?;

    Ok(Json(PostLikeResult { user_id: user.id(), post_id }))
}

pub(crate) async fn cancel_like_post(
    Path(post_id): Path<PostId>,
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse> {
    user.cancel_like_post(&Post::from_id(post_id, &user, &state.database).await?, &state.database)
        .await?;

    Ok(Json(PostLikeResult { user_id: user.id(), post_id }))
}

pub(crate) async fn get_my_posts(
    Query(params): Query<PostListSchema>,
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse> {
    let posts = Post::from_user(&user, params.last_id(), params.limit(), &state.database).await?;

    Ok(Json(PostGetResult::from_posts(posts, &state.database).await?))
}

pub(crate) async fn get_user_posts(
    Path(target_id): Path<UserId>,
    Query(params): Query<PostListSchema>,
    State(state): State<Arc<AppState>>,
    Extension(_user): Extension<User>,
) -> Result<impl IntoResponse> {
    let target_user = User::from_id(target_id, &state.database).await?;
    let posts =
        Post::from_user(&target_user, params.last_id(), params.limit(), &state.database).await?;

    println!("last_id = {}, limit = {}", params.last_id(), params.limit());

    Ok(Json(PostGetResult::from_posts(posts, &state.database).await?))
}

#[derive(Serialize)]
struct UserBlockResult {
    id: UserId,
    target_id: UserId,
}

pub(crate) async fn block_user(
    Path(target_id): Path<UserId>,
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse> {
    let target = User::from_id(target_id, &state.database).await?;

    user.block_user(&target, &state.database).await?;

    Ok(Json(UserBlockResult { id: user.id(), target_id: target.id() }))
}

pub(crate) async fn unblock_user(
    Path(target_id): Path<UserId>,
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse> {
    let target = User::from_id(target_id, &state.database).await?;

    user.unblock_user(&target, &state.database).await?;

    Ok(Json(UserBlockResult { id: user.id(), target_id: target.id() }))
}

#[derive(Serialize)]
struct PostBlockResult {
    id: UserId,
    post_id: UserId,
}

pub(crate) async fn block_post(
    Path(post_id): Path<PostId>,
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse> {
    let post = Post::from_id(post_id, &user, &state.database).await?;

    user.block_post(&post, &state.database).await?;

    Ok(Json(PostBlockResult { id: user.id(), post_id: post.id() }))
}

pub(crate) async fn unblock_post(
    Path(post_id): Path<PostId>,
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse> {
    let post = Post::from_id_ignore_block(post_id, &user, &state.database).await?;

    user.unblock_post(&post, &state.database).await?;

    Ok(Json(PostBlockResult { id: user.id(), post_id: post.id() }))
}

#[derive(Serialize)]
struct CommentBlockResult {
    id: UserId,
    comment_id: UserId,
}

pub(crate) async fn block_post_comment(
    Path((post_id, comment_id)): Path<(PostId, CommentId)>,
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse> {
    let comment = Comment::from_id(comment_id, &user, &state.database).await?;

    if post_id != comment.post_id() {
        return Err(Error::InvalidRequest);
    }

    user.block_post_comment(&comment, &state.database).await?;

    Ok(Json(CommentBlockResult { id: user.id(), comment_id: comment.id() }))
}

pub(crate) async fn unblock_post_comment(
    Path((post_id, comment_id)): Path<(PostId, CommentId)>,
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse> {
    let comment = Comment::from_id_ignore_block(comment_id, &state.database).await?;

    if post_id != comment.post_id() {
        return Err(Error::InvalidRequest);
    }

    user.unblock_post_comment(&comment, &state.database).await?;

    Ok(Json(CommentBlockResult { id: user.id(), comment_id: comment.id() }))
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
