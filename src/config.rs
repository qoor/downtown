// Copyright 2023. The downtown authors all rights reserved.

use crate::env::get_env_or_panic;

#[derive(Clone)]
pub struct Config {
    address: String,
    port: u16,
}

impl Config {
    pub fn new() -> Self {
        let port: u16 = get_env_or_panic("PORT").parse().unwrap();

        Self {
            address: format!("0.0.0.0:{port}"),
            port
        }
    }

    pub fn address(&self) -> &str {
        &self.address
    }

    pub fn port(&self) -> u16 {
        self.port
    }
}
