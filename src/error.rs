// Copyright 2023. The downtown authors all rights reserved.

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("cannot parse value {value} to {type_name} type")]
    CannotParse { value: String, type_name: String },
}
