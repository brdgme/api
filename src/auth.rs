use rustless::framework::client::Client;
use rustless::framework::Namespace;
use rustless::json::{JsonValue, ToJson};
use rustless::backend::HandleResult;
use rustless::Nesting;
use valico::json_dsl;
use diesel::prelude::*;

use brdgme_db::models::*;

use CONN;
use to_error_response;

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
    use brdgme_db::schema::user_emails::dsl::*;

    let create_email = params.pointer("/email").and_then(|v| v.as_str()).unwrap_or_else(|| "");
    let ref conn_r = *CONN.r
                          .get()
                          .map_err(to_error_response)?;
    let ref conn_w = *CONN.w
                          .get()
                          .map_err(to_error_response)?;

    // Check if there already exists a user with that email.
    let results = user_emails.filter(email.eq(create_email))
        .limit(1)
        .load::<UserEmail>(conn_r)
        .map_err(to_error_response)?;
    /*let user = if results.is_empty() {
        diesel::insert(&NewUser {
            ..Default::default()
        }).get_result(conn_w).map_err(to_error_response)?
    } else {
    }*/
    client.json(&create_email.to_json())
}

pub fn confirm<'a>(client: Client<'a>, params: &JsonValue) -> HandleResult<Client<'a>> {
    client.json(&params.to_json())
}
