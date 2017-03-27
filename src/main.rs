#![recursion_limit = "1024"]

extern crate rustless;
extern crate hyper;
extern crate iron;
extern crate valico;
extern crate email;
extern crate diesel;
extern crate r2d2;
extern crate r2d2_diesel;
#[macro_use]
extern crate lazy_static;
#[macro_use]
extern crate error_chain;

extern crate brdgme_db;

use rustless::{Application, Api, Nesting, Versioning};
use rustless::batteries::swagger;
use rustless::errors::{Error, ErrorResponse};

use diesel::pg::PgConnection;
use r2d2_diesel::ConnectionManager;

use std::default::Default;

mod auth;
mod game;
mod mail;

mod errors {
    error_chain!{}
}

pub struct Conn {
    r: r2d2::Pool<ConnectionManager<PgConnection>>,
    w: r2d2::Pool<ConnectionManager<PgConnection>>,
}

lazy_static! {
    pub static ref CONN: Conn = Conn {
        r: connect(env!("DATABASE_URL_R")),
        w: connect(env!("DATABASE_URL_W")),
    };
}

fn connect(db_url: &str) -> r2d2::Pool<ConnectionManager<PgConnection>> {
    let config = r2d2::Config::default();
    let manager = ConnectionManager::<PgConnection>::new(db_url);
    r2d2::Pool::new(config, manager).expect("Failed to create connection pool.")
}

pub fn to_error_response<T: Error + Send>(e: T) -> ErrorResponse {
    ErrorResponse {
        error: Box::new(e),
        response: None,
    }
}

fn main() {
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
