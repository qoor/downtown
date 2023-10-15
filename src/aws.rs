// Copyright 2023. The downtown authors all rights reserved.

use std::path::Path;

use aws_sdk_s3::primitives::ByteStream;

use crate::{env::get_env_or_panic, Error, Result};

pub(crate) struct S3Client {
    client: aws_sdk_s3::Client,
    region: String,
    bucket: String,
}

impl S3Client {
    pub async fn from_env() -> Self {
        let aws_config = aws_config::load_from_env().await;

        Self {
            client: aws_sdk_s3::Client::new(&aws_config),
            region: aws_config.region().unwrap().to_string(),
            bucket: get_env_or_panic("AWS_S3_BUCKET"),
        }
    }

    pub async fn push_file(&self, file_path: &Path, target_path: &str) -> Result<String> {
        let body = ByteStream::from_path(&file_path).await.map_err(|err| Error::FileToStream {
            path: file_path.to_path_buf(),
            source: Box::new(err),
        })?;

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(target_path)
            .body(body)
            .send()
            .await
            .map_err(|err| Error::Upload {
                path: file_path.to_path_buf(),
                source: Box::new(err),
            })?;

        Ok(format!("https://{}.s3.{}.amazonaws.com/{}", self.bucket, self.region, target_path))
    }

    pub async fn delete_file(&self, target_path: &str) -> Result<String> {
        self.client.delete_object().bucket(&self.bucket).key(target_path).send().await.unwrap();

        Ok(format!("https://{}.s3.{}.amazonaws.com/{}", self.bucket, self.region, target_path))
    }
}
