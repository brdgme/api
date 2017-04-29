use redis::{self, Client};
use serde_json;

use errors::*;
use config::CONFIG;
use db::models::*;
use db::query::PublicGameExtended;

lazy_static! {
    pub static ref CLIENT: Client = connect().unwrap();
}

pub fn connect() -> Result<Client> {
    Client::open(CONFIG.redis_url.as_ref()).chain_err(|| "unable to open client")
}

#[derive(Serialize)]
pub struct GameUpdateOpts<'a> {
    pub game: &'a PublicGame,
    pub game_version: &'a PublicGameVersion,
}
pub fn game_update<'a>(game: &'a PublicGameExtended) -> Result<()> {
    let conn = CLIENT
        .get_connection()
        .chain_err(|| "unable to get Redis connection from client")?;
    let json_game = serde_json::to_string(game)
        .chain_err(|| "error encoding game state to JSON")?;
    let mut pipe = redis::pipe();
    pipe.cmd("PUBLISH")
        .arg(format!("game.{}", game.game.id))
        .arg(&json_game)
        .ignore();
    for gp in &game.game_players {
        pipe.cmd("PUBLISH")
            .arg(format!("user.{}", gp.user.id))
            .arg(&json_game)
            .ignore();
    }
    pipe.query(&conn)
        .chain_err(|| "error publishing game updates")
}
