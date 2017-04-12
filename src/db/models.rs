use uuid::Uuid;
use chrono::NaiveDateTime;
use rustless::json::JsonValue;

use std::collections::BTreeMap;

use db::schema::*;

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
    pub fn to_public_json(&self) -> JsonValue {
        JsonValue::Object({
                              let mut props = BTreeMap::new();
                              props.insert("id".to_string(),
                                           JsonValue::String(self.id.to_string()));
                              props.insert("name".to_string(),
                                           JsonValue::String(self.name.to_owned()));
                              props.insert("pref_colors".to_string(), JsonValue::Array(self.pref_colors.iter().map(|pc|
            JsonValue::String(pc.to_owned())).collect()));
                              props
                          })
    }
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

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations)]
#[has_many(game_versions)]
pub struct GameType {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub name: String,
}

impl GameType {
    pub fn to_public_json(&self) -> JsonValue {
        JsonValue::Object({
                              let mut props = BTreeMap::new();
                              props.insert("id".to_string(),
                                           JsonValue::String(self.id.to_string()));
                              props.insert("name".to_string(),
                                           JsonValue::String(self.name.to_owned()));
                              props
                          })
    }
}

#[derive(Insertable)]
#[table_name="game_types"]
pub struct NewGameType<'a> {
    pub name: &'a str,
}

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations)]
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
    pub fn to_public_json(&self) -> JsonValue {
        JsonValue::Object({
                              let mut props = BTreeMap::new();
                              props.insert("id".to_string(),
                                           JsonValue::String(self.id.to_string()));
                              props.insert("game_type_id".to_string(),
                                           JsonValue::String(self.game_type_id.to_string()));
                              props.insert("name".to_string(),
                                           JsonValue::String(self.name.to_owned()));
                              props.insert("is_public".to_string(),
                                           JsonValue::Bool(self.is_public));
                              props.insert("is_deprecated".to_string(),
                                           JsonValue::Bool(self.is_deprecated));
                              props
                          })
    }
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

impl Game {
    pub fn to_public_json(&self) -> JsonValue {
        JsonValue::Object({
                              let mut props = BTreeMap::new();
                              props.insert("id".to_string(),
                                           JsonValue::String(self.id.to_string()));
                              props.insert("game_version_id".to_string(),
                                           JsonValue::String(self.game_version_id.to_string()));
                              props.insert("is_finished".to_string(),
                                           JsonValue::Bool(self.is_finished));
                              props
                          })
    }
}

#[derive(Insertable)]
#[table_name="games"]
pub struct NewGame<'a> {
    pub game_version_id: Uuid,
    pub is_finished: bool,
    pub game_state: &'a str,
}

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations)]
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

impl GamePlayer {
    pub fn to_public_json(&self) -> JsonValue {
        JsonValue::Object({
                              let mut props = BTreeMap::new();
                              props.insert("id".to_string(),
                                           JsonValue::String(self.id.to_string()));
                              props.insert("position".to_string(),
                                           JsonValue::U64(self.position as u64));
                              props.insert("color".to_string(),
                                           JsonValue::String(self.color.to_owned()));
                              props.insert("has_accepted".to_string(),
                                           JsonValue::Bool(self.has_accepted));
                              props.insert("is_turn".to_string(), JsonValue::Bool(self.is_turn));
                              props.insert("is_eliminated".to_string(),
                                           JsonValue::Bool(self.is_eliminated));
                              props.insert("is_winner".to_string(),
                                           JsonValue::Bool(self.is_winner));
                              props
                          })
    }
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

#[derive(Debug, PartialEq, Clone, Queryable, Identifiable, Associations)]
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

impl GameLog {
    pub fn to_public_json(&self) -> JsonValue {
        JsonValue::Object({
                              let mut props = BTreeMap::new();
                              props.insert("id".to_string(),
                                           JsonValue::String(self.id.to_string()));
                              props.insert("game_id".to_string(),
                                           JsonValue::String(self.game_id.to_string()));
                              props.insert("is_public".to_string(),
                                           JsonValue::Bool(self.is_public));
                              props.insert("body".to_string(),
                                           JsonValue::String(self.body.to_owned()));
                              props.insert("logged_at".to_string(),
                                           JsonValue::String(self.logged_at.to_string()));
                              props
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
