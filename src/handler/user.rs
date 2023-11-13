// Copyright 2023. The downtown authors all rights reserved.

use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    headers::{authorization::Bearer, Authorization},
    response::IntoResponse,
    Extension, Json, TypedHeader,
};
use axum_typed_multipart::TypedMultipart;
use chrono::Duration;
use hyper::StatusCode;
use serde::Serialize;

use crate::{
    post::{Post, PostId},
    schema::{
        PhoneVerificationSchema, PhoneVerificationSetupSchema, PostGetResult, PostLikeResult,
        PostListSchema, ProfileBioUpdateSchema, ProfilePictureUpdateSchema, RegistrationSchema,
        TokenSchema, UserLikeResult,
    },
    user::{
        account::{User, UserId},
        jwt::{authorize_user, Token},
        verification::PhoneVerification,
    },
    AppState, Result,
};

pub async fn create_user(
    State(state): State<Arc<AppState>>,
    TypedMultipart(payload): TypedMultipart<RegistrationSchema>,
) -> Result<impl IntoResponse> {
    let phone = payload.phone.clone();
    let authorization_code = payload.authorization_code.clone();

    PhoneVerification::verify(&phone, &authorization_code, &state.database).await?;

    let user = User::register(payload, &state.database, &state.s3).await?;

    PhoneVerification::cancel(&phone, &state.database).await?;

    Ok(Json(create_jwt_token_pairs(&user, &state).await?))
}

pub(crate) async fn get_other_user_info(
    Path(target_id): Path<UserId>,
    Extension(user): Extension<User>,
    State(state): State<Arc<AppState>>,
) -> Result<impl IntoResponse> {
    Ok(Json(
        User::from_id(target_id, &state.database)
            .await?
            .to_other_user_schema(&user, &state.database)
            .await?,
    ))
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
    user.like_post(&Post::from_id(post_id, &state.database).await?, &state.database).await?;

    Ok(Json(PostLikeResult { user_id: user.id(), post_id }))
}

pub(crate) async fn cancel_like_post(
    Path(post_id): Path<PostId>,
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse> {
    user.cancel_like_post(&Post::from_id(post_id, &state.database).await?, &state.database).await?;

    Ok(Json(PostLikeResult { user_id: user.id(), post_id }))
}

pub(crate) async fn get_my_posts(
    params: Query<PostListSchema>,
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<User>,
) -> Result<impl IntoResponse> {
    let posts = Post::from_user(&user, params.last_id(), params.limit(), &state.database).await?;

    Ok(Json(PostGetResult::from_posts(posts, &state.database).await?))
}

pub(crate) async fn get_user_posts(
    Path(target_id): Path<UserId>,
    params: Query<PostListSchema>,
    State(state): State<Arc<AppState>>,
    Extension(_user): Extension<User>,
) -> Result<impl IntoResponse> {
    let target_user = User::from_id(target_id, &state.database).await?;
    let posts =
        Post::from_user(&target_user, params.last_id(), params.limit(), &state.database).await?;

    Ok(Json(PostGetResult::from_posts(posts, &state.database).await?))
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
