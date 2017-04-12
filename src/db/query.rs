use diesel;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use uuid::Uuid;
use rand::{self, Rng};
use chrono::{Duration, UTC};

use brdgme_cmd::cli::CliLog;

use std::collections::{HashSet, HashMap};
use std::iter::FromIterator;

use errors::*;
use db::models::*;
use db::color::{self, Color};

lazy_static! {
    static ref CONFIRMATION_EXPIRY: Duration = Duration::minutes(30);
    static ref TOKEN_EXPIRY: Duration = Duration::days(30);
    static ref FINISHED_GAME_RELEVANCE: Duration = Duration::days(3);
}

pub fn create_user_by_name(name: &str, conn: &PgConnection) -> Result<User> {
    use db::schema::users;
    diesel::insert(&NewUser {
                        name: name,
                        pref_colors: &[],
                        login_confirmation: None,
                        login_confirmation_at: None,
                    })
            .into(users::table)
            .get_result(conn)
            .chain_err(|| "error creating user")
}

pub fn find_user(find_id: &Uuid, conn: &PgConnection) -> Result<Option<User>> {
    use db::schema::users;

    users::table
        .find(find_id)
        .first(conn)
        .optional()
        .chain_err(|| "error finding user")
}

pub fn find_user_by_email(by_email: &str,
                          conn: &PgConnection)
                          -> Result<Option<(UserEmail, User)>> {
    use db::schema::{user_emails, users};

    user_emails::table
        .filter(user_emails::email.eq(by_email))
        .limit(1)
        .inner_join(users::table)
        .first::<(UserEmail, User)>(conn)
        .optional()
        .chain_err(|| "error finding user")
}

pub fn find_or_create_user_by_email(email: &str, conn: &PgConnection) -> Result<(UserEmail, User)> {
    if let Some(v) = find_user_by_email(email, conn)? {
        return Ok(v);
    }
    create_user_by_email(email, conn)
}

pub fn create_user_by_email(email: &str, conn: &PgConnection) -> Result<(UserEmail, User)> {
    conn.transaction(|| {
        let u = create_user_by_name(email, conn)?;
        let ue = create_user_email(&NewUserEmail {
                                        user_id: u.id,
                                        email: email,
                                        is_primary: true,
                                    },
                                   conn)?;
        Ok((ue, u))
    })
}

pub fn create_user_email(ue: &NewUserEmail, conn: &PgConnection) -> Result<UserEmail> {
    use db::schema::user_emails;
    diesel::insert(ue)
        .into(user_emails::table)
        .get_result(conn)
        .chain_err(|| "error creating user email")
}

fn rand_code() -> String {
    let mut rng = rand::thread_rng();
    format!("{}{:05}",
            (rng.gen::<usize>() % 9) + 1,
            rng.gen::<usize>() % 100000)
}

pub fn generate_user_login_confirmation(user_id: &Uuid, conn: &PgConnection) -> Result<String> {
    use db::schema::users;

    let code = rand_code();
    diesel::update(users::table.find(user_id))
        .set((users::login_confirmation.eq(&code),
              users::login_confirmation_at.eq(UTC::now().naive_utc())))
        .execute(conn)?;
    Ok(code)
}

pub fn user_login_request(email: &str, conn: &PgConnection) -> Result<String> {
    conn.transaction(|| {
        let (_, user) = find_or_create_user_by_email(email, conn)?;

        let confirmation = match (user.login_confirmation, user.login_confirmation_at) {
            (Some(ref uc), Some(at)) if at + *CONFIRMATION_EXPIRY > UTC::now().naive_utc() => {
                uc.to_owned()
            }
            _ => generate_user_login_confirmation(&user.id, conn)?,
        };
        Ok(confirmation)
    })
}

