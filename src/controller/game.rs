use rocket::State;
use rocket_contrib::JSON;
use uuid::Uuid;
use diesel::Connection;
use diesel::pg::PgConnection;
use chrono::Utc;

use brdgme_cmd::cli;
use brdgme_game::{Status, Stat};
use brdgme_game::command::Spec as CommandSpec;
use brdgme_markup as markup;

use std::collections::HashMap;
use std::borrow::Cow;
use std::sync::Mutex;
use std::sync::mpsc::Sender;

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

#[post("/", data = "<data>")]
pub fn create(data: JSON<CreateRequest>, user: models::User) -> Result<CORS<JSON<ShowResponse>>> {
    let user_id = user.id;
    let data = data.into_inner();
    let conn = &*CONN.w.get().chain_err(|| "unable to get connection")?;

    let (created_game, created_logs, public_render, player_renders, user_ids) =
        conn.transaction::<_, Error, _>(move || {
            let opponent_ids = data.opponent_ids.unwrap_or_else(|| vec![]);
            let opponent_emails = data.opponent_emails.unwrap_or_else(|| vec![]);
            let player_count: usize = 1 + opponent_ids.len() + opponent_emails.len();
            let game_version = query::find_game_version(&data.game_version_id, conn)
                .chain_err(|| "error finding game version")?
                .ok_or_else::<Error, _>(|| "could not find game version".into())?;

            let resp = game_client::request(
                &game_version.uri,
                &cli::Request::New { players: player_count },
            )?;
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
            let created_game = query::create_game_with_users(
                &query::CreateGameOpts {
                    new_game: &models::NewGame {
                        game_version_id: data.game_version_id,
                        is_finished: status.is_finished,
                        game_state: &game_info.state,
                    },
                    whose_turn: &status.whose_turn,
                    eliminated: &status.eliminated,
                    winners: &status.winners,
                    points: &game_info.points,
                    creator_id: &user_id,
                    opponent_ids: &opponent_ids,
                    opponent_emails: &opponent_emails,
                },
                conn,
            ).chain_err(|| "unable to create game")?;
            let created_logs = query::create_game_logs_from_cli(&created_game.game.id, logs, conn)
                .chain_err(|| "unable to create game logs")?;
            let mut user_ids = opponent_ids.clone();
            user_ids.push(user_id);
            Ok((
                created_game,
                created_logs,
                public_render,
                player_renders,
                user_ids,
            ))
        }).chain_err(|| "error committing transaction")?;
    let game_extended = query::find_game_extended(&created_game.game.id, conn)
        .chain_err(|| "unable to get extended game")?;
    let player = created_game.players.iter().find(|p| p.user_id == user_id);
    websocket::game_update(
        &game_extended.clone().into_public(),
        &created_logs,
        &public_render,
        &player_renders,
        &query::find_valid_user_auth_tokens_for_users(&user_ids, conn)?,
    )?;
    Ok(CORS(JSON(game_extended_to_show_response(
        player,
        &game_extended,
        player.and_then(|p| player_renders.get(p.position as usize)),
        conn,
    )?)))
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
        } => (
            false,
            whose_turn.clone(),
            eliminated.clone(),
            vec![],
            vec![],
        ),
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
    pub game_players: Vec<models::PublicGamePlayerTypeUser>,
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
            .find(|&gptu| u.id == gptu.user.id)
            .map(|gptu| &gptu.game_player)
    });
    Ok(CORS(JSON(game_extended_to_show_response(
        game_player,
        &game_extended,
        None,
        conn,
    )?)))
}

