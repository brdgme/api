use rustless::framework::client::Client;
use rustless::framework::Namespace;
use rustless::json::{JsonValue, ToJson};
use rustless::backend::HandleResult;
use rustless::{Nesting, ErrorResponse};
use rustless::server::header;
use valico::json_dsl;
use lettre::email::EmailBuilder;
use uuid::Uuid;

use brdgme_db::query;

use std::{error, fmt};

use errors::*;
use CONN;
use mail;

#[derive(Debug)]
pub struct UnauthorizedError;

impl fmt::Display for UnauthorizedError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "UnauthorizedError")
    }
}

impl error::Error for UnauthorizedError {
    fn description(&self) -> &str {
        "UnauthorizedError"
    }
}

impl Into<ErrorResponse> for UnauthorizedError {
    fn into(self) -> ErrorResponse {
        ErrorResponse {
            error: Box::new(self),
            response: None,
        }
    }
}

pub fn namespace(ns: &mut Namespace) {
    ns.post("", |endpoint| {
        endpoint.desc("Request login");
        endpoint.params(|params| { params.req_typed("email", json_dsl::string()); });
        endpoint.handle(create)
    });
    ns.post("confirm", |endpoint| {
        endpoint.desc("Confirm login");
        endpoint.params(|params| {
                            params.req_typed("email", json_dsl::string());
                            params.req_typed("confirmation", json_dsl::string());
                        });
        endpoint.handle(confirm)
    });
}

pub fn create<'a>(client: Client<'a>, params: &JsonValue) -> HandleResult<Client<'a>> {
    let params = params.as_object()
        .ok_or::<Error>("params not an object".into())?;
    let create_email = params.get("email")
        .ok_or::<Error>("email param missing".into())?
        .as_str()
        .ok_or::<Error>("email not a string".into())?;
    let ref conn = *CONN.w.get().chain_err(|| "unable to get connection")?;
    let confirmation =
        query::user_login_request(create_email, conn).chain_err(|| "unable to request user login")?;

    mail::send(EmailBuilder::new()
        .to(create_email)
        .from("play@brdg.me")
        .subject("brdg.me login confirmation")
        .html(&mail::html_layout(&format!("Your brdg.me confirmation is <b>{}</b>

This confirmation will expire in 30 minutes if not used.", confirmation)))
        .build()
        .chain_err(|| "unable to create login confirmation email")?
    )
    .chain_err(|| "unable to send login confirmation email")?;

    client.empty()
}

pub fn confirm<'a>(client: Client<'a>, params: &JsonValue) -> HandleResult<Client<'a>> {
    let email = params.pointer("/email")
        .and_then(|v| v.as_str())
        .ok_or::<Error>("unable to get email parameter".into())?;
    let confirmation = params.pointer("/confirmation")
        .and_then(|v| v.as_str())
        .ok_or::<Error>("unable to get confirmation parameter".into())?;
    let ref conn = *CONN.w.get().chain_err(|| "unable to get connection")?;

    match query::user_login_confirm(email, confirmation, conn).chain_err(|| "unable to confirm login")? {
        Some(token) => client.json(&token.id.to_string().to_json()),
        None => client.error::<Error>("unable to confirm login".into()),
    }
}

pub fn authenticate<'a>(client: &Client<'a>) -> HandleResult<query::UserByEmail> {
    let conn = CONN.r.get().chain_err(|| "unable to get connection")?;
    let auth_header = &client.request
                           .headers()
                           .get::<header::Authorization<header::Basic>>()
                           .ok_or::<Error>("unable to get Authorization header".into())?;
    let email = auth_header.username.to_owned();
    let password = auth_header.password
        .to_owned()
        .ok_or::<Error>("password not specified".into())?;
    Ok(query::authenticate(&email, &Uuid::parse_str(&password)
        .map_err::<ErrorResponse, _>(|_| UnauthorizedError{}.into())?, &conn)
        .chain_err(|| "unable to authenticate")?.ok_or::<ErrorResponse>(UnauthorizedError{}.into())?)

}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
