#![recursion_limit = "1024"]

extern crate rustless;
extern crate iron;
extern crate valico;
extern crate email;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate error_chain;
extern crate postgres;
extern crate rand;
extern crate chrono;
extern crate lettre;
extern crate log;
extern crate env_logger;
extern crate uuid;
extern crate hyper;
extern crate hyper_native_tls;
extern crate serde_json;

extern crate brdgme_db;
extern crate brdgme_cmd;

use rustless::{Application, Api, Nesting, Versioning, Response};
use rustless::server::status::StatusCode;
use rustless::batteries::swagger;
use rustless::errors::ErrorResponse;

use std::default::Default;

mod config;
mod auth;
mod game;
mod mail;

mod errors {
    error_chain!{
        links {
            Db(::brdgme_db::errors::Error, ::brdgme_db::errors::ErrorKind);
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
}

lazy_static! {
    pub static ref CONN: brdgme_db::Connections = brdgme_db::connect_env().unwrap();
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
