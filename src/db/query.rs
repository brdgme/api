use postgres::GenericConnection;
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
}

pub struct UserByEmail {
    pub user: User,
    pub user_email: UserEmail,
}

pub fn create_user_by_name(name: &str, conn: &GenericConnection) -> Result<User> {
    for row in &conn.query("
        INSERT INTO users
        (
            name
        ) VALUES (
            $1
        )
        RETURNING *",
                           &[&name])
                    .chain_err(|| "error creating user")? {
        return Ok(User::from_row(&row, ""));
    }
    Err("unable to create user".into())
}

pub fn find_user(id: &Uuid, conn: &GenericConnection) -> Result<Option<User>> {
    for row in &conn.query("
        SELECT *
        FROM users
        WHERE id=$1
        LIMIT 1
    ",
                           &[id])? {
        return Ok(Some(User::from_row(&row, "")));
    }
    Ok(None)
}

pub fn find_user_by_email(email: &str, conn: &GenericConnection) -> Result<Option<UserByEmail>> {
    for row in &conn.query(&format!("
        SELECT
            {}, {}
        FROM user_emails ue
        INNER JOIN users u
        ON (ue.user_id = u.id)
        WHERE ue.email = $1
        LIMIT 1",
                                    User::select_cols("u", "u_"),
                                    UserEmail::select_cols("ue", "ue_")),
                           &[&email])? {
        return Ok(Some(UserByEmail {
                           user: User::from_row(&row, "u_"),
                           user_email: UserEmail::from_row(&row, "ue_"),
                       }));
    }
    Ok(None)
}

pub fn find_or_create_user_by_email(email: &str, conn: &GenericConnection) -> Result<UserByEmail> {
    if let Some(u) = find_user_by_email(email, conn)? {
        return Ok(u);
    }
    create_user_by_email(email, conn)
}

pub fn create_user_by_email(email: &str, conn: &GenericConnection) -> Result<UserByEmail> {
    let trans = conn.transaction()?;
    let u = create_user_by_name(email, &trans)?;
    let ue = create_user_email(&NewUserEmail {
                                    user_id: &u.id,
                                    email: email,
                                    is_primary: true,
                                },
                               &trans)?;
    trans.commit()?;
    Ok(UserByEmail {
           user: u,
           user_email: ue,
       })
}

pub fn create_user_email(ue: &NewUserEmail, conn: &GenericConnection) -> Result<UserEmail> {
    for row in &conn.query("
        INSERT INTO user_emails
        (
            email,
            user_id,
            is_primary
        ) VALUES (
            $1,
            $2,
            $3
        ) RETURNING *",
                           &[&ue.email, &ue.user_id, &ue.is_primary])? {
        return Ok(UserEmail::from_row(&row, ""));
    }
    Err("could not create user email".into())
}

fn rand_code() -> String {
    let mut rng = rand::thread_rng();
    format!("{}{:05}",
            (rng.gen::<usize>() % 9) + 1,
            rng.gen::<usize>() % 100000)
}

pub fn generate_user_login_confirmation(user_id: &Uuid,
                                        conn: &GenericConnection)
                                        -> Result<String> {
    let code = rand_code();
    match conn.execute("
        UPDATE users
        SET
            login_confirmation=$1,
            login_confirmation_at=(now() AT TIME ZONE 'utc')
        WHERE id=$2
    ",
                       &[&Some(&code), user_id])? {
        0 => Err("could not update login confirmation".into()),
        _ => Ok(code),
    }
}

pub fn user_login_request(email: &str, conn: &GenericConnection) -> Result<String> {
    let trans = conn.transaction()?;
    let user = find_or_create_user_by_email(email, &trans)?.user;

    let confirmation = match (user.login_confirmation, user.login_confirmation_at) {
        (Some(ref uc), Some(at)) if at + *CONFIRMATION_EXPIRY > UTC::now().naive_utc() => {
            uc.to_owned()
        }
        _ => generate_user_login_confirmation(&user.id, &trans)?,
    };
    trans.commit()?;
    Ok(confirmation)
}