pub fn user_login_confirm(email: &str,
                          confirmation: &str,
                          conn: &PgConnection)
                          -> Result<Option<UserAuthToken>> {
    let user = match find_user_by_email(email, conn)? {
        Some((_, u)) => u,
        None => return Ok(None),
    };
    Ok(match (user.login_confirmation, user.login_confirmation_at) {
           (Some(ref uc), Some(at)) if at + *CONFIRMATION_EXPIRY > UTC::now().naive_utc() &&
                                       uc == confirmation => {
               Some(create_auth_token(&user.id, conn)?)
           }
           _ => None,
       })
}

pub fn create_auth_token(for_user_id: &Uuid, conn: &PgConnection) -> Result<UserAuthToken> {
    use db::schema::user_auth_tokens;

    diesel::insert(&NewUserAuthToken { user_id: *for_user_id })
        .into(user_auth_tokens::table)
        .get_result::<UserAuthToken>(conn)
        .chain_err(|| "error creating auth token")
}

pub fn authenticate(search_email: &str,
                    search_token: &Uuid,
                    conn: &PgConnection)
                    -> Result<Option<(UserEmail, User)>> {
    use db::schema::{users, user_emails, user_auth_tokens};

    let uat: UserAuthToken = match user_auth_tokens::table
              .find(search_token)
              .filter(user_auth_tokens::created_at.gt(UTC::now().naive_utc() -
                                                      *TOKEN_EXPIRY))
              .first(conn)
              .optional()? {
        Some(v) => v,
        None => return Ok(None),
    };

    user_emails::table
        .filter(user_emails::email.eq(search_email))
        .filter(user_emails::user_id.eq(uat.user_id))
        .inner_join(users::table)
        .first(conn)
        .optional()
        .chain_err(|| "error finding user")
}

pub fn find_game_version(id: &Uuid, conn: &PgConnection) -> Result<Option<GameVersion>> {
    use db::schema::game_versions;

    game_versions::table
        .find(id)
        .first(conn)
        .optional()
        .chain_err(|| "error finding game version")
}

pub fn find_game_with_version(id: &Uuid,
                              conn: &PgConnection)
                              -> Result<Option<(Game, GameVersion)>> {
    use db::schema::{games, game_versions};

    games::table
        .find(id)
        .inner_join(game_versions::table)
        .first(conn)
        .optional()
        .chain_err(|| "error finding game")
}

pub struct GameExtended {
    pub game: Game,
    pub game_type: GameType,
    pub game_version: GameVersion,
    pub game_players: Vec<(GamePlayer, User)>,
}
pub fn find_active_games_for_user(id: &Uuid, conn: &PgConnection) -> Result<Vec<GameExtended>> {
    use db::schema::{games, game_players, game_versions, game_types};

    Ok(games::table
           .inner_join(game_players::table)
           .filter(game_players::user_id.eq(id))
           .filter(games::is_finished
                       .eq(false)
                       .or(games::updated_at
                               .gt(UTC::now().naive_utc() - *FINISHED_GAME_RELEVANCE)
                               .and(game_players::is_read.eq(false))))
           .get_results::<(Game, GamePlayer)>(conn)?
           .iter()
           .map(|&(ref game, _)| {
        let game_version: GameVersion = game_versions::table
            .find(game.game_version_id)
            .get_result(conn)
            .unwrap();
        let game_type: GameType = game_types::table
            .find(game_version.game_type_id)
            .get_result(conn)
            .unwrap();
        let players = find_game_players_with_user_by_game(&game.id, conn).unwrap();
        GameExtended {
            game: game.clone(),
            game_type: game_type,
            game_version: game_version,
            game_players: players,
        }
    })
           .collect())
}

pub fn find_game_extended(id: &Uuid, conn: &PgConnection) -> Result<GameExtended> {
    use db::schema::{games, game_versions, game_types};

    let (game, game_version) = games::table
        .find(id)
        .inner_join(game_versions::table)
        .get_result::<(Game, GameVersion)>(conn)?;
    let game_type: GameType = game_types::table
        .find(game_version.game_type_id)
        .get_result(conn)?;
    let players = find_game_players_with_user_by_game(&game.id, conn)?;
    Ok(GameExtended {
           game: game.clone(),
           game_type: game_type,
           game_version: game_version,
           game_players: players,
       })
}

