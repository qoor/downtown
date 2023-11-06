// Copyright 2023. The downtown authors all rights reserved.

pub(crate) mod comment;

use axum::{async_trait, body};
use axum_typed_multipart::{FieldData, FieldMetadata, TryFromChunks, TypedMultipartError};
use chrono::{DateTime, Utc};
use rand::{distributions::Alphanumeric, Rng};
use serde::Serialize;
use sqlx::{MySql, QueryBuilder};
use tempfile::NamedTempFile;
use tokio::fs;

use crate::{aws::S3Client, town::TownId, user::account::UserId, Error, Result};

pub(crate) type PostId = u64;

const POST_IMAGE_PATH: &str = "post_image/";

#[derive(Clone, Copy, sqlx::Type, Serialize)]
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
    created_at: DateTime<Utc>,
}

struct PostImage {
    id: u64,
    post_id: PostId,
    image_url: Option<String>,
    created_at: DateTime<Utc>,
}

impl Post {
    pub(crate) async fn create(
        author_id: UserId,
        post_type: PostType,
        town_id: TownId,
        content: &str,
        images: Vec<FieldData<NamedTempFile>>,
        db: &sqlx::Pool<MySql>,
        s3: &S3Client,
    ) -> Result<Self> {
        let tx = db.begin().await?;

        let id = sqlx::query!(
            "INSERT INTO post (author_id, post_type, town_id, content) VALUES (?, ?, ?, ?)",
            author_id,
            post_type,
            town_id,
            content
        )
        .execute(db)
        .await
        .map(|row| row.last_insert_id())?;
        let post = Self::from_id(id, db).await?;
        post.upload_images(images, db, s3).await?;

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

    pub(crate) async fn from_id(id: u64, db: &sqlx::Pool<MySql>) -> Result<Self> {
        sqlx::query_as!(Self, "SELECT * FROM post WHERE id = ?", id).fetch_one(db).await.map_err(
            |err| match err {
                sqlx::Error::RowNotFound => Error::PostNotFound(id),
                _ => Error::Database(err),
            },
        )
    }

    pub(crate) async fn from_user_id(user_id: UserId, db: &sqlx::Pool<MySql>) -> Result<Vec<Self>> {
        Ok(sqlx::query_as!(
            Self,
            "SELECT id,
author_id,
post_type,
town_id,
content,
created_at
FROM post WHERE author_id = ?",
            user_id
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

    pub(crate) fn created_at(&self) -> DateTime<Utc> {
        self.created_at
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
            if let Some(url) = image.image_url {
                let parts: Vec<&str> = url.split('/').collect();

                if parts.len() < 2 {
                    continue;
                }

                let path = parts[1];

                if s3.delete_file(path).await.is_ok() {
                    deleted_ids.push(image.id);
                }
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
