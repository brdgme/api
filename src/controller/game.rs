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
use diesel::Connection;

use brdgme_cmd::cli;
use brdgme_game::Status;

use std::collections::BTreeMap;

use controller::auth::authenticate;
use db::{query, models};
use errors::*;
use db::CONN;

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
    ns.get("version/public", |endpoint| {
        endpoint.desc("Public game versions");
        endpoint.handle(version_public)
    });
    ns.get("my_active", |endpoint| {
        endpoint.desc("My active games");
        endpoint.handle(my_active)
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
        Some(v) => {
            v.as_array()
                .unwrap()
                .iter()
                // TODO handle `parse_str` failures as error instead of panicking.
                .map(|e| Uuid::parse_str(e.as_str().unwrap()).unwrap())
                .collect()
        }
        None => vec![],
    };
    let opponent_emails: Vec<String> = match params.find("opponent_emails") {
        Some(v) => {
            v.as_array()
                .unwrap()
                .iter()
                .map(|e| e.as_str().unwrap().to_owned())
                .collect()
        }
        None => vec![],
    };

    let conn = &*CONN.w.get().chain_err(|| "unable to get connection")?;
    let (_, user) = authenticate(&client, conn)?;
    let player_count: usize = 1 + opponent_ids.len() + opponent_emails.len();

    let created_game: query::CreatedGame = conn.transaction::<_, Error, _>(|| {
            let game_version =
                query::find_game_version(&game_version_id, conn)
                    .chain_err(|| "error finding game version")?
                    .ok_or_else::<Error, _>(|| "could not find game version".into())?;

            let resp = game_request(&game_version.uri,
                                    &cli::Request::New { players: player_count })?;
            let (game_info, logs) = match resp {
                cli::Response::New { game, logs } => (game, logs),
                _ => bail!("expected cli::Response::New"),
            };
            let status = game_status_values(&game_info.status);
            let created_game =
                query::create_game_with_users(&query::CreateGameOpts {
                                                   new_game: &models::NewGame {
                                                                  game_version_id: game_version_id,
                                                                  is_finished: status.is_finished,
                                                                  game_state: &game_info.state,
                                                              },
                                                   whose_turn: &status.whose_turn,
                                                   eliminated: &status.eliminated,
                                                   winners: &status.winners,
                                                   creator_id: &user.id,
                                                   opponent_ids: &opponent_ids,
                                                   opponent_emails: &opponent_emails,
                                               },
                                              conn)
                        .chain_err(|| "unable to create game")?;
            let created_logs = query::create_game_logs_from_cli(&created_game.game.id, logs, conn)
                .chain_err(|| "unable to create game logs")?;
            Ok(created_game)
        })
        .chain_err(|| "error committing transaction")?;

    client.json(&created_game.game.id.to_string().to_json())
}

struct StatusValues {
    is_finished: bool,
    whose_turn: Vec<usize>,
    eliminated: Vec<usize>,
    winners: Vec<usize>,
}
fn game_status_values(status: &Status) -> StatusValues {

    let (is_finished, whose_turn, eliminated, winners) = match *status {
        Status::Active {
            ref whose_turn,
            ref eliminated,
        } => (false, whose_turn.clone(), eliminated.clone(), vec![]),
        Status::Finished { ref winners } => (true, vec![], vec![], winners.clone()),
    };
    StatusValues {
        is_finished: is_finished,
        whose_turn: whose_turn,
        eliminated: eliminated,
        winners: winners,
    }
}

