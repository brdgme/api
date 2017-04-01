use rustless::framework::client::Client;
use rustless::framework::Namespace;
use rustless::json::{JsonValue, ToJson};
use rustless::backend::HandleResult;
use rustless::Nesting;
use valico::json_dsl;
use lettre::email::EmailBuilder;

use brdgme_db::query;

use errors::*;
use CONN;
use to_error_response;
use mail;

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
                            params.req_typed("code", json_dsl::string());
                        });
        endpoint.handle(confirm)
    });
}

pub fn create<'a>(client: Client<'a>, params: &JsonValue) -> HandleResult<Client<'a>> {
    let create_email = params.pointer("/email")
        .and_then(|v| v.as_str())
        .ok_or::<Error>("unable to get email parameter".into())
        .map_err(to_error_response)?;
    let ref conn = *CONN.w.get().map_err(to_error_response)?;
    let confirmation = query::user_login_request(create_email, conn).map_err(to_error_response)?;

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
    let email = params.pointer("/email")
        .and_then(|v| v.as_str())
        .ok_or::<Error>("unable to get email parameter".into())
        .map_err(to_error_response)?;
    let confirm = params.pointer("/confirm")
        .and_then(|v| v.as_str())
        .ok_or::<Error>("unable to get confirm parameter".into())
        .map_err(to_error_response)?;
    let ref conn = *CONN.w.get().map_err(to_error_response)?;

    match query::user_login_confirm(email, confirm, conn).map_err(to_error_response)? {
        Some(token) => client.json(&token.id.to_string().to_json()),
        None => client.error::<Error>("blah".into()),
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
