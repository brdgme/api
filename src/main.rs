#![recursion_limit = "1024"]
#![allow(dead_code)]
#![allow(unused_variables)]

extern crate rustless;
extern crate iron;
extern crate valico;
extern crate email;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate error_chain;
extern crate r2d2;
extern crate rand;
extern crate chrono;
extern crate lettre;
extern crate log;
extern crate env_logger;
extern crate uuid;
extern crate hyper;
extern crate hyper_native_tls;
extern crate serde_json;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;
extern crate r2d2_diesel;

extern crate brdgme_cmd;
extern crate brdgme_game;
extern crate brdgme_color;
extern crate brdgme_markup;

use rustless::ErrorResponse;

mod config;
mod controller;
mod db;
mod mail;
mod game_client;

mod errors {
    error_chain!{
        links {
            Markup(::brdgme_markup::errors::Error, ::brdgme_markup::errors::ErrorKind);
        }

        foreign_links {
            EnvVar(::std::env::VarError);
            Chrono(::chrono::ParseError);
            Diesel(::diesel::result::Error);
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

fn main() {
    env_logger::init().unwrap();
    iron::Iron::new(controller::app())
        .http("0.0.0.0:8000")
        .unwrap();
}
