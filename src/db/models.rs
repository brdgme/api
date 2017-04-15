use uuid::Uuid;
use chrono::NaiveDateTime;

use brdgme_markup as markup;

use db::schema::*;
use errors::*;

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations)]
#[has_many(user_emails)]
#[has_many(game_players)]
#[has_many(user_auth_tokens)]
pub struct User {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub name: String,
    pub pref_colors: Vec<String>,
    pub login_confirmation: Option<String>,
    pub login_confirmation_at: Option<NaiveDateTime>,
}

impl User {
    pub fn into_public(self) -> PublicUser {
        PublicUser {
            id: self.id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            name: self.name,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PublicUser {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub name: String,
}

#[derive(Insertable)]
#[table_name="users"]
pub struct NewUser<'a> {
    pub name: &'a str,
    pub pref_colors: &'a [&'a str],
    pub login_confirmation: Option<&'a str>,
    pub login_confirmation_at: Option<NaiveDateTime>,
}

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations)]
#[belongs_to(User)]
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

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations)]
#[belongs_to(User)]
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

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations, Serialize, Deserialize)]
#[has_many(game_versions)]
pub struct GameType {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub name: String,
}

pub type PublicGameType = GameType;

#[derive(Insertable)]
#[table_name="game_types"]
pub struct NewGameType<'a> {
    pub name: &'a str,
}

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations, Serialize, Deserialize)]
#[belongs_to(GameType)]
#[has_many(games)]
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

impl GameVersion {
    pub fn into_public(self) -> PublicGameVersion {
        PublicGameVersion {
            id: self.id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            game_type_id: self.game_type_id,
            name: self.name,
            is_public: self.is_public,
            is_deprecated: self.is_deprecated,
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PublicGameVersion {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_type_id: Uuid,
    pub name: String,
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

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations)]
#[belongs_to(GameVersion)]
#[has_many(game_players)]
#[has_many(game_logs)]
pub struct Game {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_version_id: Uuid,
    pub is_finished: bool,
    pub game_state: String,
}

#[derive(Debug, PartialEq, Clone, Serialize, Deserialize)]
pub struct PublicGame {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_version_id: Uuid,
    pub is_finished: bool,
}

impl Game {
    pub fn into_public(self) -> PublicGame {
        PublicGame {
            id: self.id,
            created_at: self.created_at,
            updated_at: self.updated_at,
            game_version_id: self.game_version_id,
            is_finished: self.is_finished,
        }
    }
}

#[derive(Insertable)]
#[table_name="games"]
pub struct NewGame<'a> {
    pub game_version_id: Uuid,
    pub is_finished: bool,
    pub game_state: &'a str,
}

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations, Serialize, Deserialize)]
#[belongs_to(Game)]
#[belongs_to(User)]
#[has_many(game_log_targets)]
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

pub type PublicGamePlayer = GamePlayer;

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

#[derive(Serialize, Deserialize, Clone)]
pub struct PublicGamePlayerUser {
    pub game_player: PublicGamePlayer,
    pub user: PublicUser,
}

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations, Serialize, Deserialize)]
#[belongs_to(Game)]
#[has_many(game_log_targets)]
pub struct GameLog {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_id: Uuid,
    pub body: String,
    pub is_public: bool,
    pub logged_at: NaiveDateTime,
}

pub type PublicGameLog = GameLog;

#[derive(Serialize, Deserialize)]
pub struct RenderedGameLog {
    game_log: PublicGameLog,
    html: String,
}

impl GameLog {
    fn render(&self, players: &[markup::Player]) -> Result<String> {
        let (parsed, _) = markup::from_string(&self.body)
            .chain_err(|| "error parsing log body")?;
        Ok(markup::html(&markup::transform(&parsed, players)))
    }

    pub fn into_rendered(self, players: &[markup::Player]) -> Result<RenderedGameLog> {
        let html = self.render(players)?;
        Ok(RenderedGameLog {
               game_log: self,
               html: html,
           })
    }
}

#[derive(Insertable)]
#[table_name="game_logs"]
pub struct NewGameLog<'a> {
    pub game_id: Uuid,
    pub body: &'a str,
    pub is_public: bool,
    pub logged_at: NaiveDateTime,
}

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations)]
#[belongs_to(GameLog)]
#[belongs_to(GamePlayer)]
pub struct GameLogTarget {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_log_id: Uuid,
    pub game_player_id: Uuid,
}

#[derive(Insertable)]
#[table_name="game_log_targets"]
pub struct NewGameLogTarget {
    pub game_log_id: Uuid,
    pub game_player_id: Uuid,
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
