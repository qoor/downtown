// Copyright 2023. The downtown authors all rights reserved.

use axum_typed_multipart::{FieldData, TryFromMultipart};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::MySql;
use tempfile::NamedTempFile;

use crate::{
    post::{comment::CommentId, GatheringAgeRange, Post, PostId, PostType},
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
    pub verified: bool,
    pub verification_type: String,
    pub verification_photo_url: String,
    pub picture: String,
    pub bio: String,
    pub total_likes: i64,
}

#[derive(Serialize)]
pub struct OtherUserSchema {
    pub id: UserId,
    pub name: String,
    pub phone: String,
    pub birthdate: NaiveDate,
    pub sex: String,
    pub town: Town,
    pub verified: bool,
    pub picture: String,
    pub bio: String,
    pub total_likes: i64,
    pub my_like: bool,
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
    pub total_likes: i64,
    pub my_like: bool,
    pub total_comments: i64,
    pub created_at: DateTime<Utc>,
}

impl PostGetResult {
    pub(crate) async fn from_post(post: &Post, db: &sqlx::Pool<MySql>) -> Result<Self> {
        let user = User::from_id(post.author_id(), db).await?;
        let age_range = match post.age_range() {
            Some(age_range) => GatheringAgeRange::from_id(age_range, db)
                .await
                .map(|data| data.description().to_string())
                .ok(),
            _ => None,
        };
        let my_like = sqlx::query!(
            "SELECT id FROM post_like WHERE user_id = ? AND post_id = ? LIMIT 1",
            user.id(),
            post.id()
        )
        .fetch_optional(db)
        .await?
        .is_some();

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
            total_likes: post.total_likes(),
            my_like,
            total_comments: post.total_comments(),
            created_at: post.created_at(),
        })
    }

    pub(crate) async fn from_posts(posts: Vec<Post>, db: &sqlx::Pool<MySql>) -> Result<Vec<Self>> {
        let age_ranges = GatheringAgeRange::get_all(db).await?;
        let mut results: Vec<Self> = vec![];

        results.reserve(posts.len());
        for post in posts {
            let user = User::from_id(post.author_id(), db).await?;
            let my_like = sqlx::query!(
                "SELECT id FROM post_like WHERE user_id = ? AND post_id = ? LIMIT 1",
                user.id(),
                post.id()
            )
            .fetch_optional(db)
            .await?
            .is_some();

            results.push(Self {
                id: post.id(),
                author: user.into(),
                post_type: post.post_type(),
                town_id: post.town_id(),
                content: post.content().to_string(),
                images: post.images(db).await?,
                age_range: {
                    if let Some(post_age_range) = post.age_range() {
                        age_ranges
                            .iter()
                            .find(|age_range| post_age_range == age_range.id())
                            .map(|age_range| age_range.description().to_string())
                    } else {
                        None
                    }
                },
                capacity: post.capacity(),
                place: post.place().map(str::to_string),
                total_likes: post.total_likes(),
                my_like,
                total_comments: post.total_comments(),
                created_at: post.created_at(),
            })
        }

        Ok(results)
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

#[derive(Deserialize)]
pub struct PostListSchema {
    pub last_id: Option<PostId>,
    pub limit: Option<i32>,
}

#[derive(Serialize)]
pub struct UserLikeResult {
    pub issuer_id: UserId,
    pub target_id: UserId,
}

#[derive(Serialize)]
pub struct PostLikeResult {
    pub user_id: UserId,
    pub post_id: UserId,
}
