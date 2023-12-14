// Copyright 2023. The downtown authors all rights reserved.

use axum_typed_multipart::{FieldData, TryFromMultipart};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::MySql;
use tempfile::NamedTempFile;

use crate::{
    post::{
        comment::{Comment, CommentId, CommentNode},
        GatheringAgeRange, Post, PostId, PostType,
    },
    town::{Town, TownId},
    user::{
        self,
        account::{User, UserId, VerificationResult},
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
    pub verification_result: VerificationResult,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub verification_picture_url: Option<String>,
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
    pub verification_result: VerificationResult,
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

        Ok(Self::new(post, user, post.images(db).await?, age_range, my_like))
    }

    pub(crate) async fn from_posts(posts: Vec<Post>, db: &sqlx::Pool<MySql>) -> Result<Vec<Self>> {
        let age_ranges = GatheringAgeRange::get_all(db).await?;
        let mut results: Vec<Self> = vec![];

        results.reserve(posts.len());
        for post in posts.iter() {
            let user = User::from_id(post.author_id(), db).await?;
            let my_like = sqlx::query!(
                "SELECT id FROM post_like WHERE user_id = ? AND post_id = ? LIMIT 1",
                user.id(),
                post.id()
            )
            .fetch_optional(db)
            .await?
            .is_some();
            let age_range = if let Some(post_age_range) = post.age_range() {
                age_ranges
                    .iter()
                    .find(|age_range| post_age_range == age_range.id())
                    .map(|age_range| age_range.description().to_string())
            } else {
                None
            };

            results.push(Self::new(post, user, post.images(db).await?, age_range, my_like));
        }

        Ok(results)
    }

    fn new(
        post: &Post,
        user: User,
        images: Vec<String>,
        age_range: Option<String>,
        my_like: bool,
    ) -> Self {
        Self {
            id: post.id(),
            author: user.into(),
            post_type: post.post_type(),
            town_id: post.town_id(),
            content: post.content().to_string(),
            images,
            age_range,
            capacity: post.capacity(),
            place: post.place().map(str::to_string),
            total_likes: post.total_likes(),
            my_like,
            total_comments: post.total_comments(),
            created_at: post.created_at(),
        }
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

#[derive(Serialize)]
pub struct CommentGetResult {
    pub id: CommentId,
    pub post_id: PostId,
    pub author: Option<PostAuthor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    pub deleted: bool,
    pub created_at: DateTime<Utc>,
}

impl CommentGetResult {
    pub(crate) async fn from_comment(comment: Comment, db: &sqlx::Pool<MySql>) -> Result<Self> {
        let author = if let Some(author_id) = comment.author_id() {
            Some(User::from_id(author_id, db).await?)
        } else {
            None
        };

        Ok(Self {
            id: comment.id(),
            post_id: comment.post_id(),
            author: author.map(Into::into),
            content: {
                if !comment.is_deleted() {
                    Some(comment.content().to_string())
                } else {
                    None
                }
            },
            deleted: comment.is_deleted(),
            created_at: comment.created_at(),
        })
    }

    pub(crate) async fn from_comment_node(
        comment_node: CommentNode,
        db: &sqlx::Pool<MySql>,
    ) -> Result<CommentResultNode> {
        Ok(CommentResultNode {
            comment: Self::from_comment(comment_node.comment().clone(), db).await?,
            parent_comment_id: comment_node.parent_comment_id(),
            child_comment_id: comment_node.child_comment_id(),
        })
    }

    pub(crate) async fn from_comment_nodes(
        comment_nodes: Vec<CommentNode>,
        db: &sqlx::Pool<MySql>,
    ) -> Result<Vec<CommentResultNode>> {
        let mut result_nodes: Vec<CommentResultNode> = vec![];
        result_nodes.reserve(comment_nodes.len());

        for node in comment_nodes.into_iter() {
            result_nodes.push(Self::from_comment_node(node, db).await?);
        }

        Ok(result_nodes)
    }
}

#[derive(Serialize)]
pub(crate) struct CommentResultNode {
    #[serde(flatten)]
    comment: CommentGetResult,
    parent_comment_id: CommentId,
    child_comment_id: CommentId,
}

#[derive(Deserialize)]
pub struct PostListSchema {
    pub last_id: Option<PostId>,
    pub limit: Option<i32>,
}

impl PostListSchema {
    pub fn last_id(&self) -> PostId {
        self.last_id.unwrap_or(PostId::MAX)
    }

    pub fn limit(&self) -> i32 {
        self.limit.unwrap_or(10)
    }
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

#[derive(TryFromMultipart)]
pub struct UserVerification {
    pub verification_type: IdVerificationType,
    #[form_data(limit = "unlimited")]
    pub verification_picture: FieldData<NamedTempFile>,
}
