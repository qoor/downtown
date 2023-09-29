// Copyright 2023. The downtown authors all rights reserved.

use chrono::{DateTime, Utc};
use serde::Serialize;
use sqlx::MySql;

use crate::Result;

pub(crate) type TownId = u64;

#[derive(Debug, sqlx::FromRow, Serialize)]
pub struct Town {
    id: TownId,
    address: String,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl Town {
    pub(crate) async fn from_address(address: &str, db: &sqlx::Pool<MySql>) -> Result<Self> {
        let mut town = Self::from_address_internal(address, db).await?;

        if town.is_none() {
            Self::create(address, db).await?;
            town = Self::from_address_internal(address, db).await?;
        }

        Ok(town.ok_or(sqlx::Error::RowNotFound)?)
    }

    pub(crate) async fn from_id(id: TownId, db: &sqlx::Pool<MySql>) -> Result<Self> {
        Ok(sqlx::query_as!(Self, "SELECT * FROM town WHERE id = ?", id).fetch_one(db).await?)
    }

    pub(crate) fn id(&self) -> TownId {
        self.id
    }

    async fn from_address_internal(address: &str, db: &sqlx::Pool<MySql>) -> Result<Option<Self>> {
        Ok(sqlx::query_as!(Self, "SELECT * FROM town WHERE address = ?", address)
            .fetch_optional(db)
            .await?)
    }

    async fn create(address: &str, db: &sqlx::Pool<MySql>) -> Result<TownId> {
        Ok(sqlx::query!("INSERT INTO town (address) VALUES (?)", address)
            .execute(db)
            .await
            .map(|row| row.last_insert_id())?)
    }
}