fn game_request(uri: &str, request: &cli::Request) -> Result<cli::Response> {
    let ssl = NativeTlsClient::new()
        .chain_err(|| "unable to get native TLS client")?;
    let connector = HttpsConnector::new(ssl);
    let https = HttpClient::with_connector(connector);
    let res = https
        .post(uri)
        .body(&serde_json::to_string(request)
                   .chain_err(|| "error converting request to JSON")?)
        .send()
        .chain_err(|| "error getting new game state")?;
    if res.status != hyper::Ok {
        bail!("game request failed");
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
    let id = Uuid::parse_str(params.find("id").unwrap().as_str().unwrap())
        .map_err::<Error, _>(|_| {
                                 ErrorKind::UserError("game_version_id is not a UUID".to_string())
                                     .into()
                             })?;
    let cmd_text = params.find("command").unwrap().as_str().unwrap();

    let conn = &*CONN.w.get().chain_err(|| "unable to get connection")?;
    let (_, user) = authenticate(&client, conn)?;

    conn.transaction::<_, Error, _>(|| {

            let (game, game_version) = query::find_game_with_version(&id, conn)
                .chain_err(|| "error finding game")?
                .ok_or_else::<Error, _>(|| {
                                            ErrorKind::UserError("game does not exist".to_string())
                                                .into()
                                        })?;
            let players = query::find_game_players_with_user_by_game(&id, conn)
                .chain_err(|| "error finding game players")?;
            let position = players
                .iter()
                .find(|&&(ref p, _)| p.user_id == user.id)
                .ok_or_else::<Error, _>(|| {
                                            ErrorKind::UserError("you are not a player in this game"
                                                                     .to_string())
                                                    .into()
                                        })?
                .0
                .position;

            let names = players
                .iter()
                .map(|&(_, ref user)| user.name.clone())
                .collect::<Vec<String>>();

            let (game_response, logs, remaining_command) =
                match game_request(&game_version.uri,
                                   &cli::Request::Play {
                                        player: position as usize,
                                        game: game.game_state,
                                        command: cmd_text.to_string(),
                                        names: names,
                                    })? {
                    cli::Response::Play {
                        game,
                        logs,
                        remaining_command,
                    } => (game, logs, remaining_command),
                    _ => bail!("invalid response type"),
                };
            let status = game_status_values(&game_response.status);

            let updated = query::update_game_and_players(&id,
                                                         &models::NewGame {
                                                              game_version_id: game.game_version_id,
                                                              is_finished: status.is_finished,
                                                              game_state: &game_response.state,
                                                          },
                                                         &status.whose_turn,
                                                         &status.eliminated,
                                                         &status.winners,
                                                         conn)
                    .chain_err(|| "error updating game")?;

            let created_logs = query::create_game_logs_from_cli(&id, logs, conn)
                .chain_err(|| "unable to create game logs")?;
            Ok(updated)

        })
        .chain_err(|| "error committing transaction")?;

    client.json(&params.to_json())
}

pub fn version_public<'a>(client: Client<'a>, params: &JsonValue) -> HandleResult<Client<'a>> {
    let conn = &*CONN.r.get().chain_err(|| "unable to get connection")?;

    client.json(&JsonValue::Array(query::public_game_versions(conn)
                                      .chain_err(|| "error getting public game versions")?
                                      .iter()
                                      .map(|&(ref game_version, ref game_type)| {
        JsonValue::Object({
                              let mut props = BTreeMap::new();
                              props.insert("game_version_id".to_string(),
                                           JsonValue::String(game_version.id.to_string()));
                              props.insert("name".to_string(),
                                           JsonValue::String(game_type.name.to_owned()));
                              props
                          })
    })
                                      .collect::<Vec<JsonValue>>())
                         .to_json())
}

pub fn my_active<'a>(client: Client<'a>, params: &JsonValue) -> HandleResult<Client<'a>> {
    let conn = &*CONN.r.get().chain_err(|| "unable to get connection")?;

    let (_, user) = authenticate(&client, conn)?;

    client.json(&JsonValue::Array(query::find_active_games_for_user(&user.id, conn)
                                      .chain_err(|| "error getting active_games")?
                                      .iter()
                                      .map(|game_extended| {
        JsonValue::Object({
                              let mut props = BTreeMap::new();
                              props.insert("game_id".to_string(),
                                           JsonValue::String(game_extended.game.id.to_string()));
                              props.insert("game_version_id".to_string(),
                                           JsonValue::String(game_extended
                                                                 .game_version
                                                                 .id
                                                                 .to_string()));
                              props.insert("name".to_string(),
                                           JsonValue::String(game_extended
                                                                 .game_type
                                                                 .name
                                                                 .to_owned()));
                              props.insert("is_finished".to_string(),
                                           JsonValue::Bool(game_extended.game.is_finished));
                              props.insert("game_players".to_string(),
                            JsonValue::Array(game_extended.game_players.iter()
                            .map(|&(ref game_player, ref user)| JsonValue::Object({
                                let mut props = BTreeMap::new();
                                props.insert("name".to_string(),
                                    JsonValue::String(user.name.to_owned()));
                                props.insert("color".to_string(),
                                    JsonValue::String(game_player.color.to_owned()));
                                props.insert("is_winner".to_string(),
                                    JsonValue::Bool(game_player.is_winner));
                                props.insert("is_turn".to_string(),
                                    JsonValue::Bool(game_player.is_turn));
                                props.insert("is_eliminated".to_string(),
                                    JsonValue::Bool(game_player.is_eliminated));
                                props
                            })

                            ).collect::<Vec<JsonValue>>()));
                              props
                          })
    })
                                      .collect::<Vec<JsonValue>>())
                         .to_json())
}
