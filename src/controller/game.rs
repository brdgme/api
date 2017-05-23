use rocket_contrib::JSON;
use uuid::Uuid;
use diesel::Connection;
use diesel::pg::PgConnection;

use brdgme_cmd::cli;
use brdgme_game::{Status, Stat};
use brdgme_game::command::Spec as CommandSpec;
use brdgme_markup as markup;

use std::collections::HashMap;

use db::{query, models};
use errors::*;
use db::CONN;
use game_client;
use render;
use controller::{UuidParam, CORS};
use websocket;

#[derive(Deserialize)]
pub struct CreateRequest {
    game_version_id: Uuid,
    opponent_ids: Option<Vec<Uuid>>,
    opponent_emails: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct CreateResponse {
    id: Uuid,
}

#[post("/", data = "<data>")]
pub fn create(data: JSON<CreateRequest>, user: models::User) -> Result<CORS<JSON<CreateResponse>>> {
    let data = data.into_inner();
    let conn = &*CONN.w.get().chain_err(|| "unable to get connection")?;

    let (created_game, created_logs, public_render, player_renders, user_ids) =
        conn.transaction::<_, Error, _>(move || {
                let opponent_ids = data.opponent_ids.unwrap_or_else(|| vec![]);
                let opponent_emails = data.opponent_emails.unwrap_or_else(|| vec![]);
                let player_count: usize = 1 + opponent_ids.len() + opponent_emails.len();
                let game_version =
                    query::find_game_version(&data.game_version_id, conn)
                        .chain_err(|| "error finding game version")?
                        .ok_or_else::<Error, _>(|| "could not find game version".into())?;

                let resp = game_client::request(&game_version.uri,
                                                &cli::Request::New { players: player_count })?;
                let (game_info, logs, public_render, player_renders) = match resp {
                    cli::Response::New {
                        game,
                        logs,
                        public_render,
                        player_renders,
                    } => (game, logs, public_render, player_renders),
                    _ => bail!("expected cli::Response::New"),
                };
                let status = game_status_values(&game_info.status);
                let created_game =
                    query::create_game_with_users(&query::CreateGameOpts {
                                                      new_game: &models::NewGame {
                                                          game_version_id: data.game_version_id,
                                                          is_finished: status.is_finished,
                                                          game_state: &game_info.state,
                                                      },
                                                      whose_turn: &status.whose_turn,
                                                      eliminated: &status.eliminated,
                                                      winners: &status.winners,
                                                      points: &game_info.points,
                                                      creator_id: &user.id,
                                                      opponent_ids: &opponent_ids,
                                                      opponent_emails: &opponent_emails,
                                                  },
                                                  conn)
                            .chain_err(|| "unable to create game")?;
                let created_logs =
                    query::create_game_logs_from_cli(&created_game.game.id, logs, conn)
                        .chain_err(|| "unable to create game logs")?;
                let mut user_ids = opponent_ids.clone();
                user_ids.push(user.id);
                Ok((created_game, created_logs, public_render, player_renders, user_ids))
            })
            .chain_err(|| "error committing transaction")?;
    websocket::game_update(&query::find_game_extended(&created_game.game.id, conn)
                                .chain_err(|| "unable to get extended game")?
                                .into_public(),
                           &created_logs,
                           &public_render,
                           &player_renders,
                           &query::find_valid_user_auth_tokens_for_users(&user_ids, conn)?)?;
    Ok(CORS(JSON(CreateResponse { id: created_game.game.id })))
}

struct StatusValues {
    is_finished: bool,
    whose_turn: Vec<usize>,
    eliminated: Vec<usize>,
    winners: Vec<usize>,
    stats: Vec<HashMap<String, Stat>>,
}
fn game_status_values(status: &Status) -> StatusValues {
    let (is_finished, whose_turn, eliminated, winners, stats) = match *status {
        Status::Active {
            ref whose_turn,
            ref eliminated,
        } => (false, whose_turn.clone(), eliminated.clone(), vec![], vec![]),
        Status::Finished {
            ref winners,
            ref stats,
        } => (true, vec![], vec![], winners.clone(), stats.clone()),
    };
    StatusValues {
        is_finished: is_finished,
        whose_turn: whose_turn,
        eliminated: eliminated,
        winners: winners,
        stats: stats,
    }
}

#[derive(Serialize)]
pub struct ShowResponse {
    pub game: models::PublicGame,
    pub pub_state: String,
    pub game_version: models::PublicGameVersion,
    pub game_type: models::PublicGameType,
    pub game_player: Option<models::PublicGamePlayer>,
    pub game_players: Vec<models::PublicGamePlayerUser>,
    pub html: String,
    pub game_logs: Vec<models::RenderedGameLog>,
    pub command_spec: Option<CommandSpec>,
}

#[get("/<id>")]
pub fn show(id: UuidParam, user: Option<models::User>) -> Result<CORS<JSON<ShowResponse>>> {
    let id = id.into_uuid();
    let conn = &*CONN.r.get().chain_err(|| "error getting connection")?;

    let game_extended = query::find_game_extended(&id, conn)?;
    let game_player: Option<&models::GamePlayer> = user.and_then(|u| {
        game_extended
            .game_players
            .iter()
            .find(|&&(ref gp, _)| u.id == gp.user_id)
            .map(|&(ref gp, _)| gp)
    });
    Ok(CORS(JSON(game_extended_to_show_response(game_player, &game_extended, conn)?)))
}

fn game_extended_to_show_response(game_player: Option<&models::GamePlayer>,
                                  game_extended: &query::GameExtended,
                                  conn: &PgConnection)
                                  -> Result<ShowResponse> {
    let public = game_extended.clone().into_public();
    let (pub_state, render, command_spec) =
        match game_client::request(&game_extended.game_version.uri,
                                   &cli::Request::Render {
                                       player: game_player.map(|gp| gp.position as usize),
                                       game: game_extended.game.game_state.to_owned(),
                                   })? {
            cli::Response::Render {
                render: cli::Render {
                    pub_state,
                    render,
                    command_spec,
                },
            } => (pub_state, render, command_spec),
            _ => bail!("invalid render response"),
        };

    let (nodes, _) = markup::from_string(&render)
        .chain_err(|| "error parsing render markup")?;

    let markup_players = render::game_players_to_markup_players(&game_extended.game_players);
    let game_logs = match game_player {
        Some(gp) => query::find_game_logs_for_player(&gp.id, conn),
        None => query::find_public_game_logs_for_game(&game_extended.game.id, conn),
    }?;

    Ok(ShowResponse {
           game_player: game_player.map(|gp| gp.to_owned().into_public()),
           game: public.game,
           pub_state: pub_state,
           game_version: public.game_version,
           game_type: public.game_type,
           game_players: public.game_players,
           html: markup::html(&markup::transform(&nodes, &markup_players)),
           game_logs: game_logs
               .into_iter()
               .map(|gl| gl.into_rendered(&markup_players))
               .collect::<Result<Vec<models::RenderedGameLog>>>()?,
           command_spec: command_spec,
       })
}

#[derive(Deserialize)]
pub struct CommandRequest {
    command: String,
}

#[post("/<id>/command", data = "<data>")]
pub fn command(id: UuidParam,
               user: models::User,
               data: JSON<CommandRequest>)
               -> Result<CORS<JSON<ShowResponse>>> {
    let id = id.into_uuid();
    let conn = &*CONN.w.get().chain_err(|| "unable to get connection")?;

    conn.transaction::<_, Error, _>(|| {

        let (game, game_version) = query::find_game_with_version(&id, conn)
            .chain_err(|| "error finding game")?
            .ok_or_else::<Error, _>(|| {
                                        ErrorKind::UserError("game does not exist".to_string())
                                            .into()
                                    })?;
        let players: Vec<(models::GamePlayer, models::User)> =
            query::find_game_players_with_user_by_game(&id, conn)
                .chain_err(|| "error finding game players")?;
        let player: &models::GamePlayer =
            &players
                 .iter()
                 .find(|&&(ref p, _)| p.user_id == user.id)
                 .ok_or_else::<Error, _>(|| "you are not a player in this game".into())?
                 .0;
        let position = player.position;

        let names = players
            .iter()
            .map(|&(_, ref user)| user.name.clone())
            .collect::<Vec<String>>();

        let (game_response, logs, can_undo, remaining_command, public_render, player_renders) =
            match game_client::request(&game_version.uri,
                                       &cli::Request::Play {
                                           player: position as usize,
                                           game: game.game_state.clone(),
                                           command: data.command.to_owned(),
                                           names: names,
                                       })? {
                cli::Response::Play {
                    game,
                    logs,
                    can_undo,
                    remaining_input,
                    public_render,
                    player_renders,
                } => (game, logs, can_undo, remaining_input, public_render, player_renders),
                cli::Response::UserError { message } => bail!(ErrorKind::UserError(message)),
                _ => bail!("invalid response type"),
            };
        let status = game_status_values(&game_response.status);

        let updated = query::update_game_command_success(&id,
                                                         &models::NewGame {
                                                             game_version_id: game.game_version_id,
                                                             is_finished: status.is_finished,
                                                             game_state: &game_response.state,
                                                         },
                                                         if can_undo {
                                                             Some((&player.id, &game.game_state))
                                                         } else {
                                                             None
                                                         },
                                                         &status.whose_turn,
                                                         &status.eliminated,
                                                         &status.winners,
                                                         conn)
                .chain_err(|| "error updating game")?;

        let created_logs = query::create_game_logs_from_cli(&id, logs, conn)
            .chain_err(|| "unable to create game logs")?;
        let game_extended = query::find_game_extended(&id, conn)
            .chain_err(|| "unable to get extended game")?;
        let user_ids: Vec<Uuid> = game_extended
            .game_players
            .iter()
            .map(|gp| gp.1.id.clone())
            .collect();
        websocket::game_update(&game_extended.clone().into_public(),
                               &created_logs,
                               &public_render,
                               &player_renders,
                               &query::find_valid_user_auth_tokens_for_users(&user_ids, conn)?)?;
        Ok(CORS(JSON(game_extended_to_show_response(game_extended
                                                        .game_players
                                                        .iter()
                                                        .find(|gp| gp.0.id == player.id)
                                                        .map(|gp| &gp.0),
                                                    &game_extended,
                                                    conn)?)))
    })
}

#[post("/<id>/undo")]
pub fn undo(id: UuidParam, user: models::User) -> Result<CORS<JSON<query::PublicGameExtended>>> {
    let id = id.into_uuid();
    let conn = &*CONN.w.get().chain_err(|| "unable to get connection")?;

    conn.transaction::<_, Error, _>(|| {

        let (game, game_version) = query::find_game_with_version(&id, conn)
            .chain_err(|| "error finding game")?
            .ok_or_else::<Error, _>(|| {
                                        ErrorKind::UserError("game does not exist".to_string())
                                            .into()
                                    })?;

        let game_response =
            match game_client::request(&game_version.uri,
                                       &cli::Request::Status { game: game.game_state.clone() })? {
                cli::Response::Status { game } => (game),
                _ => bail!("invalid response type"),
            };
        let status = game_status_values(&game_response.status);
        let updated = query::update_game_command_success(&id,
                                                         &models::NewGame {
                                                             game_version_id: game.game_version_id,
                                                             is_finished: status.is_finished,
                                                             game_state: &game_response.state,
                                                         },
                                                         None,
                                                         &status.whose_turn,
                                                         &status.eliminated,
                                                         &status.winners,
                                                         conn)
                .chain_err(|| "error updating game")?;
        Ok(CORS(JSON(query::find_game_extended(&id, conn)?.into_public())))
    })
}

#[derive(Serialize)]
pub struct VersionPublicResponse {
    versions: Vec<GameVersionType>,
}

#[derive(Serialize)]
struct GameVersionType {
    game_version: models::PublicGameVersion,
    game_type: models::PublicGameType,
}

#[get("/version_public")]
pub fn version_public() -> Result<CORS<JSON<VersionPublicResponse>>> {
    let conn = &*CONN.r.get().chain_err(|| "unable to get connection")?;

    Ok(CORS(JSON(VersionPublicResponse {
                     versions: query::public_game_versions(conn)
                         .chain_err(|| "error getting public game versions")?
                         .into_iter()
                         .map(|(game_version, game_type)| {
                                  GameVersionType {
                                      game_version: game_version.into_public(),
                                      game_type: game_type,
                                  }
                              })
                         .collect(),
                 })))
}

#[derive(Serialize)]
pub struct MyActiveGame {
    pub game: models::PublicGame,
    pub game_type: models::PublicGameType,
    pub game_version: models::PublicGameVersion,
    pub game_player: Option<models::PublicGamePlayer>,
    pub game_players: Vec<models::PublicGamePlayerUser>,
}

#[derive(Serialize)]
pub struct MyActiveResponse {
    games: Vec<MyActiveGame>,
}

#[get("/my_active")]
pub fn my_active(user: models::User) -> Result<CORS<JSON<MyActiveResponse>>> {
    let conn = &*CONN.r.get().chain_err(|| "unable to get connection")?;

    Ok(CORS(JSON(MyActiveResponse {
                     games: query::find_active_games_for_user(&user.id, conn)
                         .chain_err(|| "error getting active_games")?
                         .into_iter()
                         .map(|game_extended| {
        let pg = game_extended.into_public();
        MyActiveGame {
            game: pg.game,
            game_type: pg.game_type,
            game_version: pg.game_version,
            game_player: pg.game_players
                .iter()
                .find(|gp| gp.user.id == user.id)
                .map(|gp| gp.game_player.to_owned()),
            game_players: pg.game_players,
        }
    })
                         .collect(),
                 })))
}
