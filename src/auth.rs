use rustless::framework::client::Client;
use rustless::framework::Namespace;
use rustless::json::{JsonValue, ToJson};
use rustless::backend::HandleResult;
use rustless::Nesting;
use valico::json_dsl;
use rand::{self, Rng};
use chrono::{NaiveDateTime, Duration, UTC};

use CONN;
use to_error_response;
use errors::*;

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

fn generate_login_confirmation() -> String {
    let mut rng = rand::thread_rng();
    format!("{}{:05}",
            (rng.gen::<usize>() % 9) + 1,
            rng.gen::<usize>() % 100000)
}

pub fn create<'a>(client: Client<'a>, params: &JsonValue) -> HandleResult<Client<'a>> {
    let create_email = params.pointer("/email").and_then(|v| v.as_str());
    let ref conn = *CONN.w.get().map_err(to_error_response)?;

    let mut opt_user_id: Option<i32> = None;
    let mut opt_login_confirmation: Option<String> = None;
    for row in &conn.query("
        SELECT
            user_id,
            login_confirmation,
            login_confirmation_at
        FROM user_emails AS ue
        INNER JOIN users AS u
        ON ue.user_id = u.id
        WHERE ue.email = $1
        LIMIT 1",
                           &[&create_email])
                    .map_err(to_error_response)? {
        opt_user_id = Some(row.get("user_id"));
        if let Some(at) = row.get::<_, Option<NaiveDateTime>>("login_confirmation_at") {
            if at + *TOKEN_EXPIRY > UTC::now().naive_utc() {
                // There's a token that's still valid, use it.
                opt_login_confirmation = row.get("login_confirmation");
            }
        }
    }

    let user_id = opt_user_id.or_else(|| {
                     // We couldn't find a user, so we need to create one.
                     None
                 })
        .ok_or::<Error>("unable to create user with that email".into())
        .map_err(to_error_response)?;

    client.json(&create_email.to_json())
}

pub fn confirm<'a>(client: Client<'a>, params: &JsonValue) -> HandleResult<Client<'a>> {
    client.json(&params.to_json())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_login_confirmation_works() {
        for _ in 1..100000 {
            let n: usize = generate_login_confirmation().parse().unwrap();
            assert!(n > 99999, "n <= 99999");
            assert!(n < 1000000, "n >= 1000000");
        }
    }
}
