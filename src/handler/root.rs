// Copyright 2023. The downtown authors all rights reserved.

use crate::about;

pub(crate) async fn root() -> String {
    about()
}
