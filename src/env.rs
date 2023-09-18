// Copyright 2023. The downtown authors all rights reserved.

pub fn get_env_or_panic(env: &str) -> String {
    std::env::var(env).unwrap_or_else(|_| panic!("{env} must be set"))
}
