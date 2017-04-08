use uuid::Uuid;
use chrono::NaiveDateTime;
use postgres::rows::Row;

use db::color::Color;

#[derive(Debug, PartialEq, Clone)]
pub struct User {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub name: String,
    pub pref_colors: Vec<Color>,
    pub login_confirmation: Option<String>,
    pub login_confirmation_at: Option<NaiveDateTime>,
}

impl User {
    pub fn from_row(row: &Row, prefix: &str) -> Self {
        Self {
            id: row.get(format!("{}id", prefix).as_ref()),
            created_at: row.get(format!("{}created_at", prefix).as_ref()),
            updated_at: row.get(format!("{}updated_at", prefix).as_ref()),
            name: row.get(format!("{}name", prefix).as_ref()),
            pref_colors: row.get(format!("{}pref_colors", prefix).as_ref()),
            login_confirmation: row.get(format!("{}login_confirmation", prefix).as_ref()),
            login_confirmation_at: row.get(format!("{}login_confirmation_at", prefix).as_ref()),
        }
    }
}

impl Model for User {
    fn cols() -> Vec<String> {
        vec!["id".to_string(),
             "created_at".to_string(),
             "updated_at".to_string(),
             "name".to_string(),
             "pref_colors".to_string(),
             "login_confirmation".to_string(),
             "login_confirmation_at".to_string()]
    }
}

pub struct NewUser<'a> {
    pub name: &'a str,
    pub pref_colors: &'a [&'a Color],
    pub login_confirmation: Option<&'a str>,
    pub login_confirmation_at: Option<&'a NaiveDateTime>,
}

pub struct UserEmail {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub user_id: Uuid,
    pub email: String,
    pub is_primary: bool,
}

impl UserEmail {
    pub fn from_row(row: &Row, prefix: &str) -> Self {
        Self {
            id: row.get(format!("{}id", prefix).as_ref()),
            created_at: row.get(format!("{}created_at", prefix).as_ref()),
            updated_at: row.get(format!("{}updated_at", prefix).as_ref()),
            user_id: row.get(format!("{}user_id", prefix).as_ref()),
            email: row.get(format!("{}email", prefix).as_ref()),
            is_primary: row.get(format!("{}is_primary", prefix).as_ref()),
        }
    }
}

impl Model for UserEmail {
    fn cols() -> Vec<String> {
        vec!["id".to_string(),
             "created_at".to_string(),
             "updated_at".to_string(),
             "user_id".to_string(),
             "email".to_string(),
             "is_primary".to_string()]
    }
}

pub struct NewUserEmail<'a> {
    pub user_id: &'a Uuid,
    pub email: &'a str,
    pub is_primary: bool,
}

pub struct UserAuthToken {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub user_id: Uuid,
}

impl UserAuthToken {
    pub fn from_row(row: &Row, prefix: &str) -> Self {
        Self {
            id: row.get(format!("{}id", prefix).as_ref()),
            created_at: row.get(format!("{}created_at", prefix).as_ref()),
            updated_at: row.get(format!("{}updated_at", prefix).as_ref()),
            user_id: row.get(format!("{}user_id", prefix).as_ref()),
        }
    }
}

impl Model for UserAuthToken {
    fn cols() -> Vec<String> {
        vec!["id".to_string(),
             "created_at".to_string(),
             "updated_at".to_string(),
             "user_id".to_string()]
    }
}

pub struct NewUserAuthToken<'a> {
    pub user_id: &'a Uuid,
}

pub struct GameType {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub name: String,
}

impl GameType {
    pub fn from_row(row: &Row, prefix: &str) -> Self {
        Self {
            id: row.get(format!("{}id", prefix).as_ref()),
            created_at: row.get(format!("{}created_at", prefix).as_ref()),
            updated_at: row.get(format!("{}updated_at", prefix).as_ref()),
            name: row.get(format!("{}name", prefix).as_ref()),
        }
    }
}

pub struct NewGameType<'a> {
    pub name: &'a str,
}

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
    pub fn from_row(row: &Row, prefix: &str) -> Self {
        Self {
            id: row.get(format!("{}id", prefix).as_ref()),
            created_at: row.get(format!("{}created_at", prefix).as_ref()),
            updated_at: row.get(format!("{}updated_at", prefix).as_ref()),
            game_type_id: row.get(format!("{}game_type_id", prefix).as_ref()),
            name: row.get(format!("{}name", prefix).as_ref()),
            uri: row.get(format!("{}uri", prefix).as_ref()),
            is_public: row.get(format!("{}is_public", prefix).as_ref()),
            is_deprecated: row.get(format!("{}is_deprecated", prefix).as_ref()),
        }
    }
}

