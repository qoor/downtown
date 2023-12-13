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
const ALIGO_MESSAGE_PREFIX: &str = "이프 회원가입을 위해 인증번호 ['";
const ALIGO_MESSAGE_SUFFIX: &str = "]를 입력해주세요.'";
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
    info: AligoSendInfo,
}

#[derive(Deserialize)]
struct AligoSendInfo {
    #[serde(rename = "type")]
    #[allow(dead_code)]
    send_type: String,
    #[allow(dead_code)]
    mid: i64,
    #[allow(dead_code)]
    current: f64,
    #[allow(dead_code)]
    unit: f64,
    #[allow(dead_code)]
    total: f64,
    #[allow(dead_code)]
    scnt: i64,
    #[allow(dead_code)]
    fcnt: i64,
}

impl PhoneAuthentication {
    // TODO: Send a verification code message to user
    pub(crate) async fn send(phone: &str, db: &sqlx::Pool<MySql>) -> Result<Self> {
        let tx = db.begin().await?;

        Self::cancel(phone, db).await?;

        let code = Self::generate_random_code();

        let mut token_url = ALIGO_HOST
            .join(ALIGO_TOKEN_CREATE_PATH)?
            .join("s/")?
            .join(&ALIGO_TOKEN_LIFETIME_SEC.to_string())?;
        token_url
            .query_pairs_mut()
            .append_pair("apikey", ALIGO_API_KEY)
            .append_pair("userid", ALIGO_USER_ID)
            .finish();
        println!("{}", token_url);
        let token: String = reqwest::Client::new()
            .post(token_url)
            .send()
            .await?
            .json::<AligoTokenCreationResult>()
            .await
            .map_err(Error::from)
            .and_then(|result| match result.code {
                0 => Ok(result),
                _ => Err(Error::MessageSend(result.code)),
            })
            .map(|result| result.token)?;

        let mut send_url = ALIGO_HOST.join(ALIGO_SEND_PATH)?;

        send_url
            .query_pairs_mut()
            .append_pair("apikey", ALIGO_API_KEY)
            .append_pair("userid", ALIGO_USER_ID)
            .append_pair("token", &token)
            .append_pair("sender_key", ALIGO_SENDER_KEY)
            .append_pair("tpl_code", ALIGO_TEMPLATE_CODE)
            .append_pair("sender", ALIGO_SENDER_PHONE)
            // .append_pair("senddate", todo!())
            .append_pair("receiver_1", phone)
            // .append_pair("recvname_1", todo!())
            .append_pair("subject_1", ALIGO_MESSAGE_SUBJECT)
            .append_pair(
                "message_1",
                &format!("{ALIGO_MESSAGE_PREFIX}{code}{ALIGO_MESSAGE_SUFFIX}"),
            )
            .append_pair("failover", "N")
            .append_pair("testMode", if ALIGO_TEST_MODE { "Y" } else { "N" })
            .finish();
        println!("{}", send_url);

        reqwest::Client::new()
            .post(send_url)
            .send()
            .await?
            .json::<AligoSendResult>()
            .await
            .map_err(Error::from)
            .and_then(|result| match result.code {
                0 => Ok(result),
                _ => Err(Error::MessageSend(result.code)),
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
