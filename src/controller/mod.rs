use rocket::request::FromParam;
use rocket::response::{self, Responder};
use rocket::http::hyper::header::{AccessControlAllowOrigin, AccessControlAllowMethods,
                                  AccessControlAllowHeaders, AccessControlAllowCredentials};
use hyper::method::Method;
use uuid::Uuid;
use unicase::UniCase;

use std::str::FromStr;
use std::path::PathBuf;

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

pub struct CORS<R>(R);

impl<'r, R: Responder<'r>> Responder<'r> for CORS<R> {
    fn respond(self) -> response::Result<'r> {
        let mut response = self.0.respond()?;
        response.set_header(AccessControlAllowOrigin::Any);
        response.set_header(AccessControlAllowMethods(vec![Method::Get,
                                                           Method::Post,
                                                           Method::Put,
                                                           Method::Delete,
                                                           Method::Options]));
        response.set_header(AccessControlAllowHeaders(vec![UniCase("Authorization".to_string()),
                                                           UniCase("Content-Type".to_string())]));
        response.set_header(AccessControlAllowCredentials);
        Ok(response)
    }
}

#[options("/<path..>")]
pub fn options(path: PathBuf) -> CORS<()> {
    CORS(())
}
