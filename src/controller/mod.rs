use rustless::{Application, Api, Nesting, Versioning, Response};
use rustless::server::status::StatusCode;
use rustless::batteries::swagger;

pub mod auth;
pub mod game;
pub mod mail;

use errors::*;

pub fn app() -> Application {
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
    app
}
