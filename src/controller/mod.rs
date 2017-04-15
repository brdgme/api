use rocket::request::FromParam;
use uuid::Uuid;

use std::str::FromStr;

pub mod auth;
pub mod game;
pub mod mail;

use errors::*;

pub struct UuidParam(Uuid);

impl UuidParam {
    pub fn into_uuid(self) -> Uuid {
        self.0
    }
}

impl<'a> FromParam<'a> for UuidParam {
    type Error = Error;

    fn from_param(param: &'a str) -> Result<Self> {
        Ok(UuidParam(Uuid::from_str(param)
                         .chain_err(|| "failed to parse UUID")?))
    }
}
