extern crate rustless;
extern crate hyper;
extern crate iron;
extern crate valico;
extern crate email;

use rustless::{Application, Api, Nesting, Versioning};
use rustless::batteries::swagger;

use std::default::Default;

mod game;
mod mail;

fn main() {
    let api = Api::build(|api| {
        api.prefix("api");
        api.mount(swagger::create_api("docs"));
        api.mount(Api::build(|v1| {
                                 v1.version("v1", Versioning::Path);
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