pub struct CreatedGame {
    pub game: Game,
    pub opponents: Vec<(UserEmail, User)>,
    pub players: Vec<GamePlayer>,
}
pub struct CreateGameOpts<'a> {
    pub new_game: &'a NewGame<'a>,
    pub whose_turn: &'a [usize],
    pub eliminated: &'a [usize],
    pub winners: &'a [usize],
    pub creator_id: &'a Uuid,
    pub opponent_ids: &'a [Uuid],
    pub opponent_emails: &'a [String],
}
pub fn create_game_with_users(opts: &CreateGameOpts, conn: &PgConnection) -> Result<CreatedGame> {
    conn.transaction(|| {

        // Find or create users.
        let creator = find_user(opts.creator_id, conn)
            .chain_err(|| "could not find creator")?
            .ok_or_else::<Error, _>(|| "could not find creator".into())?;
        let opponents = create_game_users(opts.opponent_ids, opts.opponent_emails, conn)
            .chain_err(|| "could not create game users")?;
        let mut users: Vec<User> = opponents.iter().map(|&(_, ref u)| u.clone()).collect();
        users.push(creator);

        // Randomise the users so player order is random.
        let mut rnd = rand::thread_rng();
        rnd.shuffle(&mut users);

        // Assign colors to each player using preferences.
        let color_prefs: Vec<Vec<Color>> = users
            .iter()
            .map(|u| Color::from_strings(&u.pref_colors).unwrap())
            .collect();
        let player_colors = color::choose(&HashSet::from_iter(color::COLORS.iter()), &color_prefs);

        // Create game record.
        let game = create_game(opts.new_game, conn)
            .chain_err(|| "could not create new game")?;

        // Create a player record for each user.
        let mut players: Vec<GamePlayer> = vec![];
        for (pos, user) in users.iter().enumerate() {
            players.push(create_game_player(&NewGamePlayer {
                                                 game_id: game.id,
                                                 user_id: user.id,
                                                 position: pos as i32,
                                                 color: &player_colors[pos].to_string(),
                                                 has_accepted: user.id == *opts.creator_id,
                                                 is_turn: opts.whose_turn.contains(&pos),
                                                 is_eliminated: opts.eliminated.contains(&pos),
                                                 is_winner: opts.winners.contains(&pos),
                                                 is_read: false,
                                             },
                                            conn)
                                 .chain_err(|| "could not create game player")?);
        }
        Ok(CreatedGame {
               game: game,
               opponents: opponents,
               players: players,
           })
    })
}

pub struct UpdatedGame {
    pub game: Option<Game>,
    pub whose_turn: Vec<GamePlayer>,
    pub eliminated: Vec<GamePlayer>,
    pub winners: Vec<GamePlayer>,
}
pub fn update_game_and_players(game_id: &Uuid,
                               update: &NewGame,
                               whose_turn: &[usize],
                               eliminated: &[usize],
                               winners: &[usize],
                               conn: &PgConnection)
                               -> Result<UpdatedGame> {
    conn.transaction(|| {
        let result = UpdatedGame {
            game: update_game(game_id, update, conn)?,
            whose_turn: update_game_whose_turn(game_id, whose_turn, conn)?,
            eliminated: update_game_eliminated(game_id, eliminated, conn)?,
            winners: update_game_winners(game_id, winners, conn)?,
        };
        Ok(result)
    })
}

fn to_i32_vec(from: &[usize]) -> Vec<i32> {
    from.iter().map(|p| *p as i32).collect::<Vec<i32>>()
}

