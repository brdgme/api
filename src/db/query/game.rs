use diesel;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use uuid::Uuid;

use errors::*;
use db::models::*;

pub fn update_chat_id(game_id: &Uuid, chat_id: &Uuid, conn: &PgConnection) -> Result<Option<Game>> {
    use db::schema::games;

    diesel::update(games::table.find(game_id))
        .set(games::chat_id.eq(chat_id))
        .get_result(conn)
        .optional()
        .chain_err(|| "error updating chat_id for game")
}

pub fn update_restarted_game_id(
    game_id: &Uuid,
    restarted_game_id: &Uuid,
    conn: &PgConnection,
) -> Result<Option<Game>> {
    use db::schema::games;

    diesel::update(games::table.find(game_id))
        .set(games::restarted_game_id.eq(restarted_game_id))
        .get_result(conn)
        .optional()
        .chain_err(|| "error updating restarted_game_id for game")
}