pub struct NewGameVersion<'a> {
    pub game_type_id: &'a Uuid,
    pub name: &'a str,
    pub uri: &'a str,
    pub is_public: bool,
    pub is_deprecated: bool,
}

pub struct Game {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_version_id: Uuid,
    pub is_finished: bool,
    pub game_state: String,
}

impl Game {
    pub fn from_row(row: &Row, prefix: &str) -> Self {
        Self {
            id: row.get(format!("{}id", prefix).as_ref()),
            created_at: row.get(format!("{}created_at", prefix).as_ref()),
            updated_at: row.get(format!("{}updated_at", prefix).as_ref()),
            game_version_id: row.get(format!("{}game_version_id", prefix).as_ref()),
            is_finished: row.get(format!("{}is_finished", prefix).as_ref()),
            game_state: row.get(format!("{}game_state", prefix).as_ref()),
        }
    }
}

pub struct NewGame<'a> {
    pub game_version_id: &'a Uuid,
    pub is_finished: bool,
    pub game_state: &'a str,
}

pub struct GamePlayer {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_id: Uuid,
    pub user_id: Uuid,
    pub position: i32,
    pub color: Color,
    pub has_accepted: bool,
    pub is_turn: bool,
    pub is_eliminated: bool,
    pub is_winner: bool,
}

impl GamePlayer {
    pub fn from_row(row: &Row, prefix: &str) -> Self {
        Self {
            id: row.get(format!("{}id", prefix).as_ref()),
            created_at: row.get(format!("{}created_at", prefix).as_ref()),
            updated_at: row.get(format!("{}updated_at", prefix).as_ref()),
            game_id: row.get(format!("{}game_id", prefix).as_ref()),
            user_id: row.get(format!("{}user_id", prefix).as_ref()),
            position: row.get(format!("{}position", prefix).as_ref()),
            color: row.get(format!("{}color", prefix).as_ref()),
            has_accepted: row.get(format!("{}has_accepted", prefix).as_ref()),
            is_turn: row.get(format!("{}is_turn", prefix).as_ref()),
            is_eliminated: row.get(format!("{}is_eliminated", prefix).as_ref()),
            is_winner: row.get(format!("{}is_winner", prefix).as_ref()),
        }
    }
}

pub struct NewGamePlayer<'a> {
    pub game_id: &'a Uuid,
    pub user_id: &'a Uuid,
    pub position: i32,
    pub color: &'a Color,
    pub has_accepted: bool,
    pub is_turn: bool,
    pub is_eliminated: bool,
    pub is_winner: bool,
}

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
    pub fn from_row(row: &Row, prefix: &str) -> Self {
        Self {
            id: row.get(format!("{}id", prefix).as_ref()),
            created_at: row.get(format!("{}created_at", prefix).as_ref()),
            updated_at: row.get(format!("{}updated_at", prefix).as_ref()),
            game_id: row.get(format!("{}game_id", prefix).as_ref()),
            body: row.get(format!("{}body", prefix).as_ref()),
            is_public: row.get(format!("{}is_public", prefix).as_ref()),
            logged_at: row.get(format!("{}logged_at", prefix).as_ref()),
        }
    }
}

pub struct NewGameLog<'a> {
    pub game_id: &'a Uuid,
    pub body: &'a str,
    pub is_public: bool,
    pub logged_at: &'a NaiveDateTime,
}

pub struct GameLogTarget {
    pub id: Uuid,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub game_log_id: Uuid,
    pub player_id: Uuid,
}

impl GameLogTarget {
    pub fn from_row(row: &Row, prefix: &str) -> Self {
        Self {
            id: row.get(format!("{}id", prefix).as_ref()),
            created_at: row.get(format!("{}created_at", prefix).as_ref()),
            updated_at: row.get(format!("{}updated_at", prefix).as_ref()),
            game_log_id: row.get(format!("{}game_log_id", prefix).as_ref()),
            player_id: row.get(format!("{}player_id", prefix).as_ref()),
        }
    }
}

pub struct NewGameLogTarget<'a> {
    pub game_log_id: &'a Uuid,
    pub player_id: &'a Uuid,
}

pub trait Model {
    fn cols() -> Vec<String>;

    fn select_cols(table: &str, prefix: &str) -> String {
        let mut table = table.to_string();
        if !table.is_empty() {
            table = format!("{}.", table);
        }
        Self::cols()
            .iter()
            .map(|c| {
                     format!("{table}{col} AS {prefix}{col}",
                             table = table,
                             prefix = prefix,
                             col = c)
                 })
            .collect::<Vec<String>>()
            .join(", ")
    }
}
