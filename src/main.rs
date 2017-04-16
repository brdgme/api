#![recursion_limit = "1024"]
#![allow(dead_code)]
#![allow(unused_variables)]
#![feature(plugin)]
#![plugin(rocket_codegen)]
#![feature(custom_derive)]

extern crate rocket;
extern crate rocket_contrib;
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
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
#[macro_use]
extern crate diesel;
#[macro_use]
extern crate diesel_codegen;
extern crate r2d2_diesel;
extern crate unicase;

extern crate brdgme_cmd;
extern crate brdgme_game;
extern crate brdgme_color;
extern crate brdgme_markup;

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
            Io(::std::io::Error);
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
}

fn main() {
    rocket::ignite()
        .mount("/game",
               routes![
            controller::game::create,
            controller::game::show,
            controller::game::command,
            controller::game::version_public,
        ])
        .mount("/auth",
               routes![
            controller::auth::create,
            controller::auth::confirm,
        ])
        .mount("/mail", routes![controller::mail::index])
        .mount("/", routes![controller::options])
        .launch();
}
