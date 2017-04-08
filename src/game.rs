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

use brdgme_cmd::cli;
use brdgme_game::Status;

use auth::authenticate;
use db::{query, models};
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
    let game_version_id = Uuid::parse_str(params
                                              .find("game_version_id")
                                              .unwrap()
                                              .as_str()
                                              .unwrap())
            .map_err::<Error, _>(|_| {
                                     ErrorKind::UserError("game_version_id is not a UUID"
                                                              .to_string())
                                             .into()
                                 })?;
    let opponent_ids: Vec<Uuid> = match params.find("opponent_ids") {
        Some(ref v) => {
            v.as_array()
                .unwrap()
                .iter()
                // TODO handle `parse_str` failures as error instead of panicking.
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

    let ref conn = *CONN.w.get().chain_err(|| "unable to get connection")?;
    let ube = authenticate(&client, conn)?;
    let player_count: usize = 1 + opponent_ids.len() + opponent_emails.len();

    let trans = conn.transaction()
        .chain_err(|| "error starting transaction")?;
    let game_version = query::find_game_version(&game_version_id, &trans)
        .chain_err(|| "error finding game version")?
        .ok_or_else::<Error, _>(|| "could not find game version".into())?;

    let resp = game_request(&game_version.uri,
                            &cli::Request::New { players: player_count })?;
    let (game_info, logs) = match resp {
        cli::Response::New { game, logs } => (game, logs),
        _ => bail!(err_resp("expected cli::Response::New")),
    };
    let (is_finished, whose_turn, eliminated, winners) = match game_info.status {
        Status::Active {
            whose_turn,
            eliminated,
        } => (false, whose_turn, eliminated, vec![]),
        Status::Finished { winners } => (true, vec![], vec![], winners),
    };
    let created_game = query::create_game_with_users(&models::NewGame {
                                                          game_version_id: &game_version_id,
                                                          is_finished: is_finished,
                                                          game_state: &game_info.state,
                                                      },
                                                     &whose_turn,
                                                     &eliminated,
                                                     &winners,
                                                     &ube.user.id,
                                                     &opponent_ids,
                                                     &opponent_emails,
                                                     &trans)
            .chain_err(|| "unable to create game")?;
    let created_logs = query::create_game_logs_from_cli(&created_game.game.id, logs, &trans)
        .chain_err(|| "unable to create game logs")?;

    trans
        .commit()
        .chain_err(|| "error committing transaction")?;

    client.json(&created_game.game.game_state.to_json())
}

fn game_request(uri: &str, request: &cli::Request) -> Result<cli::Response> {
    // TODO handle error
    let ssl = NativeTlsClient::new().unwrap();
    let connector = HttpsConnector::new(ssl);
    let https = HttpClient::with_connector(connector);
    let res = https
        .post(uri)
        // TODO handle error
        .body(&serde_json::to_string(request).unwrap())
        .send()
        .chain_err(|| "error getting new game state")?;
    if res.status != hyper::Ok {
        bail!("request failed");
    }
    match serde_json::from_reader::<_, cli::Response>(res)
              .chain_err(|| "error parsing JSON response")? {
        cli::Response::UserError { message } => Err(ErrorKind::UserError(message).into()),
        cli::Response::SystemError { message } => Err(message.into()),
        default => Ok(default),
    }
}

pub fn show<'a>(client: Client<'a>, params: &JsonValue) -> HandleResult<Client<'a>> {
    client.json(&params.to_json())
}

pub fn command<'a>(client: Client<'a>, params: &JsonValue) -> HandleResult<Client<'a>> {
    client.json(&params.to_json())
}