fn game_extended_to_show_response(
    game_player: Option<&models::GamePlayer>,
    game_extended: &query::GameExtended,
    render: Option<&cli::Render>,
    conn: &PgConnection,
) -> Result<ShowResponse> {
    let public = game_extended.clone().into_public();
    let render: Cow<cli::Render> = match render {
        Some(r) => Cow::Borrowed(r),
        None => {
            match game_client::request(
                &game_extended.game_version.uri,
                &cli::Request::Render {
                    player: game_player.map(|gp| gp.position as usize),
                    game: game_extended.game.game_state.to_owned(),
                },
            )? {
                cli::Response::Render { render } => Cow::Owned(render),
                _ => bail!("invalid render response"),
            }
        }
    };

    let (nodes, _) = markup::from_string(&render.render)
        .chain_err(|| "error parsing render markup")?;

    let markup_players = render::game_players_to_markup_players(&game_extended.game_players)?;
    let game_logs = match game_player {
        Some(gp) => query::find_game_logs_for_player(&gp.id, conn),
        None => query::find_public_game_logs_for_game(&game_extended.game.id, conn),
    }?;

    Ok(ShowResponse {
        game_player: game_player.map(|gp| gp.to_owned().into_public()),
        game: public.game,
        pub_state: render.pub_state.to_owned(),
        game_version: public.game_version,
        game_type: public.game_type,
        game_players: public.game_players,
        html: markup::html(&markup::transform(&nodes, &markup_players)),
        game_logs: game_logs
            .into_iter()
            .map(|gl| gl.into_rendered(&markup_players))
            .collect::<Result<Vec<models::RenderedGameLog>>>()?,
        command_spec: render.command_spec.to_owned(),
    })
}

#[derive(Deserialize)]
pub struct CommandRequest {
    command: String,
}

#[post("/<id>/command", data = "<data>")]
pub fn command(
    id: UuidParam,
    user: models::User,
    game_update_tx: State<Mutex<Sender<websocket::GameUpdateOpts>>>,
    data: JSON<CommandRequest>,
) -> Result<CORS<JSON<ShowResponse>>> {
    let id = id.into_uuid();
    let conn = &*CONN.w.get().chain_err(|| "unable to get connection")?;
    conn.transaction::<_, Error, _>(|| {

        let (game, game_version) = query::find_game_with_version(&id, conn)
            .chain_err(|| "error finding game")?
            .ok_or_else::<Error, _>(|| {
                ErrorKind::UserError("game does not exist".to_string()).into()
            })?;
        if game.is_finished {
            bail!(ErrorKind::UserError("game is already finished".to_string()));
        }

        let players: Vec<(models::GamePlayer, models::User)> =
            query::find_game_players_with_user_by_game(&id, conn)
                .chain_err(|| "error finding game players")?;
        let player: &models::GamePlayer = &players
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
            match game_client::request(
                &game_version.uri,
                &cli::Request::Play {
                    player: position as usize,
                    game: game.game_state.clone(),
                    command: data.command.to_owned(),
                    names: names,
                },
            )? {
                cli::Response::Play {
                    game,
                    logs,
                    can_undo,
                    remaining_input,
                    public_render,
                    player_renders,
                } => (
                    game,
                    logs,
                    can_undo,
                    remaining_input,
                    public_render,
                    player_renders,
                ),
                cli::Response::UserError { message } => bail!(ErrorKind::UserError(message)),
                _ => bail!("invalid response type"),
            };
        let status = game_status_values(&game_response.status);

        let updated = query::update_game_command_success(
            &id,
            &player.id,
            &models::NewGame {
                game_version_id: game.game_version_id,
                is_finished: status.is_finished,
                game_state: &game_response.state,
            },
            if can_undo {
                Some(&game.game_state)
            } else {
                None
            },
            &status.whose_turn,
            &status.eliminated,
            &status.winners,
            &game_response.points,
            conn,
        ).chain_err(|| "error updating game")?;

        let created_logs = query::create_game_logs_from_cli(&id, logs, conn)
            .chain_err(|| "unable to create game logs")?;
        let game_extended = query::find_game_extended(&id, conn)
            .chain_err(|| "unable to get extended game")?;
        let user_ids: Vec<Uuid> = game_extended
            .game_players
            .iter()
            .map(|gptu| gptu.user.id)
            .collect();
        let tx = {
            game_update_tx
                .inner()
                .lock()
                .map_err::<Error, _>(|_| {
                    ErrorKind::Msg("unable to get lock on game_update_tx".to_string()).into()
                })?
                .clone()
        };
        tx.send(websocket::GameUpdateOpts {
            game: game_extended.clone().into_public(),
            game_logs: created_logs.clone(),
            public_render: public_render.clone(),
            player_renders: player_renders.clone(),
            user_auth_tokens: query::find_valid_user_auth_tokens_for_users(&user_ids, conn)?,
        }).map_err::<Error, _>(|_| {
                ErrorKind::Msg("unable to send game update options".to_string()).into()
            })?;
        let gp = game_extended
            .game_players
            .iter()
            .find(|gptu| gptu.game_player.id == player.id)
            .map(|gptu| &gptu.game_player);
        Ok(CORS(JSON(game_extended_to_show_response(
            gp,
            &game_extended,
            gp.and_then(|gp| player_renders.get(gp.position as usize)),
            conn,
        )?)))
    })
}

