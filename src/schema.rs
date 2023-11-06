// Copyright 2023. The downtown authors all rights reserved.

use axum_typed_multipart::{FieldData, TryFromMultipart};
use chrono::{DateTime, NaiveDate, Utc};
use serde::Serialize;
use sqlx::MySql;
use tempfile::NamedTempFile;

use crate::{
    post::{comment::CommentId, Post, PostId, PostType},
    town::{Town, TownId},
    user::{
        self,
        account::{User, UserId},
        IdVerificationType,
    },
    Result,
};

#[derive(TryFromMultipart)]
pub struct RegistrationSchema {
    pub authorization_code: String,
    pub name: String,
    pub birthdate: String,
    pub sex: user::Sex,
    pub phone: String,
    pub address: String,
    pub verification_type: IdVerificationType,
    #[form_data(limit = "unlimited")]
    pub verification_photo: FieldData<NamedTempFile>,
}

#[derive(TryFromMultipart)]
pub struct PhoneVerificationSetupSchema {
    pub phone: String,
}

#[derive(TryFromMultipart)]
pub struct PhoneVerificationSchema {
    pub phone: String,
    pub code: String,
}

#[derive(Serialize)]
pub struct UserSchema {
    pub id: UserId,
    pub name: String,
    pub phone: String,
    pub birthdate: NaiveDate,
    pub sex: String,
    pub town: Town,
    pub verification_type: String,
    pub verification_photo_url: String,
    pub picture: String,
    pub bio: String,
}

#[derive(Serialize)]
pub struct TokenSchema {
    pub user_id: UserId,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(TryFromMultipart)]
pub struct ProfilePictureUpdateSchema {
    #[form_data(limit = "unlimited")]
    pub picture: FieldData<NamedTempFile>,
}

#[derive(TryFromMultipart)]
pub struct ProfileBioUpdateSchema {
    pub bio: String,
}

#[derive(TryFromMultipart)]
pub struct PostCreationSchema {
    pub author_id: UserId,
    pub post_type: PostType,
    pub content: String,
    pub age_range: Option<String>,
    pub capacity: Option<u32>,
    pub place: Option<String>,
    #[form_data(limit = "unlimited")]
    pub images: Vec<FieldData<NamedTempFile>>,
}

#[derive(Serialize)]
pub struct PostAuthor {
    pub id: UserId,
    pub name: String,
    pub picture: String,
}

impl From<User> for PostAuthor {
    fn from(value: User) -> Self {
        Self {
            id: value.id(),
            name: value.name().to_string(),
            picture: value.picture().to_string(),
        }
    }
}

#[derive(Serialize)]
pub struct PostGetResult {
    pub id: PostId,
    pub author: PostAuthor,
    pub post_type: PostType,
    pub town_id: TownId,
    pub content: String,
    pub images: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub age_range: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub place: Option<String>,
    pub created_at: DateTime<Utc>,
}

impl PostGetResult {
    pub(crate) async fn from_post(post: &Post, db: &sqlx::Pool<MySql>) -> Result<Self> {
        struct GatheringAgeRange {
            #[allow(dead_code)]
            id: u32,
            #[allow(dead_code)]
            min_age: Option<u32>,
            #[allow(dead_code)]
            max_age: Option<u32>,
            description: String,
        }

        let user = User::from_id(post.author_id(), db).await?;
        let age_range = match post.age_range() {
            Some(age_range) => sqlx::query_as!(
                GatheringAgeRange,
                "SELECT * FROM gathering_age_range WHERE id = ?",
                age_range
            )
            .fetch_one(db)
            .await
            .map(|data| data.description)
            .ok(),
            _ => None,
        };

        Ok(Self {
            id: post.id(),
            author: user.into(),
            post_type: post.post_type(),
            town_id: post.town_id(),
            content: post.content().to_string(),
            images: post.images(db).await?,
            age_range,
            capacity: post.capacity(),
            place: post.place().map(str::to_string),
            created_at: post.created_at(),
        })
    }
}

#[derive(TryFromMultipart)]
pub struct PostEditSchema {
    pub content: String,
    #[form_data(limit = "unlimited")]
    pub images: Vec<FieldData<NamedTempFile>>,
}

#[derive(Serialize)]
pub struct PostResultSchema {
    pub post_id: PostId,
    pub author_id: UserId,
}

#[derive(TryFromMultipart)]
pub struct CommentCreationSchema {
    pub content: String,
    pub parent_comment_id: Option<CommentId>,
}
