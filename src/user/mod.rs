// Copyright 2023. The downtown authors all rights reserved.

pub(crate) mod account;
pub(crate) mod jwt;
pub(crate) mod verification;

use std::str::FromStr;

use axum_typed_multipart::TryFromField;

#[derive(Debug, TryFromField, sqlx::Type, Clone, Copy)]
#[repr(u32)]
#[try_from_field(rename_all = "snake_case")]
pub enum Sex {
    Male = 1,
    Female = 2,
}

impl FromStr for Sex {
    type Err = crate::Error;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "male" => Ok(Sex::Male),
            "female" => Ok(Sex::Female),
            _ => Err(crate::Error::CannotParse {
                value: s.to_string(),
                type_name: String::from("Sex"),
            }),
        }
    }
}

impl std::fmt::Display for Sex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            Sex::Male => "male",
            Sex::Female => "female",
        };

        write!(f, "{s}")
    }
}

#[derive(Debug, TryFromField, sqlx::Type, Clone, Copy)]
#[repr(u32)]
#[try_from_field(rename_all = "snake_case")]
pub enum IdVerificationType {
    IdCard = 1,
    DriverLicense = 2,
    ResidentRegister = 3,
}

impl std::fmt::Display for IdVerificationType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            IdVerificationType::IdCard => "id_card",
            IdVerificationType::DriverLicense => "driver_license",
            IdVerificationType::ResidentRegister => "resident_register",
        };

        write!(f, "{s}")
    }
}
