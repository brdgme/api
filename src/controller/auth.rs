use rocket::request::{self, Request, FromRequest};
use rocket_contrib::JSON;
use rocket::http::Status;
use rocket::http::hyper::header::Basic;
use rocket::Outcome;
use lettre::email::EmailBuilder;
use uuid::Uuid;

use std::str::FromStr;

use errors::*;
use db::{CONN, query};
use db::models::*;
use mail;

#[derive(Deserialize)]
pub struct CreateForm {
    email: String,
}

#[post("/", data = "<data>")]
pub fn create(data: JSON<CreateForm>) -> Result<()> {
    let create_email = data.into_inner().email;
    let conn = &*CONN.w.get().chain_err(|| "unable to get connection")?;
    let confirmation = query::user_login_request(&create_email, conn)
        .chain_err(|| "unable to request user login")?;

    mail::send(EmailBuilder::new()
                   .to(create_email.as_ref())
                   .from("play@brdg.me")
                   .subject("brdg.me login confirmation")
                   .html(&mail::html_layout(&format!("Your brdg.me confirmation is <b>{}</b>

This confirmation will expire in 30 minutes if not used.",
                                                     confirmation)))
                   .build()
                   .chain_err(|| "unable to create login confirmation email")?)
            .chain_err(|| "unable to send login confirmation email")?;

    Ok(())
}

#[derive(Deserialize)]
pub struct ConfirmForm {
    email: String,
    confirmation: String,
}

#[post("/confirm", data = "<data>")]
pub fn confirm(data: JSON<ConfirmForm>) -> Result<JSON<String>> {
    let data = data.into_inner();
    let conn = &*CONN.w.get().chain_err(|| "unable to get connection")?;

    match query::user_login_confirm(&data.email, &data.confirmation, conn)
              .chain_err(|| "unable to confirm login")? {
        Some(token) => Ok(JSON(token.id.to_string())),
        None => Err("unable to confirm login".into()),
    }
}

impl<'a, 'r> FromRequest<'a, 'r> for User {
    type Error = Error;

    fn from_request(request: &'a Request<'r>) -> request::Outcome<Self, Error> {
        let auth_header = match request.headers().get_one("Authorization") {
            Some(a) => a,
            None => {
                return Outcome::Failure((Status::Unauthorized,
                                         "missing Authorization header".into()))
            }
        };
        if !auth_header.starts_with("Basic ") {
            return Outcome::Failure((Status::Unauthorized,
                                     "expected Basic Authorization header".into()));
        }
        let auth = match Basic::from_str(&auth_header[6..]) {
            Ok(a) => a,
            Err(e) => {
                return Outcome::Failure((Status::Unauthorized,
                                         "invalid Authorization header".into()))
            }
        };
        let pass_uuid = match auth.password {
            Some(p) => {
                match Uuid::parse_str(&p) {
                    Ok(uuid) => uuid,
                    Err(_) => {
                        return Outcome::Failure((Status::Unauthorized,
                                                 "Authorization password not in valid format"
                                                     .into()))
                    }
                }
            }
            None => {
                return Outcome::Failure((Status::Unauthorized,
                                         "Authorization password not present".into()))
            }
        };
        let conn = &*match CONN.r.get() {
                         Ok(c) => c,
                         Err(_) => {
                             return Outcome::Failure((Status::InternalServerError,
                                                      "error getting connection".into()))
                         }
                     };

        match query::authenticate(&auth.username, &pass_uuid, conn) {
            Ok(Some((_, user))) => Outcome::Success(user),
            _ => Outcome::Failure((Status::Unauthorized, "invalid credentials".into())),
        }
    }
}