pub fn update_game(update_id: &Uuid,
                   update: &NewGame,
                   conn: &PgConnection)
                   -> Result<Option<Game>> {
    use db::schema::games;
    diesel::update(games::table.find(update_id))
        .set((games::game_version_id.eq(update.game_version_id),
              games::is_finished.eq(update.is_finished),
              games::game_state.eq(update.game_state)))
        .get_result(conn)
        .optional()
        .chain_err(|| "error updating game")
}

pub fn update_game_whose_turn(id: &Uuid,
                              positions: &[usize],
                              conn: &PgConnection)
                              -> Result<Vec<GamePlayer>> {
    use db::schema::game_players;

    diesel::update(game_players::table.filter(game_players::game_id.eq(id)))
        .set(game_players::is_turn.eq(game_players::position.eq_any(to_i32_vec(positions))))
        .get_results(conn)
        .chain_err(|| "error updating game players")
}

pub fn update_game_eliminated(id: &Uuid,
                              positions: &[usize],
                              conn: &PgConnection)
                              -> Result<Vec<GamePlayer>> {
    use db::schema::game_players;

    diesel::update(game_players::table.filter(game_players::game_id.eq(id)))
        .set(game_players::is_eliminated.eq(game_players::position.eq_any(to_i32_vec(positions))))
        .get_results(conn)
        .chain_err(|| "error updating game players")
}

pub fn update_game_winners(id: &Uuid,
                           positions: &[usize],
                           conn: &PgConnection)
                           -> Result<Vec<GamePlayer>> {
    use db::schema::game_players;

    diesel::update(game_players::table.filter(game_players::game_id.eq(id)))
        .set(game_players::is_winner.eq(game_players::position.eq_any(to_i32_vec(positions))))
        .get_results(conn)
        .chain_err(|| "error updating game players")
}

pub fn create_game_logs_from_cli(game_id: &Uuid,
                                 logs: Vec<CliLog>,
                                 conn: &PgConnection)
                                 -> Result<Vec<CreatedGameLog>> {
    conn.transaction(|| {
        let mut player_id_by_position: HashMap<usize, Uuid> = HashMap::new();
        for p in find_game_players_by_game(game_id, conn)? {
            player_id_by_position.insert(p.position as usize, p.id);
        }
        let mut created: Vec<CreatedGameLog> = vec![];
        for l in logs {
            let mut player_to: Vec<Uuid> = vec![];
            for t in l.to {
                player_to.push(player_id_by_position
                                   .get(&t)
                                   .ok_or_else::<Error, _>(|| {
                                                               "no player with that position exists"
                                                                   .into()
                                                           })?
                                   .to_owned());
            }
            created.push(create_game_log(&NewGameLog {
                                              game_id: *game_id,
                                              body: &l.content,
                                              is_public: l.public,
                                              logged_at: l.at,
                                          },
                                         &player_to,
                                         conn)?);
        }
        Ok(created)
    })
}

pub fn find_game_players_by_game(game_id: &Uuid, conn: &PgConnection) -> Result<Vec<GamePlayer>> {
    use db::schema::game_players;

    game_players::table
        .filter(game_players::game_id.eq(game_id))
        .order(game_players::position)
        .get_results(conn)
        .chain_err(|| "error finding players")
}

pub fn find_game_players_with_user_by_game(game_id: &Uuid,
                                           conn: &PgConnection)
                                           -> Result<Vec<(GamePlayer, User)>> {
    use db::schema::{game_players, users};

    game_players::table
        .filter(game_players::game_id.eq(game_id))
        .order(game_players::position)
        .inner_join(users::table)
        .get_results(conn)
        .chain_err(|| "error finding game players")
}

pub struct CreatedGameLog {
    pub game_log: GameLog,
    pub targets: Vec<GameLogTarget>,
}
pub fn create_game_log(log: &NewGameLog,
                       to: &[Uuid],
                       conn: &PgConnection)
                       -> Result<CreatedGameLog> {
    use db::schema::game_logs;
    conn.transaction(|| {
        let created_log: GameLog = diesel::insert(log)
            .into(game_logs::table)
            .get_result(conn)?;
        let clid = created_log.id;
        Ok(CreatedGameLog {
               game_log: created_log,
               targets: create_game_log_targets(&clid, to, conn)?,
           })
    })
}

