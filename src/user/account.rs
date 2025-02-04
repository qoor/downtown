// Copyright 2023. The downtown authors all rights reserved.

use std::{fs::File, path::PathBuf};

use axum_typed_multipart::FieldData;
use chrono::{DateTime, NaiveDate, Utc};
use rand::{distributions::Alphanumeric, Rng};
use serde_repr::Serialize_repr;
use sqlx::MySql;
use tempfile::NamedTempFile;
use tokio::{fs, io};

use crate::{
    aws,
    post::{comment::Comment, Post},
    schema::{OtherUserSchema, RegistrationSchema, UserSchema},
    town::{Town, TownId},
    Error, Result,
};

use super::{IdVerificationType, Sex};

pub(crate) type UserId = u64;

const VERIFICATION_PHOTO_PATH: &str = "verification_photo/";

#[derive(Debug, sqlx::Type, Clone, Copy, Serialize_repr)]
#[repr(u32)]
pub enum VerificationResult {
    NotVerified = 0,
    Verified = 1,
    InvalidPicture = 2,
    LowQualityPicture = 3,
    NonMaskedIdCard = 4,
    NonMaskedDriverLicense = 5,
    NonMaskedAll = 6,
    NonResident = 7,
}

#[derive(Debug, sqlx::FromRow, Clone)]
pub(crate) struct User {
    id: UserId,
    name: String,
    phone: String,
    birthdate: NaiveDate,
    sex: Sex,
    town_id: TownId,
    verification_result: VerificationResult,
    verification_type: Option<IdVerificationType>,
    verification_picture_url: Option<String>,
    picture: String,
    bio: Option<String>,
    deleted: bool,
    refresh_token: Option<String>,
    total_likes: i64,
    created_at: DateTime<Utc>,
    #[allow(dead_code)]
    updated_at: DateTime<Utc>,
}

impl User {
    pub(crate) async fn register(data: RegistrationSchema, db: &sqlx::Pool<MySql>) -> Result<Self> {
        let tx = db.begin().await?;

        let town_id = Town::from_address(&data.address, db).await.map(|town| town.id())?;
        let user_id = sqlx::query!(
            "INSERT INTO user (
name,
phone,
birthdate,
sex,
town_id) VALUES (
?,
?,
?,
?,
?
)",
            data.name,
            data.phone,
            data.birthdate,
            data.sex,
            town_id,
        )
        .execute(db)
        .await
        .map(|row| row.last_insert_id())?;

        let user = Self::from_id(user_id, db).await?;

        tx.commit().await?;

