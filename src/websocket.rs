use redis::{self, Client};
use serde_json;
use uuid::Uuid;

use brdgme_cmd::cli;
use brdgme_markup as markup;

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

fn created_logs_for_player(player_id: Option<Uuid>,
                           logs: &[CreatedGameLog],
                           players: &[markup::Player])
                           -> Vec<RenderedGameLog> {
    logs.iter()
        .filter_map(|gl| if gl.game_log.is_public ||
                            player_id
                                .map(|p_id| {
                                         gl.targets.iter().find(|t| t.game_player_id == p_id)
                                     })
                                .is_some() {
                        Some(gl.game_log.to_owned().into_rendered(players).unwrap())
                    } else {
                        None
                    })
        .collect()
}

#[derive(Serialize)]
pub struct GameUpdateOpts<'a> {
    pub game: &'a PublicGame,
    pub game_version: &'a PublicGameVersion,
}
pub fn game_update<'a>(game: &'a PublicGameExtended,
                       game_logs: &[CreatedGameLog],
                       public_render: &cli::Render,
                       player_renders: &[cli::Render])
                       -> Result<()> {
    let conn = CLIENT
        .get_connection()
        .chain_err(|| "unable to get Redis connection from client")?;
    let markup_players = render::public_game_players_to_markup_players(&game.game_players);
    let mut pipe = redis::pipe();
    pipe.cmd("PUBLISH")
        .arg(format!("game.{}", game.game.id))
        .arg(&serde_json::to_string(&ShowResponse {
                                         game: game.game.to_owned(),
                                         game_type: game.game_type.to_owned(),
                                         game_version: game.game_version.to_owned(),
                                         game_players: game.game_players.to_owned(),
                                         game_logs: created_logs_for_player(None,
                                                                            game_logs,
                                                                            &markup_players),
                                         pub_state: public_render.pub_state.to_owned(),
                                         game_html: render::markup_html(&public_render.render,
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
        pipe.cmd("PUBLISH")
            .arg(format!("user.{}", gp.user.id))
            .arg(&serde_json::to_string(&ShowResponse {
                                             game: game.game.to_owned(),
                                             game_type: game.game_type.to_owned(),
                                             game_version: game.game_version.to_owned(),
                                             game_players: game.game_players.to_owned(),
                                             game_logs:
                                                 created_logs_for_player(Some(gp.game_player.id),
                                                                         game_logs,
                                                                         &markup_players),
                                             pub_state: player_render.pub_state.to_owned(),
                                             game_html: render::markup_html(&player_render.render,
                                                                            &markup_players)?,
                                             command_spec: player_render.command_spec.to_owned(),
                                         })
                          .chain_err(|| "unable to convert game to JSON")?)
            .ignore();
    }
    pipe.query(&conn)
        .chain_err(|| "error publishing game updates")
}