pub fn create_game_log_targets(log_id: &Uuid,
                               player_ids: &[Uuid],
                               conn: &PgConnection)
                               -> Result<Vec<GameLogTarget>> {
    conn.transaction(|| {
        let mut created = vec![];
        for id in player_ids {
            created.push(create_game_log_target(&NewGameLogTarget {
                                                     game_log_id: *log_id,
                                                     game_player_id: *id,
                                                 },
                                                conn)?);
        }
        Ok(created)
    })
}

pub fn create_game_log_target(new_target: &NewGameLogTarget,
                              conn: &PgConnection)
                              -> Result<GameLogTarget> {
    use db::schema::game_log_targets;

    diesel::insert(new_target)
        .into(game_log_targets::table)
        .get_result(conn)
        .chain_err(|| "error inserting game log target")
}

pub fn create_game_users(ids: &[Uuid],
                         emails: &[String],
                         conn: &PgConnection)
                         -> Result<Vec<(UserEmail, User)>> {
    conn.transaction(|| {
        let mut users: Vec<(UserEmail, User)> = vec![];
        for id in ids.iter() {
            users.push(find_user_with_primary_email(id, conn)?
                           .ok_or_else::<Error, _>(|| "unable to find user".into())?);
        }
        for email in emails.iter() {
            users.push(match find_user_with_primary_email_by_email(email, conn)? {
                           Some(ube) => ube,
                           None => create_user_by_email(email, conn)?,
                       });
        }
        Ok(users)
    })
}

pub fn find_user_with_primary_email(find_user_id: &Uuid,
                                    conn: &PgConnection)
                                    -> Result<Option<(UserEmail, User)>> {
    use db::schema::{users, user_emails};

    user_emails::table
        .filter(user_emails::user_id.eq(find_user_id))
        .filter(user_emails::is_primary.eq(true))
        .inner_join(users::table)
        .first(conn)
        .optional()
        .chain_err(|| "error finding user")
}

pub fn find_user_with_primary_email_by_email(search_email: &str,
                                             conn: &PgConnection)
                                             -> Result<Option<(UserEmail, User)>> {
    use db::schema::{users, user_emails};

    Ok(match user_emails::table
                 .filter(user_emails::email.eq(search_email))
                 .first::<UserEmail>(conn)
                 .optional()? {
           Some(ue) => {
               Some(user_emails::table
                        .filter(user_emails::user_id.eq(ue.user_id))
                        .filter(user_emails::is_primary.eq(true))
                        .inner_join(users::table)
                        .first(conn)?)
           }
           None => return Ok(None),
       })
}

pub fn create_game(new_game: &NewGame, conn: &PgConnection) -> Result<Game> {
    use db::schema::games;

    diesel::insert(new_game)
        .into(games::table)
        .get_result(conn)
        .chain_err(|| "error inserting game")
}

pub fn create_game_version(new_game_version: &NewGameVersion,
                           conn: &PgConnection)
                           -> Result<GameVersion> {
    use db::schema::game_versions;

    diesel::insert(new_game_version)
        .into(game_versions::table)
        .get_result(conn)
        .chain_err(|| "error inserting game version")
}

pub fn create_game_type(new_game_type: &NewGameType, conn: &PgConnection) -> Result<GameType> {
    use db::schema::game_types;

    diesel::insert(new_game_type)
        .into(game_types::table)
        .get_result(conn)
        .chain_err(|| "error inserting game type")
}

pub fn create_game_players(players: &[NewGamePlayer],
                           conn: &PgConnection)
                           -> Result<Vec<GamePlayer>> {
    conn.transaction(|| {
                         let mut created: Vec<GamePlayer> = vec![];
                         for p in players.iter() {
                             created.push(create_game_player(p, conn)?);
                         }
                         Ok(created)
                     })
}

