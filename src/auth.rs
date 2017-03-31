use rustless::framework::client::Client;
use rustless::framework::Namespace;
use rustless::json::{JsonValue, ToJson};
use rustless::backend::HandleResult;
use rustless::Nesting;
use valico::json_dsl;
use rand::{self, Rng};
use chrono::{Duration, UTC};

use brdgme_db::query;

use CONN;
use to_error_response;

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

    client.empty()
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
