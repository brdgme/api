use rustless::framework::client::Client;
use rustless::framework::Namespace;
use rustless::json::{JsonValue, ToJson};
use rustless::backend::HandleResult;
use rustless::Nesting;
use valico::json_dsl;
use chrono::{Duration, UTC};
use lettre::email::EmailBuilder;

use brdgme_db::query;

use errors::*;
use CONN;
use to_error_response;
use mail;

lazy_static! {
    static ref TOKEN_EXPIRY: Duration = Duration::minutes(30);
}

pub fn namespace(ns: &mut Namespace) {
    ns.post("", |endpoint| {
        endpoint.desc("Request auth token");
        endpoint.params(|params| { params.req_typed("email", json_dsl::string()); });
        endpoint.handle(create)
    });
    ns.post(":token/confirm", |endpoint| {
        endpoint.desc("Show game");
        endpoint.params(|params| {
                            params.req_typed("token", json_dsl::string());
                            params.req_typed("code", json_dsl::string());
                        });
        endpoint.handle(confirm)
    });
}

pub fn create<'a>(client: Client<'a>, params: &JsonValue) -> HandleResult<Client<'a>> {
    let create_email = params
        .pointer("/email")
        .and_then(|v| v.as_str())
        .unwrap();
    let ref conn = *CONN.w.get().map_err(to_error_response)?;

    let user = query::find_or_create_user_by_email(create_email, conn)
        .map_err(to_error_response)?
        .user;

    let confirmation = match (user.login_confirmation, user.login_confirmation_at) {
        (Some(ref uc), Some(at)) if at + *TOKEN_EXPIRY > UTC::now().naive_utc() => uc.to_owned(),
        _ => query::generate_user_login_confirmation(&user.id, conn).map_err(to_error_response)?,
    };

    mail::send(EmailBuilder::new()
        .to(create_email)
        .from("play@brdg.me")
        .subject("brdg.me login confirmation")
        .html(&mail::html_layout(&format!("Your brdg.me confirmation is <b>{}</b>

This confirmation will expire in 30 minutes if not used.", confirmation)))
        .build()
        .chain_err(|| "unable to create login confirmation email")
        .map_err(to_error_response)?
    )
    .chain_err(|| "unable to send login confirmation email")
    .map_err(to_error_response)?;

    client.empty()
}

pub fn confirm<'a>(client: Client<'a>, params: &JsonValue) -> HandleResult<Client<'a>> {
    client.json(&params.to_json())
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
