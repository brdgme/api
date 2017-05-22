use rocket::request::Request;
use rocket::response::{self, Responder, Response};
use rocket::http::{Status, ContentType};
use rocket::http::hyper::header::{AccessControlAllowOrigin, AccessControlAllowMethods,
                                  AccessControlAllowHeaders, AccessControlAllowCredentials};
use hyper::method::Method;
use unicase::UniCase;

use std::io::Cursor;

error_chain!{
    links {
        Markup(::brdgme_markup::errors::Error, ::brdgme_markup::errors::ErrorKind);
    }

    foreign_links {
        Io(::std::io::Error);
        EnvVar(::std::env::VarError);
        Chrono(::chrono::ParseError);
        Diesel(::diesel::result::Error);
        Redis(::redis::RedisError);
        Json(::serde_json::Error);
    }

    errors {
        UserError(message: String) {
            description("user error")
            display("{}", message)
        }
    }
}

impl<'r> Responder<'r> for Error {
    fn respond_to(self, _: &Request) -> response::Result<'r> {
        match self {
            Error(ErrorKind::UserError(ref message), _) => {
                Ok(Response::build()
                       .status(Status::BadRequest)
                       .header(ContentType::Plain)
                       .header(AccessControlAllowOrigin::Any)
                       .header(AccessControlAllowMethods(vec![Method::Get,
                                                              Method::Post,
                                                              Method::Put,
                                                              Method::Delete,
                                                              Method::Options]))
                       .header(AccessControlAllowHeaders(vec![UniCase("Authorization"
                                                                          .to_string()),
                                                              UniCase("Content-Type"
                                                                          .to_string())]))
                       .header(AccessControlAllowCredentials)
                       .sized_body(Cursor::new(message.to_owned()))
                       .finalize())
            }
            _ => Err(Status::InternalServerError),
        }
    }
}
