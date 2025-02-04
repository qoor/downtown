// Copyright 2023. The downtown authors all rights reserved.

use chrono::{DateTime, Utc};
use sqlx::MySql;

use crate::{
    user::account::{User, UserId},
    Error, Result,
};

use super::PostId;

pub(crate) type CommentId = u64;

#[derive(Debug, sqlx::FromRow, Clone)]
pub(crate) struct Comment {
    id: CommentId,
    post_id: PostId,
    author_id: Option<UserId>,
    content: String,
    deleted: bool,
    created_at: DateTime<Utc>,
}

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct CommentNode {
    #[sqlx(flatten)]
    comment: Comment,
    parent_comment_id: CommentId,
    child_comment_id: CommentId,
}

impl CommentNode {
    pub(crate) fn comment(&self) -> &Comment {
        &self.comment
    }

    pub(crate) fn parent_comment_id(&self) -> CommentId {
        self.parent_comment_id
    }

    pub(crate) fn child_comment_id(&self) -> CommentId {
        self.child_comment_id
    }
}

impl Comment {
    pub(crate) async fn from_post_id(
        post_id: PostId,
        user: &User,
        db: &sqlx::Pool<MySql>,
    ) -> Result<Vec<CommentNode>> {
        Ok(sqlx::query_as(
                "SELECT
c.id,
c.post_id,
c.author_id,
c.content,
c.deleted,
c.created_at,
cc.parent_comment_id,
cc.child_comment_id
FROM post_comment as c
INNER JOIN post_comment_closure as cc ON cc.child_comment_id = c.id
WHERE
c.author_id NOT IN (SELECT target_id FROM user_block WHERE user_id = ?) AND
c.id NOT IN (SELECT comment_id FROM post_comment_block WHERE user_id = ?) AND
cc.parent_comment_id IN
(SELECT id FROM post_comment as c
INNER JOIN post_comment_closure as cc ON cc.parent_comment_id = cc.child_comment_id WHERE c.post_id = ?)
GROUP BY cc.parent_comment_id
ORDER BY c.created_at ASC",
            )
            .bind(user.id())
            .bind(user.id())
            .bind(post_id)
            .fetch_all(db).await?)
    }

    pub(crate) async fn add(
        post_id: PostId,
        author: &User,
        content: &str,
        parent_comment_id: Option<CommentId>,
        db: &sqlx::Pool<MySql>,
    ) -> Result<Self> {
        let tx = db.begin().await?;

        let id = sqlx::query!(
            "INSERT INTO post_comment (post_id, author_id, content) VALUES (?, ?, ?)",
            post_id,
            author.id(),
            content
        )
        .execute(db)
        .await
        .map(|row| row.last_insert_id())?;
        let parent_comment_id = parent_comment_id.unwrap_or(id);

        sqlx::query!(
            "INSERT INTO post_comment_closure (parent_comment_id, child_comment_id)
            SELECT cs.parent_comment_id, ? FROM post_comment_closure AS cs WHERE cs.child_comment_id = ?
            UNION ALL SELECT ?, ?",
            id,
            parent_comment_id,
            id,
            id
            )
            .execute(db)
            .await?;

        let comment = Self::from_id(id, author, db).await?;

        tx.commit().await?;

        Ok(comment)
    }

    pub(crate) async fn delete(id: CommentId, db: &sqlx::Pool<MySql>) -> Result<()> {
        let tx = db.begin().await?;

        sqlx::query!(
            "DELETE FROM post_comment_closure
            WHERE child_comment_id
            IN (SELECT child_comment_id FROM post_comment_closure WHERE parent_comment_id = ?)",
            id
        )
        .execute(db)
        .await?;
        sqlx::query!("DELETE FROM post_comment WHERE id = ?", id).execute(db).await?;

        tx.commit().await?;

        Ok(())
    }

    pub(crate) async fn from_id(
        id: CommentId,
        user: &User,
        db: &sqlx::Pool<MySql>,
    ) -> Result<Self> {
        sqlx::query_as!(
            Self,
            "SELECT
id,
post_id,
author_id as `author_id: _`,
content,
deleted as `deleted: _`,
created_at
FROM post_comment WHERE id = ? AND
author_id NOT IN (SELECT target_id FROM user_block WHERE user_id = ?) AND
id NOT IN (SELECT comment_id FROM post_comment_block WHERE user_id = ?)",
            id,
            user.id(),
            user.id()
        )
        .fetch_optional(db)
        .await?
        .ok_or(Error::CommentNotFound(id))
    }

    pub(crate) async fn from_id_ignore_block(
        id: CommentId,
        db: &sqlx::Pool<MySql>,
    ) -> Result<Self> {
        sqlx::query_as!(
            Self,
            "SELECT
id,
post_id,
author_id as `author_id: _`,
content,
deleted as `deleted: _`,
created_at
FROM post_comment WHERE id = ?",
            id,
        )
        .fetch_optional(db)
        .await?
        .ok_or(Error::CommentNotFound(id))
    }

    pub(crate) fn id(&self) -> CommentId {
        self.id
    }

    pub(crate) fn post_id(&self) -> PostId {
        self.post_id
    }

    pub(crate) fn author_id(&self) -> Option<UserId> {
        self.author_id
    }

    pub(crate) fn content(&self) -> &str {
        &self.content
    }

    pub(crate) fn is_deleted(&self) -> bool {
        self.deleted
    }

    pub(crate) fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }
}
