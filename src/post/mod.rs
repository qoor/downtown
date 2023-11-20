// Copyright 2023. The downtown authors all rights reserved.

pub(crate) mod comment;

use axum::{async_trait, body};
use axum_typed_multipart::{FieldData, FieldMetadata, TryFromChunks, TypedMultipartError};
use chrono::{DateTime, Utc};
use rand::{distributions::Alphanumeric, Rng};
use serde_repr::Serialize_repr;
use sqlx::{MySql, QueryBuilder};
use tempfile::NamedTempFile;
use tokio::fs;

use crate::{
    aws::S3Client,
    schema::PostCreationSchema,
    town::TownId,
    user::account::{User, UserId},
    Error, Result,
};

pub(crate) type PostId = u64;

const POST_IMAGE_PATH: &str = "post_image/";

#[derive(Clone, Copy, sqlx::Type, Serialize_repr)]
#[repr(u32)]
pub enum PostType {
    Daily = 1,
    Question = 2,
    Gathering = 3,
}

impl From<u32> for PostType {
    fn from(value: u32) -> Self {
        match value {
            1 => PostType::Daily,
            2 => PostType::Question,
            3 => PostType::Gathering,
            _ => panic!("undefined post type: {}", value),
        }
    }
}

#[async_trait]
impl TryFromChunks for PostType {
    async fn try_from_chunks(
        chunks: impl futures_util::stream::Stream<Item = Result<body::Bytes, TypedMultipartError>>
            + Send
            + Sync
            + Unpin,
        metadata: FieldMetadata,
    ) -> Result<Self, TypedMultipartError> {
        let value = u32::try_from_chunks(chunks, metadata).await?;

        Ok(PostType::from(value))
    }
}

#[derive(sqlx::FromRow)]
pub(crate) struct Post {
    id: PostId,
    author_id: UserId,
    post_type: PostType,
    town_id: TownId,
    content: String,
    age_range: Option<u32>,
    capacity: Option<u32>,
    place: Option<String>,
    total_likes: i64,
    total_comments: i64,
    created_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct PostImage {
    id: u64,
    post_id: PostId,
    image_url: String,
    created_at: DateTime<Utc>,
}

impl Post {
    pub(crate) async fn create(
        user: &User,
        mut data: PostCreationSchema,
        db: &sqlx::Pool<MySql>,
        s3: &S3Client,
    ) -> Result<Self> {
        let tx = db.begin().await?;

        let mut age_range_id: Option<u32> = None;

        match data.post_type {
            PostType::Gathering => match data.age_range {
                Some(description) => {
                    age_range_id = GatheringAgeRange::from_description(&description, db)
                        .await
                        .map(|row| Some(row.id))?;
                }
                _ => {
                    age_range_id = Some(1);
                }
            },
            _ => {
                data.capacity = None;
                data.place = None;
            }
        };

        let id = sqlx::query!(
            "INSERT INTO post (author_id, post_type, town_id, content, age_range, capacity, place) VALUES (?, ?, ?, ?, ?, ?, ?)",
            user.id(),
            data.post_type,
            user.town_id(),
            data.content,
            age_range_id,
            data.capacity,
            data.place
        )
        .execute(db)
        .await
        .map(|row| row.last_insert_id())?;
        let post = Self::from_id(id, user, db).await?;
        post.upload_images(data.images, db, s3).await?;

        tx.commit().await?;

        Ok(post)
    }

    pub(crate) async fn edit(
        mut self,
        author_id: UserId,
        content: &str,
        images: Vec<FieldData<NamedTempFile>>,
        db: &sqlx::Pool<MySql>,
        s3: &S3Client,
    ) -> Result<Self> {
        if author_id != self.author_id() {
            return Err(Error::PostNotFound(self.id()));
        }

        let tx = db.begin().await?;

        sqlx::query!(
            "UPDATE post SET content = ? WHERE id = ? AND author_id = ?",
            content,
            self.id,
            author_id
        )
        .execute(db)
        .await?;

        self.delete_images(db, s3).await?;
        self.upload_images(images, db, s3).await?;

        tx.commit().await?;

        self.content = content.to_string();

        Ok(self)
    }

    pub(crate) async fn delete(
        self,
        author_id: UserId,
        db: &sqlx::Pool<MySql>,
        s3: &S3Client,
    ) -> Result<()> {
        if author_id != self.author_id() {
            return Err(Error::PostNotFound(self.id()));
        }

        sqlx::query!("DELETE FROM post WHERE id = ? AND author_id = ?", self.id, self.author_id)
            .execute(db)
            .await?;

        self.delete_images(db, s3).await?;

        Ok(())
    }

