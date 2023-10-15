// Copyright 2023. The downtown authors all rights reserved.

use axum_typed_multipart::{FieldData, TryFromMultipart};
use chrono::NaiveDate;
use serde::Serialize;
use tempfile::NamedTempFile;

use crate::{
    post::PostId,
    town::Town,
    user::{self, account::UserId, IdVerificationType},
};

#[derive(TryFromMultipart)]
pub struct RegistrationSchema {
    pub authorization_code: String,
    pub name: String,
    pub birthdate: String,
    pub sex: user::Sex,
    pub phone: String,
    pub address: String,
    pub verification_type: IdVerificationType,
    pub verification_photo: FieldData<NamedTempFile>,
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
    pub verification_type: String,
    pub verification_photo_url: String,
    pub picture: String,
    pub bio: String,
}

#[derive(Serialize)]
pub struct TokenSchema {
    pub user_id: UserId,
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(TryFromMultipart)]
pub struct ProfilePictureUpdateSchema {
    pub picture: FieldData<NamedTempFile>,
}

#[derive(TryFromMultipart)]
pub struct ProfileBioUpdateSchema {
    pub bio: String,
}

#[derive(TryFromMultipart)]
pub struct PostCreationSchema {
    pub author_id: UserId,
    pub content: String,
    pub images: Vec<FieldData<NamedTempFile>>,
}

#[derive(TryFromMultipart)]
pub struct PostEditSchema {
    pub content: String,
    pub images: Vec<FieldData<NamedTempFile>>,
}

#[derive(Serialize)]
pub struct PostResultSchema {
    pub post_id: PostId,
    pub author_id: UserId,
}