pub fn create_game_player(player: &NewGamePlayer, conn: &PgConnection) -> Result<GamePlayer> {
    use db::schema::game_players;

    diesel::insert(player)
        .into(game_players::table)
        .get_result(conn)
        .chain_err(|| "error inserting game player")
}

pub fn public_game_versions(conn: &PgConnection) -> Result<Vec<(GameVersion, GameType)>> {
    use db::schema::{game_versions, game_types};

    game_versions::table
        .filter(game_versions::is_public.eq(true))
        .filter(game_versions::is_deprecated.eq(false))
        .inner_join(game_types::table)
        .get_results(conn)
        .chain_err(|| "error finding game versions")
}

pub fn find_public_game_logs_for_game(game_id: &Uuid, conn: &PgConnection) -> Result<Vec<GameLog>> {
    use db::schema::game_logs;

    game_logs::table
        .filter(game_logs::game_id.eq(game_id))
        .filter(game_logs::is_public.eq(true))
        .order(game_logs::logged_at)
        .get_results(conn)
        .chain_err(|| "error finding game logs")
}

pub fn find_game_logs_for_player(game_player_id: &Uuid,
                                 conn: &PgConnection)
                                 -> Result<Vec<GameLog>> {
    use db::schema::{game_logs, game_log_targets, game_players};

    let game_player: GamePlayer = game_players::table
        .find(game_player_id)
        .get_result(conn)?;
    Ok(game_logs::table
           .left_outer_join(game_log_targets::table)
           .filter(game_logs::game_id.eq(game_player.game_id))
           .filter(game_logs::is_public
                       .eq(true)
                       .or(game_log_targets::game_player_id.eq(game_player_id)))
           .order(game_logs::logged_at)
           .get_results::<(GameLog, Option<GameLogTarget>)>(conn)
           .chain_err(|| "error finding game logs")?
           .iter()
           .map(|&(ref gl, _)| gl.clone())
           .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use db::color::Color;
    use db::models::NewUserEmail;
    use db::Connections;
    use db::connect_env;

    lazy_static! {
        static ref CONN: Connections = connect_env().unwrap();
    }

    #[test]
    fn rand_code_works() {
        for _ in 1..100000 {
            let n: usize = rand_code().parse().unwrap();
            assert!(n > 99999, "n <= 99999");
            assert!(n < 1000000, "n >= 1000000");
        }
    }

    fn with_db<F>(closure: F)
        where F: Fn(&PgConnection)
    {
        let conn = &CONN.w.get().unwrap();
        conn.test_transaction::<_, Error, _>(|| {
                                                 closure(conn);
                                                 Ok(())
                                             });
    }

    #[test]
    #[ignore]
    fn create_user_by_name_works() {
        with_db(|conn| { create_user_by_name("beefsack", conn).unwrap(); });
    }

    #[test]
    #[ignore]
    fn find_user_works() {
        with_db(|conn| {
                    assert_eq!(find_user(&Uuid::new_v4(), conn).unwrap(), None);
                    let u = create_user_by_name("beefsack", conn).unwrap();
                    assert!(find_user(&u.id, conn).unwrap().is_some());
                });
    }

    #[test]
    #[ignore]
    fn create_user_email_works() {
        with_db(|conn| {
            assert_eq!(find_user(&Uuid::new_v4(), conn).unwrap(), None);
            let u = create_user_by_name("beefsack", conn).unwrap();
            assert!(create_user_email(&NewUserEmail {
                                           user_id: u.id,
                                           email: "beefsack@gmail.com",
                                           is_primary: true,
                                       },
                                      conn)
                            .is_ok());
        });
    }

    #[test]
    #[ignore]
    fn login_works() {
        with_db(|conn| {
            let confirmation = user_login_request("beefsack@gmail.com", conn).unwrap();
            let uat = user_login_confirm("beefsack@gmail.com", &confirmation, conn)
                .expect("error confirming auth")
                .expect("invalid confirm code");
            assert!(authenticate("beefsack@gmail.com", &uat.id, conn)
                        .unwrap()
                        .is_some());
            assert!(authenticate("beefsacke@gmail.com", &uat.id, conn)
                        .unwrap()
                        .is_none());
        });
    }

    #[test]
    #[ignore]
    fn find_user_with_primary_email_works() {
        with_db(|conn| {
            let (user_email, user) = create_user_by_email("beefsack@gmail.com", conn).unwrap();
            let (_, found_user) = find_user_with_primary_email(&user.id, conn)
                .unwrap()
                .unwrap();
            assert_eq!(user.id, found_user.id);
            assert_eq!("beefsack@gmail.com", user_email.email);
        });
    }

    #[test]
    #[ignore]
    fn find_user_with_primary_email_by_email_works() {
        with_db(|conn| {
            let (user_email, user) = create_user_by_email("beefsack@gmail.com", conn).unwrap();
            create_user_email(&NewUserEmail {
                                   user_id: user.id,
                                   email: "beefsack+two@gmail.com",
                                   is_primary: false,
                               },
                              conn)
                    .expect("error creating user email");
            let (_, found_user) = find_user_with_primary_email_by_email("beefsack+two@gmail.com",
                                                                        conn)
                    .expect("error finding user")
                    .expect("user doesn't exist");
            assert_eq!(user.id, found_user.id);
            assert_eq!("beefsack@gmail.com", user_email.email);
        });
    }

    #[test]
    #[ignore]
    fn create_game_works() {
        with_db(|conn| {
            let game_type = create_game_type(&NewGameType { name: "Lost Cities" }, conn).unwrap();
            let game_version = create_game_version(&NewGameVersion {
                                                        game_type_id: game_type.id,
                                                        uri: "https://example.com/lost-cities-1",
                                                        name: "v1",
                                                        is_public: true,
                                                        is_deprecated: false,
                                                    },
                                                   conn)
                    .unwrap();
            assert!(create_game(&NewGame {
                                     game_version_id: game_version.id,
                                     is_finished: false,
                                     game_state: "blah",
                                 },
                                conn)
                            .is_ok());
        });
    }

    #[test]
    #[ignore]
    fn create_players_works() {
        with_db(|conn| {
            let (_, p1) = create_user_by_email("beefsack@gmail.com", conn).unwrap();
            let (_, p2) = create_user_by_email("beefsack+two@gmail.com", conn).unwrap();
            let game_type = create_game_type(&NewGameType { name: "Lost Cities" }, conn).unwrap();
            let game_version = create_game_version(&NewGameVersion {
                                                        game_type_id: game_type.id,
                                                        uri: "https://example.com/lost-cities-1",
                                                        name: "v1",
                                                        is_public: true,
                                                        is_deprecated: false,
                                                    },
                                                   conn)
                    .unwrap();
            let game = create_game(&NewGame {
                                        game_version_id: game_version.id,
                                        is_finished: false,
                                        game_state: "egg",
                                    },
                                   conn)
                    .unwrap();
            create_game_players(&[NewGamePlayer {
                                      game_id: game.id,
                                      user_id: p1.id,
                                      position: 0,
                                      color: &Color::Green.to_string(),
                                      has_accepted: true,
                                      is_turn: false,
                                      is_eliminated: false,
                                      is_winner: false,
                                      is_read: false,
                                  },
                                  NewGamePlayer {
                                      game_id: game.id,
                                      user_id: p2.id,
                                      position: 1,
                                      color: &Color::Red.to_string(),
                                      has_accepted: false,
                                      is_turn: true,
                                      is_eliminated: false,
                                      is_winner: false,
                                      is_read: false,
                                  }],
                                conn)
                    .unwrap();
        });
    }
}
