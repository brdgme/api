use uuid::Uuid;
use chrono::NaiveDateTime;

use db::schema::*;

#[derive(Debug, PartialEq, Clone, Queryable)]
pub struct User {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub name: String,
    pub pref_colors: Vec<String>,
    pub login_confirmation: Option<String>,
    pub login_confirmation_at: Option<NaiveDateTime>,
}

#[derive(Insertable)]
#[table_name="users"]
pub struct NewUser<'a> {
    pub name: &'a str,
    pub pref_colors: &'a [&'a str],
    pub login_confirmation: Option<&'a str>,
    pub login_confirmation_at: Option<NaiveDateTime>,
}

#[derive(Queryable)]
pub struct UserEmail {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub user_id: Uuid,
    pub email: String,
    pub is_primary: bool,
}

#[derive(Insertable)]
#[table_name="user_emails"]
pub struct NewUserEmail<'a> {
    pub user_id: Uuid,
    pub email: &'a str,
    pub is_primary: bool,
}

#[derive(Queryable)]
pub struct UserAuthToken {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub user_id: Uuid,
}

#[derive(Insertable)]
#[table_name="user_auth_tokens"]
pub struct NewUserAuthToken {
    pub user_id: Uuid,
}

#[derive(Queryable)]
pub struct GameType {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub name: String,
}

#[derive(Insertable)]
#[table_name="game_types"]
pub struct NewGameType<'a> {
    pub name: &'a str,
}

#[derive(Queryable)]
pub struct GameVersion {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_type_id: Uuid,
    pub name: String,
    pub uri: String,
    pub is_public: bool,
    pub is_deprecated: bool,
}

#[derive(Insertable)]
#[table_name="game_versions"]
pub struct NewGameVersion<'a> {
    pub game_type_id: Uuid,
    pub name: &'a str,
    pub uri: &'a str,
    pub is_public: bool,
    pub is_deprecated: bool,
}

#[derive(Queryable)]
pub struct Game {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_version_id: Uuid,
    pub is_finished: bool,
    pub game_state: String,
}

#[derive(Insertable)]
#[table_name="games"]
pub struct NewGame<'a> {
    pub game_version_id: Uuid,
    pub is_finished: bool,
    pub game_state: &'a str,
}

#[derive(Queryable, Clone)]
pub struct GamePlayer {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_id: Uuid,
    pub user_id: Uuid,
    pub position: i32,
    pub color: String,
    pub has_accepted: bool,
    pub is_turn: bool,
    pub is_eliminated: bool,
    pub is_winner: bool,
    pub is_read: bool,
}

#[derive(Insertable)]
#[table_name="game_players"]
pub struct NewGamePlayer<'a> {
    pub game_id: Uuid,
    pub user_id: Uuid,
    pub position: i32,
    pub color: &'a str,
    pub has_accepted: bool,
    pub is_turn: bool,
    pub is_eliminated: bool,
    pub is_winner: bool,
    pub is_read: bool,
}

#[derive(Queryable)]
pub struct GameLog {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_id: Uuid,
    pub body: String,
    pub is_public: bool,
    pub logged_at: NaiveDateTime,
}

#[derive(Insertable)]
#[table_name="game_logs"]
pub struct NewGameLog<'a> {
    pub game_id: Uuid,
    pub body: &'a str,
    pub is_public: bool,
    pub logged_at: NaiveDateTime,
}

#[derive(Queryable)]
pub struct GameLogTarget {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_log_id: Uuid,
    pub player_id: Uuid,
}

#[derive(Insertable)]
#[table_name="game_log_targets"]
pub struct NewGameLogTarget {
    pub game_log_id: Uuid,
    pub player_id: Uuid,
}

#[cfg(test)]
mod tests {
    use super::*;
    use diesel::{self, Connection};
    use diesel::prelude::*;
    use db::color::Color;
    use db::{Connections, connect_env, schema};

    lazy_static! {
        static ref CONN: Connections = connect_env().unwrap();
    }

    #[test]
    #[ignore]
    fn insert_user_works() {
        let conn = &*CONN.w.get().unwrap();
        conn.begin_test_transaction().unwrap();
        diesel::insert(&NewUser {
                            name: "blah",
                            pref_colors: &[&Color::Green.to_string()],
                            login_confirmation: None,
                            login_confirmation_at: None,
                        })
                .into(schema::users::table)
                .get_result::<User>(conn)
                .expect("Error inserting user");
    }
}
