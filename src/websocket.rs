use redis::{self, Client};
use serde_json;
use uuid::Uuid;

use brdgme_cmd::cli;
use brdgme_markup as markup;

use std::sync::mpsc::{channel, Sender, Receiver};

use errors::*;
use config::CONFIG;
use db::models::*;
use db::query::{PublicGameExtended, CreatedGameLog};
use render;
use controller::game::ShowResponse;

lazy_static! {
    pub static ref CLIENT: Client = connect().unwrap();
}

pub fn connect() -> Result<Client> {
    Client::open(CONFIG.redis_url.as_ref()).chain_err(|| "unable to open client")
}

pub struct GameUpdater {
    rx: Receiver<GameUpdateOpts>,
}

impl GameUpdater {
    pub fn new() -> (Self, Sender<GameUpdateOpts>) {
        let (tx, rx) = channel();
        (GameUpdater { rx }, tx)
    }

    pub fn run(&self) {
        loop {
            match self.rx.recv() {
                Ok(opts) => {
                    if let Err(e) = game_update(&opts.game,
                                                &opts.game_logs,
                                                &opts.public_render,
                                                &opts.player_renders,
                                                &opts.user_auth_tokens) {
                        warn!("error sending game update: {}", e);
                    }
                }
                Err(e) => warn!("error receiving game update options: {}", e),
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct GameUpdateOpts {
    pub game: PublicGameExtended,
    pub game_logs: Vec<CreatedGameLog>,
    pub public_render: cli::Render,
    pub player_renders: Vec<cli::Render>,
    pub user_auth_tokens: Vec<UserAuthToken>,
}

fn created_logs_for_player(player_id: Option<Uuid>,
                           logs: &[CreatedGameLog],
                           players: &[markup::Player])
                           -> Result<Vec<RenderedGameLog>> {
    logs.iter()
        .filter(|gl| {
                    gl.game_log.is_public ||
                    player_id
                        .and_then(|p_id| gl.targets.iter().find(|t| t.game_player_id == p_id))
                        .is_some()
                })
        .map(|gl| Ok(gl.game_log.to_owned().into_rendered(players)?))
        .collect()
}

pub fn game_update<'a>(game: &'a PublicGameExtended,
                       game_logs: &[CreatedGameLog],
                       public_render: &cli::Render,
                       player_renders: &[cli::Render],
                       user_auth_tokens: &[UserAuthToken])
                       -> Result<()> {
    let conn = CLIENT
        .get_connection()
        .chain_err(|| "unable to get Redis connection from client")?;
    let markup_players = render::public_game_players_to_markup_players(&game.game_players)?;
    let mut pipe = redis::pipe();
    pipe.cmd("PUBLISH")
        .arg(format!("game.{}", game.game.id))
        .arg(&serde_json::to_string(&ShowResponse {
                                       game_player: None,
                                       game: game.game.to_owned(),
                                       game_type: game.game_type.to_owned(),
                                       game_version: game.game_version.to_owned(),
                                       game_players: game.game_players.to_owned(),
                                       game_logs: created_logs_for_player(None,
                                                                          game_logs,
                                                                          &markup_players)?,
                                       pub_state: public_render.pub_state.to_owned(),
                                       html: render::markup_html(&public_render.render,
                                                                 &markup_players)?,
                                       command_spec: None,
                                   })
                     .chain_err(|| "unable to convert game to JSON")?)
        .ignore();
    for gp in &game.game_players {
        let player_render = match player_renders.get(gp.game_player.position as usize) {
            Some(pr) => pr,
            None => continue,
        };
        let player_message = ShowResponse {
            game_player: Some(gp.game_player.to_owned()),
            game: game.game.to_owned(),
            game_type: game.game_type.to_owned(),
            game_version: game.game_version.to_owned(),
            game_players: game.game_players.to_owned(),
            game_logs: created_logs_for_player(Some(gp.game_player.id),
                                               game_logs,
                                               &markup_players)?,
            pub_state: player_render.pub_state.to_owned(),
            html: render::markup_html(&player_render.render, &markup_players)?,
            command_spec: player_render.command_spec.to_owned(),
        };
        for uat in user_auth_tokens {
            if uat.user_id == gp.user.id {
                pipe.cmd("PUBLISH")
                    .arg(format!("user.{}", uat.id))
                    .arg(&serde_json::to_string(&player_message)
                              .chain_err(|| "unable to convert game to JSON")?)
                    .ignore();
            }
        }
    }
    pipe.query(&conn)
        .chain_err(|| "error publishing game updates")
}