pub fn user_login_confirm(email: &str,
                          confirmation: &str,
                          conn: &GenericConnection)
                          -> Result<Option<UserAuthToken>> {
    let user = match find_user_by_email(email, conn)? {
        Some(ube) => ube.user,
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

pub fn create_auth_token(user_id: &Uuid, conn: &GenericConnection) -> Result<UserAuthToken> {
    for row in &conn.query("
        INSERT INTO user_auth_tokens
        (
            user_id
        ) VALUES (
            $1
        ) RETURNING *",
                           &[user_id])? {
        return Ok(UserAuthToken::from_row(&row, ""));
    }
    Err("could not create user auth token".into())
}

pub fn authenticate(email: &str,
                    token: &Uuid,
                    conn: &GenericConnection)
                    -> Result<Option<UserByEmail>> {
    for row in &conn.query(&format!("
        SELECT
            {}, {}, {}
        FROM users u
        INNER JOIN user_auth_tokens uat
        ON (uat.user_id = u.id)
        INNER JOIN user_emails ue
        ON (ue.user_id = u.id)
        WHERE ue.email = $1
        AND uat.id = $2
        AND uat.created_at > $3
        LIMIT 1",
                                    User::select_cols("u", "u_"),
                                    UserEmail::select_cols("ue", "ue_"),
                                    UserAuthToken::select_cols("uat", "uat_"),
                                    ),
                           &[&email, token, &(UTC::now().naive_utc() - *TOKEN_EXPIRY)])? {
        return Ok(Some(UserByEmail {
                           user: User::from_row(&row, "u_"),
                           user_email: UserEmail::from_row(&row, "ue_"),
                       }));
    }
    Ok(None)
}

pub fn find_game_version(id: &Uuid, conn: &GenericConnection) -> Result<Option<GameVersion>> {
    for row in &conn.query("
        SELECT *
        FROM game_versions
        WHERE id=$1
        LIMIT 1
    ",
                           &[id])? {
        return Ok(Some(GameVersion::from_row(&row, "")));
    }
    Ok(None)
}

pub fn find_game_with_version(id: &Uuid,
                              conn: &GenericConnection)
                              -> Result<Option<(Game, GameVersion)>> {
    for row in &conn.query(&format!("
        SELECT {}, {}
        FROM games g
        INNER JOIN game_versions gv
        ON (g.game_version_id = gv.id)
        WHERE g.id=$1
        LIMIT 1
    ",
                                    Game::select_cols("g", "g_"),
                                    GameVersion::select_cols("gv", "gv_")),
                           &[id])? {
        return Ok(Some((Game::from_row(&row, "g_"), GameVersion::from_row(&row, "gv_"))));
    }
    Ok(None)
}

pub struct CreatedGame {
    pub game: Game,
    pub opponents: Vec<UserByEmail>,
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
pub fn create_game_with_users(opts: &CreateGameOpts,
                              conn: &GenericConnection)
                              -> Result<CreatedGame> {
    let trans = conn.transaction()?;
    // Find or create users.
    let creator = find_user(opts.creator_id, &trans)
        .chain_err(|| "could not find creator")?
        .ok_or_else::<Error, _>(|| "could not find creator".into())?;
    let opponents = create_game_users(opts.opponent_ids, opts.opponent_emails, &trans)
        .chain_err(|| "could not create game users")?;
    let mut users: Vec<User> = opponents.iter().map(|o| o.user.clone()).collect();
    users.push(creator);

    // Randomise the users so player order is random.
    let mut rnd = rand::thread_rng();
    rnd.shuffle(&mut users);

    // Assign colors to each player using preferences.
    let color_prefs: Vec<Vec<Color>> = users.iter().map(|u| u.pref_colors.clone()).collect();
    let player_colors = color::choose(&HashSet::from_iter(color::COLORS.iter()), &color_prefs);

    // Create game record.
    let game = create_game(opts.new_game, &trans)
        .chain_err(|| "could not create new game")?;

    // Create a player record for each user.
    let mut players: Vec<GamePlayer> = vec![];
    for (pos, user) in users.iter().enumerate() {
        players.push(create_game_player(&NewGamePlayer {
                                             game_id: &game.id,
                                             user_id: &user.id,
                                             position: pos as i32,
                                             color: &player_colors[pos],
                                             has_accepted: user.id == *opts.creator_id,
                                             is_turn: opts.whose_turn.contains(&pos),
                                             is_eliminated: opts.eliminated.contains(&pos),
                                             is_winner: opts.winners.contains(&pos),
                                         },
                                        &trans)
                             .chain_err(|| "could not create game player")?);
    }
    trans.commit()?;
    Ok(CreatedGame {
           game: game,
           opponents: opponents,
           players: players,
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
                               conn: &GenericConnection)
                               -> Result<UpdatedGame> {
    let trans = conn.transaction()?;
    let result = UpdatedGame {
        game: update_game(game_id, update, &trans)?,
        whose_turn: update_game_whose_turn(game_id, whose_turn, &trans)?,
        eliminated: update_game_eliminated(game_id, eliminated, &trans)?,
        winners: update_game_winners(game_id, winners, &trans)?,
    };
    trans.commit()?;
    Ok(result)
}

pub fn update_game(id: &Uuid, update: &NewGame, conn: &GenericConnection) -> Result<Option<Game>> {
    for row in &conn.query("
        UPDATE games
        SET
            game_version_id=$1,
            is_finished=$2,
            game_state=$3
        WHERE id=$4
        RETURNING *",
                           &[&update.game_version_id,
                             &update.is_finished,
                             &update.game_state,
                             &id])? {
        return Ok(Some(Game::from_row(&row, "")));
    }
    Ok(None)
}

fn position_update_clause(positions: &[usize]) -> String {
    if positions.is_empty() {
        "FALSE".to_string()
    } else {
        format!("(position IN ({}))",
                positions
                    .iter()
                    .map(|p| p.to_string())
                    .collect::<Vec<String>>()
                    .join(","))
    }
}

pub fn update_game_whose_turn(id: &Uuid,
                              positions: &[usize],
                              conn: &GenericConnection)
                              -> Result<Vec<GamePlayer>> {
    let mut players: Vec<GamePlayer> = vec![];
    for row in &conn.query(&format!("
        UPDATE game_players
        SET is_turn={}
        WHERE game_id=$1
        RETURNING *",
                                    position_update_clause(positions)),
                           &[&id])? {
        players.push(GamePlayer::from_row(&row, ""));
    }
    Ok(players)
}

pub fn update_game_eliminated(id: &Uuid,
                              positions: &[usize],
                              conn: &GenericConnection)
                              -> Result<Vec<GamePlayer>> {
    let mut players: Vec<GamePlayer> = vec![];
    for row in &conn.query(&format!("
        UPDATE game_players
        SET is_eliminated={}
        WHERE game_id=$1
        RETURNING *",
                                    position_update_clause(positions)),
                           &[&id])? {
        players.push(GamePlayer::from_row(&row, ""));
    }
    Ok(players)
}

pub fn update_game_winners(id: &Uuid,
                           positions: &[usize],
                           conn: &GenericConnection)
                           -> Result<Vec<GamePlayer>> {
    let mut players: Vec<GamePlayer> = vec![];
    for row in &conn.query(&format!("
        UPDATE game_players
        SET is_winner={}
        WHERE game_id=$1
        RETURNING *",
                                    position_update_clause(positions)),
                           &[&id])? {
        players.push(GamePlayer::from_row(&row, ""));
    }
    Ok(players)
}

pub fn create_game_logs_from_cli(game_id: &Uuid,
                                 logs: Vec<CliLog>,
                                 conn: &GenericConnection)
                                 -> Result<Vec<CreatedGameLog>> {
    let mut player_id_by_position: HashMap<usize, Uuid> = HashMap::new();
    let trans = conn.transaction()?;
    for p in find_game_players_by_game(game_id, &trans)? {
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
                                          game_id: game_id,
                                          body: &l.content,
                                          is_public: l.public,
                                          logged_at: &l.at,
                                      },
                                     &player_to,
                                     &trans)?);
    }
    trans.commit()?;
    Ok(created)
}

pub fn find_game_players_by_game(game_id: &Uuid,
                                 conn: &GenericConnection)
                                 -> Result<Vec<GamePlayer>> {
    let mut players: Vec<GamePlayer> = vec![];
    for row in &conn.query("
        SELECT *
        FROM game_players
        WHERE game_id=$1
        ORDER BY position",
                           &[game_id])? {
        players.push(GamePlayer::from_row(&row, ""));
    }
    Ok(players)
}

pub struct GamePlayerUser {
    pub game_player: GamePlayer,
    pub user: User,
}
pub fn find_game_players_with_user_by_game(game_id: &Uuid,
                                           conn: &GenericConnection)
                                           -> Result<Vec<GamePlayerUser>> {
    let mut players: Vec<GamePlayerUser> = vec![];
    for row in &conn.query(&format!("
        SELECT {}, {}
        FROM game_players gp
        INNER JOIN users u
        ON (gp.user_id = u.id)
        WHERE gp.game_id=$1
        ORDER BY gp.position",
                                    GamePlayer::select_cols("gp", "gp_"),
                                    User::select_cols("u", "u_")),
                           &[game_id])? {
        players.push(GamePlayerUser {
                         game_player: GamePlayer::from_row(&row, "gp_"),
                         user: User::from_row(&row, "u_"),
                     });
    }
    Ok(players)
}

pub struct CreatedGameLog {
    pub game_log: GameLog,
    pub targets: Vec<GameLogTarget>,
}
pub fn create_game_log(log: &NewGameLog,
                       to: &[Uuid],
                       conn: &GenericConnection)
                       -> Result<CreatedGameLog> {
    let trans = conn.transaction()?;
    let mut created_log: Option<GameLog> = None;
    for row in &trans
                    .query("
        INSERT INTO game_logs (
            game_id,
            body,
            is_public,
            logged_at
        ) VALUES (
            $1,
            $2,
            $3,
            $4
        )
        RETURNING *",
                           &[&log.game_id, &log.body, &log.is_public, &log.logged_at])? {
        created_log = Some(GameLog::from_row(&row, ""));
    }
    let gl = created_log
        .ok_or_else::<Error, _>(|| "error creating game log".into())?;
    let targets = create_game_log_targets(&gl.id, to, &trans)?;
    trans.commit()?;
    Ok(CreatedGameLog {
           game_log: gl,
           targets: targets,
       })
}

pub fn create_game_log_targets(log_id: &Uuid,
                               player_ids: &[Uuid],
                               conn: &GenericConnection)
                               -> Result<Vec<GameLogTarget>> {
    let trans = conn.transaction()?;
    let mut created = vec![];
    for id in player_ids {
        created.push(create_game_log_target(&NewGameLogTarget {
                                                 game_log_id: log_id,
                                                 player_id: id,
                                             },
                                            &trans)?);
    }
    trans.commit()?;
    Ok(created)
}

pub fn create_game_log_target(new_target: &NewGameLogTarget,
                              conn: &GenericConnection)
                              -> Result<GameLogTarget> {
    for row in &conn.query("
        INSERT INTO game_log_targets (
            game_log_id,
            player_id
        ) VALUES (
            $1,
            $2
        )
        RETURNING *",
                           &[&new_target.game_log_id, &new_target.player_id])? {
        return Ok(GameLogTarget::from_row(&row, ""));
    }
    Err("error creating game log target".into())

}

pub fn create_game_users(ids: &[Uuid],
                         emails: &[String],
                         conn: &GenericConnection)
                         -> Result<Vec<UserByEmail>> {
    let trans = conn.transaction()?;
    let mut users: Vec<UserByEmail> = vec![];
    for id in ids.iter() {
        users.push(find_user_with_primary_email(id, &trans)?
                       .ok_or_else::<Error, _>(|| "unable to find user".into())?);
    }
    for email in emails.iter() {
        users.push(match find_user_with_primary_email_by_email(email, &trans)? {
                       Some(ube) => ube,
                       None => create_user_by_email(email, &trans)?,
                   });
    }
    trans.commit()?;
    Ok(users)
}

pub fn find_user_with_primary_email(id: &Uuid,
                                    conn: &GenericConnection)
                                    -> Result<Option<UserByEmail>> {
    for row in &conn.query(&format!("
        SELECT {}, {}
        FROM users u
        INNER JOIN user_emails ue
        ON (u.id = ue.user_id)
        WHERE u.id = $1
        AND ue.is_primary = TRUE
        LIMIT 1",
                                    User::select_cols("u", "u_"),
                                    UserEmail::select_cols("ue", "ue_")),
                           &[id])? {
        return Ok(Some(UserByEmail {
                           user: User::from_row(&row, "u_"),
                           user_email: UserEmail::from_row(&row, "ue_"),
                       }));
    }
    Ok(None)
}

pub fn find_user_with_primary_email_by_email(email: &str,
                                             conn: &GenericConnection)
                                             -> Result<Option<UserByEmail>> {
    for row in &conn.query(&format!("
        SELECT {}, {}
        FROM users u
        INNER JOIN user_emails ue
        ON (u.id = ue.user_id)
        INNER JOIN user_emails uef
        ON (u.id = uef.user_id)
        WHERE uef.email = $1
        AND ue.is_primary = TRUE
        LIMIT 1",
                                    User::select_cols("u", "u_"),
                                    UserEmail::select_cols("ue", "ue_")),
                           &[&email])? {
        return Ok(Some(UserByEmail {
                           user: User::from_row(&row, "u_"),
                           user_email: UserEmail::from_row(&row, "ue_"),
                       }));
    }
    Ok(None)
}

pub fn create_game(new_game: &NewGame, conn: &GenericConnection) -> Result<Game> {
    for row in &conn.query("
        INSERT INTO games (
            game_version_id,
            is_finished,
            game_state
        ) VALUES (
            $1,
            $2,
            $3
        )
        RETURNING *",
                           &[&new_game.game_version_id,
                             &new_game.is_finished,
                             &new_game.game_state])? {
        return Ok(Game::from_row(&row, ""));
    }
    Err("error creating game".into())
}

pub fn create_game_version(new_game_version: &NewGameVersion,
                           conn: &GenericConnection)
                           -> Result<GameVersion> {
    for row in &conn.query("
        INSERT INTO game_versions (
            game_type_id,
            uri,
            name,
            is_public,
            is_deprecated
        ) VALUES (
            $1,
            $2,
            $3,
            $4,
            $5
        )
        RETURNING *",
                           &[&new_game_version.game_type_id,
                             &new_game_version.uri,
                             &new_game_version.name,
                             &new_game_version.is_public,
                             &new_game_version.is_deprecated])? {
        return Ok(GameVersion::from_row(&row, ""));
    }
    Err("error creating game version".into())
}

pub fn create_game_type(new_game_type: &NewGameType, conn: &GenericConnection) -> Result<GameType> {
    for row in &conn.query("
        INSERT INTO game_types (
            name
        ) VALUES (
            $1
        )
        RETURNING *",
                           &[&new_game_type.name])? {
        return Ok(GameType::from_row(&row, ""));
    }
    Err("error creating game type".into())
}

pub fn create_game_players(players: &[NewGamePlayer],
                           conn: &GenericConnection)
                           -> Result<Vec<GamePlayer>> {
    let trans = conn.transaction()?;
    let mut created: Vec<GamePlayer> = vec![];
    for p in players.iter() {
        created.push(create_game_player(p, &trans)?);
    }
    trans.commit()?;
    Ok(created)
}

pub fn create_game_player(player: &NewGamePlayer, conn: &GenericConnection) -> Result<GamePlayer> {
    for row in &conn.query("
        INSERT INTO game_players (
            game_id,
            user_id,
            position,
            color,
            has_accepted,
            is_turn,
            is_eliminated,
            is_winner
        ) VALUES (
            $1,
            $2,
            $3,
            $4,
            $5,
            $6,
            $7,
            $8
        )
        RETURNING *",
                           &[&player.game_id,
                             &player.user_id,
                             &player.position,
                             &player.color,
                             &player.has_accepted,
                             &player.is_turn,
                             &player.is_eliminated,
                             &player.is_winner])? {
        return Ok(GamePlayer::from_row(&row, ""));
    }
    Err("error creating game type".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use db::color::Color;
    use db::models::NewUserEmail;
    use postgres::GenericConnection;
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
        where F: Fn(&GenericConnection)
    {
        let ref conn = *CONN.w.get().unwrap();
        closure(&conn.transaction().unwrap());
    }

    #[test]
    #[ignore]
    fn create_user_by_name_works() {
        with_db(|conn| {
                    assert!(create_user_by_name("beefsack", conn).is_ok());
                });
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
                                           user_id: &u.id,
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
                .unwrap()
                .unwrap();
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
                    let ube = create_user_by_email("beefsack@gmail.com", conn).unwrap();
                    let found = find_user_with_primary_email(&ube.user.id, conn)
                        .unwrap()
                        .unwrap();
                    assert_eq!(ube.user.id, found.user.id);
                    assert_eq!("beefsack@gmail.com", ube.user_email.email);
                });
    }

    #[test]
    #[ignore]
    fn find_user_with_primary_email_by_email_works() {
        with_db(|conn| {
            let ube = create_user_by_email("beefsack@gmail.com", conn).unwrap();
            create_user_email(&NewUserEmail {
                                   user_id: &ube.user.id,
                                   email: "beefsack+two@gmail.com",
                                   is_primary: false,
                               },
                              conn)
                    .unwrap();
            let found = find_user_with_primary_email_by_email("beefsack+two@gmail.com", conn)
                .unwrap()
                .unwrap();
            assert_eq!(ube.user.id, found.user.id);
            assert_eq!("beefsack@gmail.com", ube.user_email.email);
        });
    }

    #[test]
    #[ignore]
    fn create_game_works() {
        with_db(|conn| {
            let game_type = create_game_type(&NewGameType { name: "Lost Cities" }, conn).unwrap();
            let game_version = create_game_version(&NewGameVersion {
                                                        game_type_id: &game_type.id,
                                                        uri: "https://example.com/lost-cities-1",
                                                        name: "v1",
                                                        is_public: true,
                                                        is_deprecated: false,
                                                    },
                                                   conn)
                    .unwrap();
            assert!(create_game(&NewGame {
                                     game_version_id: &game_version.id,
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
            let p1 = create_user_by_email("beefsack@gmail.com", conn).unwrap();
            let p2 = create_user_by_email("beefsack+two@gmail.com", conn).unwrap();
            let game_type = create_game_type(&NewGameType { name: "Lost Cities" }, conn).unwrap();
            let game_version = create_game_version(&NewGameVersion {
                                                        game_type_id: &game_type.id,
                                                        uri: "https://example.com/lost-cities-1",
                                                        name: "v1",
                                                        is_public: true,
                                                        is_deprecated: false,
                                                    },
                                                   conn)
                    .unwrap();
            let game = create_game(&NewGame {
                                        game_version_id: &game_version.id,
                                        is_finished: false,
                                        game_state: "egg",
                                    },
                                   conn)
                    .unwrap();
            create_game_players(&[NewGamePlayer {
                                      game_id: &game.id,
                                      user_id: &p1.user.id,
                                      position: 0,
                                      color: &Color::Green,
                                      has_accepted: true,
                                      is_turn: false,
                                      is_eliminated: false,
                                      is_winner: false,
                                  },
                                  NewGamePlayer {
                                      game_id: &game.id,
                                      user_id: &p2.user.id,
                                      position: 1,
                                      color: &Color::Red,
                                      has_accepted: false,
                                      is_turn: true,
                                      is_eliminated: false,
                                      is_winner: false,
                                  }],
                                conn)
                    .unwrap();
        });
    }
}
