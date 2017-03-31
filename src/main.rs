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
extern crate env_logger;

extern crate brdgme_db;

use rustless::{Application, Api, Nesting, Versioning};
use rustless::batteries::swagger;
use rustless::errors::{Error, ErrorResponse};

use std::default::Default;

mod config;
mod auth;
mod game;
mod mail;

mod errors {
    error_chain!{}
}

lazy_static! {
    pub static ref CONN: brdgme_db::Connections = brdgme_db::connect_env().unwrap();
}

pub fn to_error_response<T: Error + Send>(e: T) -> ErrorResponse {
    ErrorResponse {
        error: Box::new(e),
        response: None,
    }
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