    pub(crate) async fn from_id(id: u64, user: &User, db: &sqlx::Pool<MySql>) -> Result<Self> {
        sqlx::query_as!(
            Self,
            "SELECT id,
author_id,
post_type,
town_id,
content,
age_range,
capacity,
place,
(SELECT COUNT(*) FROM post_like as pl WHERE pl.post_id = p.id) as `total_likes!`,
(SELECT COUNT(*) FROM post_comment as pc WHERE pc.post_id = p.id) as `total_comments!`,
created_at FROM post as p WHERE
id = ? AND town_id = ? AND
author_id NOT IN (SELECT target_id FROM user_block WHERE user_id = ?) AND
id NOT IN (SELECT post_id FROM post_block WHERE user_id = ?)
",
            id,
            user.town_id(),
            user.id(),
            user.id()
        )
        .fetch_one(db)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => Error::PostNotFound(id),
            _ => Error::Database(err),
        })
    }

    pub(crate) async fn from_id_ignore_block(
        id: u64,
        user: &User,
        db: &sqlx::Pool<MySql>,
    ) -> Result<Self> {
        sqlx::query_as!(
            Self,
            "SELECT id,
author_id,
post_type,
town_id,
content,
age_range,
capacity,
place,
(SELECT COUNT(*) FROM post_like as pl WHERE pl.post_id = p.id) as `total_likes!`,
(SELECT COUNT(*) FROM post_comment as pc WHERE pc.post_id = p.id) as `total_comments!`,
created_at FROM post as p WHERE
id = ? AND town_id = ?",
            id,
            user.town_id(),
        )
        .fetch_one(db)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => Error::PostNotFound(id),
            _ => Error::Database(err),
        })
    }

    pub(crate) async fn from_user(
        user: &User,
        last_id: PostId,
        limit: i32,
        db: &sqlx::Pool<MySql>,
    ) -> Result<Vec<Self>> {
        Ok(sqlx::query_as!(
            Self,
            "SELECT id,
author_id,
post_type,
town_id,
content,
age_range,
capacity,
place,
(SELECT COUNT(*) FROM post_like as pl WHERE pl.post_id = p.id) as `total_likes!`,
(SELECT COUNT(*) FROM post_comment as pc WHERE pc.post_id = p.id) as `total_comments!`,
created_at
FROM post as p WHERE
id < ? AND town_id = ? AND author_id = ? AND
author_id NOT IN (SELECT target_id FROM user_block WHERE user_id = ?) AND
id NOT IN (SELECT post_id FROM post_block WHERE user_id = ?)
ORDER BY id DESC LIMIT ?",
            last_id,
            user.town_id(),
            user.id(),
            user.id(),
            user.id(),
            limit
        )
        .fetch_all(db)
        .await?)
    }

    pub(crate) async fn get(
        user: &User,
        last_id: PostId,
        limit: i32,
        db: &sqlx::Pool<MySql>,
    ) -> Result<Vec<Self>> {
        Ok(sqlx::query_as!(
            Self,
            "SELECT id,
author_id,
post_type,
town_id,
content,
age_range,
capacity,
place,
(SELECT COUNT(*) FROM post_like as pl WHERE pl.post_id = p.id) as `total_likes!`,
(SELECT COUNT(*) FROM post_comment as pc WHERE pc.post_id = p.id) as `total_comments!`,
created_at
FROM post as p WHERE
id < ? AND town_id = ? AND
author_id NOT IN (SELECT target_id FROM user_block WHERE user_id = ?) AND
id NOT IN (SELECT post_id FROM post_block WHERE user_id = ?)
ORDER BY id DESC LIMIT ?",
            last_id,
            user.town_id(),
            user.id(),
            user.id(),
            limit
        )
        .fetch_all(db)
        .await?)
    }

    pub(crate) fn id(&self) -> PostId {
        self.id
    }

    pub(crate) fn author_id(&self) -> UserId {
        self.author_id
    }

    pub(crate) fn post_type(&self) -> PostType {
        self.post_type
    }

    pub(crate) fn town_id(&self) -> TownId {
        self.town_id
    }

    pub(crate) fn content(&self) -> &str {
        &self.content
    }

    pub(crate) fn age_range(&self) -> Option<u32> {
        self.age_range
    }

    pub(crate) fn capacity(&self) -> Option<u32> {
        self.capacity
    }

    pub(crate) fn place(&self) -> Option<&str> {
        self.place.as_deref()
    }

    pub(crate) fn total_likes(&self) -> i64 {
        self.total_likes
    }

    pub(crate) fn total_comments(&self) -> i64 {
        self.total_comments
    }

    pub(crate) fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    pub(crate) async fn images(&self, db: &sqlx::Pool<MySql>) -> Result<Vec<String>> {
        Ok(sqlx::query_as!(PostImage, "SELECT * FROM post_image WHERE post_id = ?", self.id)
            .fetch_all(db)
            .await?
            .iter()
            .map(|image| image.image_url.clone())
            .collect())
    }

    async fn upload_images(
        &self,
        images: Vec<FieldData<NamedTempFile>>,
        db: &sqlx::Pool<MySql>,
        s3: &S3Client,
    ) -> Result<()> {
        let mut image_urls: Vec<String> = vec![];

        for image in images {
            let basename: String =
                rand::thread_rng().sample_iter(Alphanumeric).take(32).map(char::from).collect();
            let dir = std::env::temp_dir().join(std::env!("CARGO_PKG_NAME"));
            let temp_path = dir.join(&basename);

            fs::create_dir_all(&dir)
                .await
                .map_err(|err| Error::Io { path: dir.to_path_buf(), source: err })?;

            image.contents.persist(&temp_path).map_err(|err| Error::PersistFile {
                path: temp_path.clone(),
                source: err.into(),
            })?;

            let url =
                s3.push_file(&temp_path, &(String::from(POST_IMAGE_PATH) + &basename)).await?;
            // if let Ok(url) = url {
            image_urls.push(url)
            // }
        }

        sqlx::query!("DELETE FROM post_image WHERE post_id = ?", self.id).execute(db).await?;

        if !image_urls.is_empty() {
            let mut sql =
                QueryBuilder::<MySql>::new("INSERT INTO post_image (post_id, image_url) ");
            sql.push_values(image_urls.iter(), |mut sql, url| {
                sql.push_bind(self.id);
                sql.push_bind(url);
            });
            let sql = sql.build().persistent(false);
            sql.execute(db).await?;
        }

        Ok(())
    }

    async fn delete_images(&self, db: &sqlx::Pool<MySql>, s3: &S3Client) -> Result<()> {
        let images =
            sqlx::query_as!(PostImage, "SELECT * FROM post_image WHERE post_id = ?", self.id)
                .fetch_all(db)
                .await?;
        let mut deleted_ids: Vec<u64> = vec![];

        for image in images {
            let url = image.image_url;
            let parts: Vec<&str> = url.split('/').collect();

            if parts.len() < 2 {
                continue;
            }

            let path = parts[1];

            if s3.delete_file(path).await.is_ok() {
                deleted_ids.push(image.id);
            }
        }

        if !deleted_ids.is_empty() {
            let mut sql = QueryBuilder::<MySql>::new("DELETE FROM post_image WHERE id IN (");

            let mut separated = sql.separated(", ");
            deleted_ids.iter().for_each(|deleted_id| {
                separated.push_bind(deleted_id);
            });
            separated.push_unseparated(")");

            let sql = sql.build().persistent(false);
            sql.execute(db).await?;
        }

        Ok(())
    }
}

