// Copyright 2023. The downtown authors all rights reserved.

use chrono::{DateTime, Utc};
use rand::Rng;
use sqlx::MySql;

use crate::{Error, Result};

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct PhoneVerification {
    id: u64,
    phone: String,
    code: String,
    created_at: DateTime<Utc>,
}

impl PhoneVerification {
    // TODO: Send a verification code message to user
    pub(crate) async fn send(phone: &str, db: &sqlx::Pool<MySql>) -> Result<Self> {
        let tx = db.begin().await?;

        Self::cancel(phone, db).await?;

        let code = format!("{:06}", rand::thread_rng().gen_range(100000..999999));

        sqlx::query!("INSERT INTO phone_verification (phone, code) VALUES (?, ?)", phone, code)
            .execute(db)
            .await?;

        let result = Self::from_phone(phone, db).await;

        tx.commit().await?;

        result
    }

    pub(crate) async fn verify(phone: &str, code: &str, db: &sqlx::Pool<MySql>) -> Result<()> {
        let data = Self::from_phone(phone, db).await?;

        match (Utc::now() - data.created_at).num_minutes() {
            minutes if minutes < 30 => match data.code == code {
                true => Ok(()),
                false => Err(Error::Verification),
            },
            _ => Err(Error::VerificationExpired),
        }
    }

    pub(crate) async fn cancel(phone: &str, db: &sqlx::Pool<MySql>) -> Result<()> {
        Ok(sqlx::query!("DELETE FROM phone_verification WHERE phone = ?", phone)
            .execute(db)
            .await
            .map(|_| ())?)
    }

    async fn from_phone(phone: &str, db: &sqlx::Pool<MySql>) -> Result<Self> {
        sqlx::query_as!(Self, "SELECT * FROM phone_verification WHERE phone = ?", phone)
            .fetch_one(db)
            .await
            .map_err(|error| match error {
                sqlx::Error::RowNotFound => Error::Verification,
                _ => Error::Database(error),
            })
    }
}