#[post("/<id>/undo")]
pub fn undo(
    id: UuidParam,
    user: models::User,
    game_update_tx: State<Mutex<Sender<websocket::GameUpdateOpts>>>,
) -> Result<CORS<JSON<ShowResponse>>> {
    let id = id.into_uuid();
    let conn = &*CONN.w.get().chain_err(|| "unable to get connection")?;

    conn.transaction::<_, Error, _>(|| {

        let (game, game_version) = query::find_game_with_version(&id, conn)
            .chain_err(|| "error finding game")?
            .ok_or_else::<Error, _>(|| {
                ErrorKind::UserError("game does not exist".to_string()).into()
            })?;
        if game.is_finished {
            bail!(ErrorKind::UserError("game is already finished".to_string()));
        }

        let player = query::find_game_player_by_user_and_game(&user.id, &id, conn)
            .chain_err(|| "error finding game player")?
            .ok_or_else::<Error, _>(|| {
                ErrorKind::UserError("you aren't a player in this game".to_string()).into()
            })?;

        let undo_state = player
            .undo_game_state
            .clone()
            .ok_or_else::<Error, _>(|| {
                ErrorKind::UserError("you can't undo at the moment".to_string()).into()
            })?;

        let (game_response, public_render, player_renders) = match game_client::request(
            &game_version.uri,
            &cli::Request::Status { game: undo_state.clone() },
        )? {
            cli::Response::Status {
                game,
                public_render,
                player_renders,
            } => (game, public_render, player_renders),
            _ => bail!("invalid response type"),
        };
        let status = game_status_values(&game_response.status);
        let updated = query::update_game_command_success(
            &id,
            &player.id,
            &models::NewGame {
                game_version_id: game.game_version_id,
                is_finished: status.is_finished,
                game_state: &game_response.state,
            },
            None,
            &status.whose_turn,
            &status.eliminated,
            &status.winners,
            &game_response.points,
            conn,
        ).chain_err(|| "error updating game")?;
        query::player_cannot_undo_set_undo_game_state(&id, conn)
            .chain_err(|| "unable to clear undo_game_state for all players")?;
        let created_log = query::create_game_log(
            &models::NewGameLog {
                game_id: id,
                body: &markup::to_string(
                    &[
                        markup::Node::Player(player.position as usize),
                        markup::Node::text(" used an undo"),
                    ],
                ),
                is_public: true,
                logged_at: Utc::now().naive_utc(),
            },
            &[],
            conn,
        ).chain_err(|| "unable to create undo game log")?;
        let game_extended = query::find_game_extended(&id, conn)
            .chain_err(|| "unable to get extended game")?;
        let user_ids: Vec<Uuid> = game_extended
            .game_players
            .iter()
            .map(|gptu| gptu.user.id)
            .collect();
        let tx = {
            game_update_tx
                .inner()
                .lock()
                .map_err::<Error, _>(|_| {
                    ErrorKind::Msg("unable to get lock on game_update_tx".to_string()).into()
                })?
                .clone()
        };
        tx.send(websocket::GameUpdateOpts {
            game: game_extended.clone().into_public(),
            game_logs: vec![created_log],
            public_render: public_render.clone(),
            player_renders: player_renders.clone(),
            user_auth_tokens: query::find_valid_user_auth_tokens_for_users(&user_ids, conn)?,
        }).map_err::<Error, _>(|_| {
                ErrorKind::Msg("unable to send game update options".to_string()).into()
            })?;
        let gp = game_extended
            .game_players
            .iter()
            .find(|gptu| gptu.game_player.id == player.id)
            .map(|gptu| &gptu.game_player);
        Ok(CORS(JSON(game_extended_to_show_response(
            gp,
            &game_extended,
            gp.and_then(|gp| player_renders.get(gp.position as usize)),
            conn,
        )?)))
    })
}

