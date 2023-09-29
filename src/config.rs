// Copyright 2023. The downtown authors all rights reserved.

use std::path::PathBuf;

use jsonwebtoken::{DecodingKey, EncodingKey};

use crate::env::get_env_or_panic;

#[derive(Clone)]
pub struct Config {
    address: String,
    port: u16,

    access_token_max_age: i64,
    refresh_token_max_age: i64,

    private_key: RsaKey,
    public_key: RsaKey,
}

#[derive(Clone)]
pub(crate) struct RsaKey {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
}

impl RsaKey {
    fn from_file(path: &std::path::PathBuf) -> std::io::Result<Self> {
        match std::fs::read_to_string(path) {
            Ok(key) => Ok(Self {
                encoding_key: EncodingKey::from_rsa_pem(key.as_bytes()).unwrap(),
                decoding_key: DecodingKey::from_rsa_pem(key.as_bytes()).unwrap(),
            }),
            Err(err) => Err(err),
        }
    }

    pub(crate) fn encoding_key(&self) -> &EncodingKey {
        &self.encoding_key
    }

    pub(crate) fn decoding_key(&self) -> &DecodingKey {
        &self.decoding_key
    }
}

impl Config {
    pub fn new() -> Self {
        let port: u16 = get_env_or_panic("PORT").parse().unwrap();

        Self {
            address: format!("0.0.0.0:{port}"),
            port,

            access_token_max_age: get_env_or_panic("ACCESS_TOKEN_MAX_AGE").parse().unwrap(),
            refresh_token_max_age: get_env_or_panic("REFRESH_TOKEN_MAX_AGE").parse().unwrap(),

            private_key: RsaKey::from_file(
                &PathBuf::from(get_env_or_panic("RSA_PRIVATE_PEM_FILE_PATH")).to_path_buf(),
            )
            .expect("Cannot open the private key file"),
            public_key: RsaKey::from_file(
                &PathBuf::from(get_env_or_panic("RSA_PUBLIC_PEM_FILE_PATH")).to_path_buf(),
            )
            .expect("Cannot open the public key file"),
        }
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn private_key(&self) -> &EncodingKey {
        self.private_key.encoding_key()
    }

    pub fn public_key(&self) -> &DecodingKey {
        self.public_key.decoding_key()
    }

    pub fn access_token_max_age(&self) -> i64 {
        self.access_token_max_age
    }

    pub fn refresh_token_max_age(&self) -> i64 {
        self.refresh_token_max_age
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}