        Ok(user)
    }

    pub(crate) async fn from_id(id: UserId, db: &sqlx::Pool<MySql>) -> Result<Self> {
        sqlx::query_as!(
            Self,
            "SELECT
id,
name,
phone,
birthdate,
sex as `sex: Sex`,
town_id,
verification_result as `verification_result: _`,
verification_type as `verification_type: _`,
verification_picture_url,
picture,
bio,
deleted as `deleted: _`,
refresh_token,
(SELECT COUNT(*) FROM user_like as ul WHERE ul.target_id = u.id) as `total_likes!`,
created_at,
updated_at
FROM user as u WHERE u.id = ?",
            id
        )
        .fetch_one(db)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => Error::UserNotFound(id.to_string()),
            _ => Error::Database(err),
        })
        .and_then(|user| match user.deleted {
            false => Ok(user),
            true => Err(Error::DeletedUser),
        })
    }

    pub(crate) async fn from_phone(phone: &str, db: &sqlx::Pool<MySql>) -> Result<Self> {
        sqlx::query_as!(
            Self,
            "SELECT
id,
name,
phone,
birthdate,
sex as `sex: Sex`,
town_id,
verification_result as `verification_result: _`,
verification_type as `verification_type: _`,
verification_picture_url,
picture,
bio,
deleted as `deleted: _`,
refresh_token,
(SELECT COUNT(*) FROM user_like as ul WHERE ul.target_id = u.id) as `total_likes!`,
created_at,
updated_at
FROM user as u WHERE phone = ?",
            phone
        )
        .fetch_one(db)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => Error::UserNotFound(phone.to_string()),
            _ => Error::Database(err),
        })
        .and_then(|user| match user.deleted {
            false => Ok(user),
            true => Err(Error::DeletedUser),
        })
    }

    pub(crate) async fn to_schema(&self, db: &sqlx::Pool<MySql>) -> Result<UserSchema> {
        let town = Town::from_id(self.town_id, db).await?;

        Ok(UserSchema {
            id: self.id,
            name: self.name.clone(),
            phone: self.phone.clone(),
            birthdate: self.birthdate,
            sex: self.sex.to_string(),
            town,
            verification_result: self.verification_result,
            verification_type: self.verification_type.map(|value| value.to_string()),
            verification_picture_url: self.verification_picture_url.clone(),
            picture: self.picture.clone(),
            bio: self.bio.clone().unwrap_or_default(),
            total_likes: self.total_likes,
        })
    }

    pub(crate) async fn to_other_user_schema(
        &self,
        requester: &User,
        db: &sqlx::Pool<MySql>,
    ) -> Result<OtherUserSchema> {
        if self.deleted {
            panic!("trying to return deleted user information")
        }

        let town = Town::from_id(self.town_id, db).await?;
        let my_like = sqlx::query!(
            "SELECT id FROM user_like WHERE issuer_id = ? AND target_id = ? LIMIT 1",
            requester.id,
            self.id
        )
        .fetch_optional(db)
        .await?
        .is_some();

        Ok(OtherUserSchema {
            id: self.id,
            name: self.name.clone(),
            phone: self.phone.clone(),
            birthdate: self.birthdate,
            sex: self.sex.to_string(),
            town,
            verification_result: self.verification_result,
            picture: self.picture.clone(),
            bio: self.bio.clone().unwrap_or_default(),
            total_likes: self.total_likes,
            my_like,
        })
    }

    pub(crate) async fn update_refresh_token(
        &self,
        token: &str,
        db: &sqlx::Pool<MySql>,
    ) -> Result<()> {
        sqlx::query!("UPDATE user SET refresh_token = ? WHERE id = ?", token, self.id)
            .execute(db)
            .await?;

        Ok(())
    }

    pub(crate) fn verify_refresh_token(&self, refresh_token: &str) -> Result<()> {
        if refresh_token.is_empty() {
            return Err(Error::InvalidToken);
        }

        if let Some(user_token) = &self.refresh_token {
            if user_token != refresh_token {
                return Err(Error::InvalidToken);
            }
        } else {
            return Err(Error::InvalidToken);
        }

        Ok(())
    }

    pub(crate) async fn update_bio(&mut self, bio: &str, db: &sqlx::Pool<MySql>) -> Result<()> {
        Ok(sqlx::query!("UPDATE user SET bio = ? WHERE id = ?", bio, self.id)
            .execute(db)
            .await
            .map(|_| {
                self.bio = Some(bio.to_string());
            })?)
    }

    pub(crate) async fn update_picture(
        &mut self,
        picture: FieldData<NamedTempFile>,
        s3: &aws::S3Client,
        db: &sqlx::Pool<MySql>,
    ) -> Result<String> {
        let picture_path = PicturePath::generate(self.id).await?;

        picture.contents.persist(&picture_path.file_path).map_err(|err| Error::PersistFile {
            path: picture_path.file_path.to_path_buf(),
            source: err.into(),
        })?;

        let picture_url = s3.push_file(&picture_path.file_path, &picture_path.upload_path).await?;

        sqlx::query!("UPDATE user SET picture = ? WHERE id = ?", picture_url, self.id)
            .execute(db)
            .await?;

        Ok(picture_url)
    }

    pub(crate) async fn like_user(&self, target: &User, db: &sqlx::Pool<MySql>) -> Result<()> {
        sqlx::query!(
            "INSERT INTO user_like (issuer_id, target_id) VALUES (?, ?)",
            self.id,
            target.id
        )
        .execute(db)
        .await?;
        Ok(())
    }

    pub(crate) async fn like_post(&self, post: &Post, db: &sqlx::Pool<MySql>) -> Result<()> {
        sqlx::query!("INSERT INTO post_like (user_id, post_id) VALUES (?, ?)", self.id, post.id())
            .execute(db)
            .await?;
        Ok(())
    }

    pub(crate) async fn cancel_like_user(
        &self,
        target: &User,
        db: &sqlx::Pool<MySql>,
    ) -> Result<()> {
        sqlx::query!(
            "DELETE FROM user_like WHERE issuer_id = ? AND target_id = ?",
            self.id,
            target.id
        )
        .execute(db)
        .await?;
        Ok(())
    }

    pub(crate) async fn cancel_like_post(&self, post: &Post, db: &sqlx::Pool<MySql>) -> Result<()> {
        sqlx::query!("DELETE FROM post_like WHERE user_id = ? AND post_id = ?", self.id, post.id())
            .execute(db)
            .await?;
        Ok(())
    }

    pub(crate) async fn is_blocked(&self, blocker: &User, db: &sqlx::Pool<MySql>) -> Result<bool> {
        Ok(sqlx::query!(
            "SELECT id FROM user_block WHERE user_id = ? AND target_id = ?",
            blocker.id,
            self.id,
        )
        .fetch_optional(db)
        .await?
        .is_some())
    }

    pub(crate) async fn block_user(&self, target: &User, db: &sqlx::Pool<MySql>) -> Result<()> {
        sqlx::query!(
            "INSERT INTO user_block (user_id, target_id) VALUES (?, ?)",
            self.id,
            target.id
        )
        .execute(db)
        .await?;

        Ok(())
    }

    pub(crate) async fn unblock_user(&self, target: &User, db: &sqlx::Pool<MySql>) -> Result<()> {
        sqlx::query!(
            "DELETE FROM user_block WHERE user_id = ? AND target_id = ?",
            self.id,
            target.id
        )
        .execute(db)
        .await?;

        Ok(())
    }

    pub(crate) async fn block_post(&self, post: &Post, db: &sqlx::Pool<MySql>) -> Result<()> {
        sqlx::query!("INSERT INTO post_block (user_id, post_id) VALUES (?, ?)", self.id, post.id())
            .execute(db)
            .await?;

        Ok(())
    }

    pub(crate) async fn unblock_post(&self, post: &Post, db: &sqlx::Pool<MySql>) -> Result<()> {
        sqlx::query!(
            "DELETE FROM post_block WHERE user_id = ? AND post_id = ?",
            self.id,
            post.id()
        )
        .execute(db)
        .await?;

        Ok(())
    }

    pub(crate) async fn block_post_comment(
        &self,
        comment: &Comment,
        db: &sqlx::Pool<MySql>,
    ) -> Result<()> {
        sqlx::query!(
            "INSERT INTO post_comment_block (user_id, comment_id) VALUES (?, ?)",
            self.id,
            comment.id()
        )
        .execute(db)
        .await?;

        Ok(())
    }

    pub(crate) async fn unblock_post_comment(
        &self,
        comment: &Comment,
        db: &sqlx::Pool<MySql>,
    ) -> Result<()> {
        sqlx::query!(
            "DELETE FROM post_comment_block WHERE user_id = ? AND comment_id = ?",
            self.id,
            comment.id()
        )
        .execute(db)
        .await?;

        Ok(())
    }

    pub(crate) async fn update_verification(
        &mut self,
        verification_type: IdVerificationType,
        picture: FieldData<NamedTempFile<File>>,
        db: &sqlx::Pool<MySql>,
        s3: &aws::S3Client,
    ) -> Result<String> {
        if self.is_verified() {
            return Err(Error::InvalidRequest);
        }

        self.delete_verification_picture(s3).await?;

        sqlx::query!(
            "UPDATE user SET verification_result = ?, verification_picture_url = NULL WHERE id = ?",
            VerificationResult::NotVerified,
            self.id
        )
        .execute(db)
        .await?;

        self.verification_result = VerificationResult::NotVerified;
        self.verification_picture_url = None;

        let url = Self::upload_verification_picture(picture, s3).await?;

        sqlx::query!(
            "UPDATE user SET verification_type = ?, verification_picture_url = ? WHERE id = ?",
            verification_type,
            url,
            self.id
        )
        .execute(db)
        .await?;

        self.verification_type = Some(verification_type);
        self.verification_picture_url = Some(url.clone());

        Ok(url)
    }

    pub(crate) async fn treat_as_deleted(self, db: &sqlx::Pool<MySql>) -> Result<()> {
        sqlx::query!("UPDATE user SET deleted = TRUE WHERE id = ?", self.id).execute(db).await?;

        Ok(())
    }

    pub(crate) fn id(&self) -> UserId {
        self.id
    }

    pub(crate) fn is_verified(&self) -> bool {
        matches!(self.verification_result, VerificationResult::Verified)
    }

    pub(crate) fn town_id(&self) -> TownId {
        self.town_id
    }

    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn picture(&self) -> &str {
        &self.picture
    }

    pub(crate) fn created_at(&self) -> DateTime<Utc> {
        self.created_at
    }

    async fn upload_verification_picture(
        photo: FieldData<NamedTempFile<File>>,
        s3: &aws::S3Client,
    ) -> Result<String> {
        let basename: String =
            rand::thread_rng().sample_iter(Alphanumeric).take(32).map(char::from).collect();
        let dir = std::env::temp_dir().join(std::env!("CARGO_PKG_NAME"));
        let temp_path = dir.join(&basename);

        fs::create_dir_all(&dir)
            .await
            .map_err(|err| Error::Io { path: dir.to_path_buf(), source: err })?;
        photo
            .contents
            .persist(&temp_path)
            .map_err(|err| Error::PersistFile { path: temp_path.clone(), source: err.into() })?;

        s3.push_file(&temp_path, &(String::from(VERIFICATION_PHOTO_PATH) + &basename)).await
    }

    async fn delete_verification_picture(&mut self, s3: &aws::S3Client) -> Result<()> {
        let picture_url = self.verification_picture_url.clone();

        match picture_url {
            Some(picture_url) if !picture_url.is_empty() => {
                s3.delete_file(&picture_url).await?;

                self.verification_picture_url = None;

                Ok(())
            }
            _ => Ok(()),
        }
    }
}

struct PicturePath {
    file_path: PathBuf,
    upload_path: String,
}

impl PicturePath {
    async fn generate(user_id: UserId) -> Result<Self> {
        let temp_dir = std::env::temp_dir().join(env!("CARGO_PKG_NAME"));

        fs::create_dir_all(&temp_dir)
            .await
            .or_else(|error| match error.kind() {
                io::ErrorKind::AlreadyExists => Ok(()),
                _ => Err(error),
            })
            .map_err(|err| Error::Io { path: temp_dir.to_path_buf(), source: err })?;

        let s3_path = format!("profile_image/{}", user_id);

        Ok(PicturePath { file_path: temp_dir.join(user_id.to_string()), upload_path: s3_path })
    }
}