#[post("/<id>/mark_read")]
pub fn mark_read(
    id: UuidParam,
    user: models::User,
) -> Result<CORS<JSON<Option<models::PublicGamePlayer>>>> {
    let id = id.into_uuid();
    let conn = &*CONN.w.get().chain_err(|| "unable to get connection")?;

    conn.transaction::<_, Error, _>(|| {
        Ok(CORS(JSON(
            query::mark_game_read(&id, &user.id, conn)
                .chain_err(|| "error marking game read")?
                .map(|gp| gp.into_public()),
        )))
    })
}

#[post("/<id>/concede")]
pub fn concede(
    id: UuidParam,
    user: models::User,
    game_update_tx: State<Mutex<Sender<websocket::GameUpdateOpts>>>,
) -> Result<CORS<JSON<ShowResponse>>> {
    let id = id.into_uuid();
    let conn = &*CONN.w.get().chain_err(|| "unable to get connection")?;

    conn.transaction::<_, Error, _>(|| {
        let (game, game_version) = query::find_game_with_version(&id, conn)
            .chain_err(|| "error finding game")?
            .ok_or_else::<Error, _>(|| {
                ErrorKind::UserError("game does not exist".to_string()).into()
            })?;
        if game.is_finished {
            bail!(ErrorKind::UserError("game is already finished".to_string()));
        }

        let player_count = query::find_player_count_by_game(&id, conn)
            .chain_err(|| "error finding player count for game")?;
        if player_count > 2 {
            bail!(ErrorKind::UserError(
                "cannot concede games with more than two players".to_string()
            ));
        }

        let player = query::find_game_player_by_user_and_game(&user.id, &id, conn)
            .chain_err(|| "error finding game player")?
            .ok_or_else::<Error, _>(|| {
                ErrorKind::UserError("you aren't a player in this game".to_string()).into()
            })?;

        let updated = query::concede_game(&id, &player.id, conn)
            .chain_err(|| "error conceding game")?;

        let (public_render, player_renders) = match game_client::request(
            &game_version.uri,
            &cli::Request::Status { game: game.game_state.clone() },
        )? {
            cli::Response::Status {
                public_render,
                player_renders,
                ..
            } => (public_render, player_renders),
            _ => bail!("invalid response type"),
        };
        let created_log = query::create_game_log(
            &models::NewGameLog {
                game_id: id,
                body: &markup::to_string(
                    &[
                        markup::Node::Player(player.position as usize),
                        markup::Node::text(" conceded"),
                    ],
                ),
                is_public: true,
                logged_at: Utc::now().naive_utc(),
            },
            &[],
            conn,
        ).chain_err(|| "unable to create concede game log")?;
        let game_extended = query::find_game_extended(&id, conn)
            .chain_err(|| "unable to get extended game")?;
        let user_ids: Vec<Uuid> = game_extended
            .game_players
            .iter()
            .map(|gptu| gptu.user.id)
            .collect();
        let tx = {
            game_update_tx
                .inner()
                .lock()
                .map_err::<Error, _>(|_| {
                    ErrorKind::Msg("unable to get lock on game_update_tx".to_string()).into()
                })?
                .clone()
        };
        tx.send(websocket::GameUpdateOpts {
            game: game_extended.clone().into_public(),
            game_logs: vec![created_log],
            public_render: public_render.clone(),
            player_renders: player_renders.clone(),
            user_auth_tokens: query::find_valid_user_auth_tokens_for_users(&user_ids, conn)?,
        }).map_err::<Error, _>(|_| {
                ErrorKind::Msg("unable to send game update options".to_string()).into()
            })?;
        let gp = game_extended
            .game_players
            .iter()
            .find(|gptu| gptu.game_player.id == player.id)
            .map(|gptu| &gptu.game_player);
        Ok(CORS(JSON(game_extended_to_show_response(
            gp,
            &game_extended,
            gp.and_then(|gp| player_renders.get(gp.position as usize)),
            conn,
        )?)))
    })
}
