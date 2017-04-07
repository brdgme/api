use rustless::framework::client::Client;
use rustless::framework::Namespace;
use rustless::json::{JsonValue, ToJson};
use rustless::backend::HandleResult;
use rustless::Nesting;
use valico::json_dsl;
use uuid::Uuid;
use hyper::{self, Client as HttpClient};
use hyper::net::HttpsConnector;
use hyper_native_tls::NativeTlsClient;
use serde_json;

use brdgme_db::query;
use brdgme_cmd::cli;

use auth::authenticate;
use errors::*;
use CONN;

pub fn namespace(ns: &mut Namespace) {
    ns.get("", |endpoint| {
        endpoint.desc("List games");
        endpoint.handle(index)
    });
    ns.post("", |endpoint| {
        endpoint.desc("Create game");
        endpoint.params(|params| {
                            params.req_typed("game_version_id", json_dsl::string());
                            params.opt_typed("opponent_ids",
                                             json_dsl::array_of(json_dsl::string()));
                            params.opt_typed("opponent_emails",
                                             json_dsl::array_of(json_dsl::string()));
                        });
        endpoint.handle(create)
    });
    ns.get(":id", |endpoint| {
        endpoint.desc("Show game");
        endpoint.params(|params| { params.req_typed("id", json_dsl::string()); });
        endpoint.handle(show)
    });
    ns.post(":id/command", |endpoint| {
        endpoint.desc("Send game command");
        endpoint.params(|params| {
                            params.req_typed("id", json_dsl::string());
                            params.req_typed("command", json_dsl::string());
                        });
        endpoint.handle(command)
    });
}

pub fn index<'a>(client: Client<'a>, params: &JsonValue) -> HandleResult<Client<'a>> {
    client.json(&params.to_json())
}

pub fn create<'a>(client: Client<'a>, params: &JsonValue) -> HandleResult<Client<'a>> {
    // Parse input
    let ube = authenticate(&client)?;
    let game_version_id = Uuid::parse_str(params
                                              .find("game_version_id")
                                              .unwrap()
                                              .as_str()
                                              .unwrap())
            .map_err::<Error, _>(|_| "game_version_id is not a UUID".into())?;
    let opponent_ids: Vec<Uuid> = match params.find("opponent_ids") {
        Some(ref v) => {
            v.as_array()
                .unwrap()
                .iter()
                .map(|ref e| Uuid::parse_str(e.as_str().unwrap()).unwrap())
                .collect()
        }
        None => vec![],
    };
    let opponent_emails: Vec<String> = match params.find("opponent_emails") {
        Some(ref v) => {
            v.as_array()
                .unwrap()
                .iter()
                .map(|ref e| e.as_str().unwrap().to_owned())
                .collect()
        }
        None => vec![],
    };

    let player_count: usize = 1 + opponent_ids.len() + opponent_emails.len();

    let ref conn = *CONN.w.get().chain_err(|| "unable to get connection")?;
    let trans = conn.transaction()
        .chain_err(|| "error starting transaction")?;
    let game_version = query::find_game_version(&game_version_id, &trans)
        .chain_err(|| "error finding game version")?
        .ok_or_else::<Error, _>(|| "could not find game version".into())?;

    let ssl = NativeTlsClient::new().unwrap();
    let connector = HttpsConnector::new(ssl);
    let https = HttpClient::with_connector(connector);
    let res = https
        .post(&game_version.uri)
        .body(&serde_json::to_string(&cli::Request::New { players: player_count }).unwrap())
        .send()
        .chain_err(|| "error getting new game state")?;
    if res.status != hyper::Ok {}
    let resp: cli::Response = serde_json::from_reader(res)
        .chain_err(|| "error parsing JSON response")?;
    println!("{:?}", resp);

    trans
        .commit()
        .chain_err(|| "error committing transaction")?;

    client.json(&opponent_emails.to_json())
}

pub fn show<'a>(client: Client<'a>, params: &JsonValue) -> HandleResult<Client<'a>> {
    client.json(&params.to_json())
}

pub fn command<'a>(client: Client<'a>, params: &JsonValue) -> HandleResult<Client<'a>> {
    client.json(&params.to_json())
}
