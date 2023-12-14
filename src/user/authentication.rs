// Copyright 2023. The downtown authors all rights reserved.

use chrono::{DateTime, Utc};
use once_cell::sync;
use rand::Rng;
use reqwest::Url;
use serde::Deserialize;
use sqlx::MySql;

use crate::{Error, Result};

static ALIGO_HOST: sync::Lazy<Url> =
    sync::Lazy::new(|| Url::parse("https://kakaoapi.aligo.in").unwrap());

const ALIGO_TOKEN_CREATE_PATH: &str = "akv10/token/create/";
const ALIGO_SEND_PATH: &str = "akv10/alimtalk/send/";

const ALIGO_TOKEN_LIFETIME_SEC: i32 = 30;

const ALIGO_API_KEY: &str = "5huac2s03ek6bz2k94ysnh1y3nju3laz";
const ALIGO_USER_ID: &str = "chysaek";
const ALIGO_SENDER_KEY: &str = "ed4c71c275d2cf3a12a238ee74ae9d025b878905";
const ALIGO_TEMPLATE_CODE: &str = "TQ_0877";
const ALIGO_SENDER_PHONE: &str = "01088074946";
const ALIGO_MESSAGE_SUBJECT: &str = "이프 휴대폰 인증";
const ALIGO_MESSAGE_PREFIX: &str = "이프 회원가입을 위해 인증번호 [";
const ALIGO_MESSAGE_SUFFIX: &str = "]를 입력해주세요.";
const ALIGO_TEST_MODE: bool = true;

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct PhoneAuthentication {
    #[allow(dead_code)]
    id: u64,
    #[allow(dead_code)]
    phone: String,
    code: String,
    created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
struct AligoTokenCreationResult {
    code: i32,
    #[allow(dead_code)]
    message: String,
    token: String,
    /// URL encoded token
    #[allow(dead_code)]
    urlencode: String,
}

#[derive(Deserialize)]
struct AligoSendResult {
    code: i32,
    #[allow(dead_code)]
    message: String,
    #[allow(dead_code)]
    info: Option<AligoSendInfo>,
}

#[derive(Deserialize)]
struct AligoSendInfo {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    send_type: String,
    #[allow(dead_code)]
    mid: Option<i64>,
    #[allow(dead_code)]
    current: String,
    #[allow(dead_code)]
    unit: f64,
    #[allow(dead_code)]
    total: f64,
    #[allow(dead_code)]
    scnt: Option<i64>,
    #[allow(dead_code)]
    fcnt: Option<i64>,
}

impl PhoneAuthentication {
    // TODO: Send a verification code message to user
    pub(crate) async fn send(phone: &str, db: &sqlx::Pool<MySql>) -> Result<Self> {
        let tx = db.begin().await?;

        Self::cancel(phone, db).await?;

        let code = Self::generate_random_code();

        let body = [("apikey", ALIGO_API_KEY), ("userid", ALIGO_USER_ID)];
        let token: String = reqwest::Client::new()
            .post(
                ALIGO_HOST
                    .join(ALIGO_TOKEN_CREATE_PATH)?
                    .join(&format!("{}/", ALIGO_TOKEN_LIFETIME_SEC))?
                    .join("s/")?,
            )
            .form(&body)
            .send()
            .await?
            .json::<AligoTokenCreationResult>()
            .await
            .map_err(Error::from)
            .and_then(|result| match result.code {
                0 => Ok(result),
                _ => Err(Error::MessageSend { code: result.code, message: result.message }),
            })
            .map(|result| result.token)?;

        let body = [
            ("apikey", ALIGO_API_KEY),
            ("userid", ALIGO_USER_ID),
            ("token", &token),
            ("senderkey", ALIGO_SENDER_KEY),
            ("tpl_code", ALIGO_TEMPLATE_CODE),
            ("sender", ALIGO_SENDER_PHONE),
            ("receiver_1", phone),
            ("subject_1", ALIGO_MESSAGE_SUBJECT),
            ("message_1", &format!("{ALIGO_MESSAGE_PREFIX}{code}{ALIGO_MESSAGE_SUFFIX}")),
            ("failover", "N"),
            ("testMode", if ALIGO_TEST_MODE { "Y" } else { "N" }),
        ];
        reqwest::Client::new()
            .post(ALIGO_HOST.join(ALIGO_SEND_PATH)?)
            .form(&body)
            .send()
            .await?
            .json::<AligoSendResult>()
            .await
            .map_err(Error::from)
            .and_then(|result| match result.code {
                0 => Ok(result),
                _ => Err(Error::MessageSend { code: result.code, message: result.message }),
            })?;

        sqlx::query!("INSERT INTO phone_authorization (phone, code) VALUES (?, ?)", phone, code)
            .execute(db)
            .await?;

        let result = Self::from_phone(phone, db).await;

        tx.commit().await?;

        result
    }

    pub(crate) async fn authorize(phone: &str, code: &str, db: &sqlx::Pool<MySql>) -> Result<()> {
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
        Ok(sqlx::query!("DELETE FROM phone_authorization WHERE phone = ?", phone)
            .execute(db)
            .await
            .map(|_| ())?)
    }

    pub(crate) fn code(&self) -> &str {
        &self.code
    }

    async fn from_phone(phone: &str, db: &sqlx::Pool<MySql>) -> Result<Self> {
        sqlx::query_as!(Self, "SELECT * FROM phone_authorization WHERE phone = ?", phone)
            .fetch_one(db)
            .await
            .map_err(|error| match error {
                sqlx::Error::RowNotFound => Error::Verification,
                _ => Error::Database(error),
            })
    }

    fn generate_random_code() -> String {
        format!("{:06}", rand::thread_rng().gen_range(100000..999999))
    }
}
