#![recursion_limit = "1024"]

extern crate rustless;
extern crate iron;
extern crate valico;
extern crate email;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate error_chain;
#[macro_use]
extern crate postgres;
#[macro_use]
extern crate postgres_derive;
extern crate r2d2;
extern crate r2d2_postgres;
extern crate rand;
extern crate chrono;
extern crate lettre;
extern crate log;
extern crate env_logger;
extern crate uuid;
extern crate hyper;
extern crate hyper_native_tls;
extern crate serde_json;

extern crate brdgme_cmd;
extern crate brdgme_game;
extern crate brdgme_color;

use rustless::{Application, Api, Nesting, Versioning, Response};
use rustless::server::status::StatusCode;
use rustless::batteries::swagger;
use rustless::errors::ErrorResponse;

use std::default::Default;
use std::env;

mod config;
mod auth;
mod game;
mod mail;
mod db;

mod errors {
    error_chain!{
        foreign_links {
            Postgres(::postgres::error::Error);
            EnvVar(::std::env::VarError);
            Chrono(::chrono::ParseError);
        }

        errors {
            UserError(message: String) {
                description("user error")
                display("{}", message)
            }
        }
    }

    impl From<Error> for ::ErrorResponse {
        fn from(e: Error) -> Self {
            Self {
                error: Box::new(e),
                response: None,
            }
        }
    }

    pub fn err_resp(msg: &str) -> ::ErrorResponse {
        let err: Error = ErrorKind::Msg(msg.to_owned()).into();
        err.into()
    }
}
use errors::*;

lazy_static! {
    pub static ref CONN: db::Connections = db::connect_env().unwrap();
}

fn main() {
    env_logger::init().unwrap();
    let api = Api::build(|api| {
        api.prefix("api");
        api.mount(swagger::create_api("docs"));
        api.mount(Api::build(|v1| {
                                 v1.version("v1", Versioning::Path);
                                 v1.namespace("auth", auth::namespace);
                                 v1.namespace("game", game::namespace);
                                 v1.namespace("mail", mail::namespace);
                             }));
        api.error_formatter(|err, _media| match err.downcast::<auth::UnauthorizedError>() {
                                Some(_) => Some(Response::new(StatusCode::Unauthorized)),
                                None => None,
                            });
        api.error_formatter(|err, _media| match err.downcast::<Error>() {
                                Some(&Error(ErrorKind::UserError(ref message), _)) => {
                                    Some(Response::from(StatusCode::BadRequest,
                                                        Box::new(message.to_owned())))
                                }
                                _ => None,
                            });
    });
    let mut app = Application::new(api);
    swagger::enable(&mut app,
                    swagger::Spec {
                        info: swagger::Info {
                            title: "brdg.me API".to_string(),
                            ..Default::default()
                        },
                        ..Default::default()
                    });

    iron::Iron::new(app).http("0.0.0.0:8000").unwrap();
}
