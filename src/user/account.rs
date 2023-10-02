// Copyright 2023. The downtown authors all rights reserved.

use chrono::{DateTime, NaiveDate, Utc};
use sqlx::MySql;

use crate::{
    schema::{RegistrationSchema, UserSchema},
    town::{Town, TownId},
    Error, Result,
};

use super::{IdVerificationType, Sex};

pub(crate) type UserId = u64;

#[derive(Debug, sqlx::FromRow, Clone)]
pub(crate) struct User {
    id: UserId,
    name: String,
    phone: String,
    birthdate: NaiveDate,
    sex: Sex,
    town_id: TownId,
    verification_type: IdVerificationType,
    verification_photo_url: String,
    refresh_token: Option<String>,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl User {
    pub(crate) async fn register(
        data: &RegistrationSchema,
        db: &sqlx::Pool<MySql>,
    ) -> Result<Self> {
        let tx = db.begin().await?;

        let town_id = Town::from_address(&data.address, db).await.map(|town| town.id())?;

        let user_id = sqlx::query!(
            "INSERT INTO user (
name,
phone,
birthdate,
sex,
town_id,
verification_type,
verification_photo_url) VALUES (
?,
?,
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
            data.verification_type,
            ""
        )
        .execute(db)
        .await
        .map(|row| row.last_insert_id())?;

        let user = Self::from_id(user_id, db).await;

        tx.commit().await?;

        user
    }

    pub(crate) async fn from_id(id: UserId, db: &sqlx::Pool<MySql>) -> Result<Self> {
        Ok(sqlx::query_as!(
            Self,
            "SELECT
id,
name,
phone,
birthdate,
sex as `sex: Sex`,
town_id,
verification_type as `verification_type: IdVerificationType`,
verification_photo_url,
refresh_token,
created_at,
updated_at
FROM user WHERE id = ?",
            id
        )
        .fetch_one(db)
        .await?)
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
verification_type as `verification_type: IdVerificationType`,
verification_photo_url,
refresh_token,
created_at,
updated_at
FROM user WHERE phone = ?",
            phone
        )
        .fetch_one(db)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => Error::UserNotFound(phone.to_string()),
            _ => Error::Database(err),
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
            verification_type: self.verification_type.to_string(),
            verification_photo_url: self.verification_photo_url.to_string(),
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

    pub(crate) fn id(&self) -> UserId {
        self.id
    }
}