pub(crate) struct GatheringAgeRange {
    #[allow(dead_code)]
    id: u32,
    #[allow(dead_code)]
    min_age: Option<u32>,
    #[allow(dead_code)]
    max_age: Option<u32>,
    description: String,
}

impl GatheringAgeRange {
    pub(crate) async fn from_description(
        description: &str,
        db: &sqlx::Pool<MySql>,
    ) -> Result<Self> {
        Ok(sqlx::query_as!(
            GatheringAgeRange,
            "SELECT * FROM gathering_age_range WHERE description = ?",
            description
        )
        .fetch_one(db)
        .await?)
    }

    pub(crate) async fn from_id(id: u32, db: &sqlx::Pool<MySql>) -> Result<Self> {
        Ok(sqlx::query_as!(GatheringAgeRange, "SELECT * FROM gathering_age_range WHERE id = ?", id)
            .fetch_one(db)
            .await?)
    }

    pub(crate) async fn get_all(db: &sqlx::Pool<MySql>) -> Result<Vec<Self>> {
        Ok(sqlx::query_as!(GatheringAgeRange, "SELECT * FROM gathering_age_range")
            .fetch_all(db)
            .await?)
    }

    pub(crate) fn id(&self) -> u32 {
        self.id
    }

    pub(crate) fn description(&self) -> &str {
        &self.description
    }
}
