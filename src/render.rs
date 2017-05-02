use brdgme_markup as markup;

use std::str::FromStr;

use errors::*;
use db::models::{GamePlayer, User, PublicGamePlayerUser};
use db::color;

pub fn public_game_players_to_markup_players(game_players: &[PublicGamePlayerUser])
                                             -> Vec<markup::Player> {
    game_players
        .iter()
        .map(|gpu| {
                 markup::Player {
                     color: color::Color::from_str(&gpu.game_player.color)
                         .unwrap()
                         .into(),
                     name: gpu.user.name.to_owned(),
                 }
             })
        .collect()
}

pub fn game_players_to_markup_players(game_players: &[(GamePlayer, User)]) -> Vec<markup::Player> {
    game_players
        .iter()
        .map(|&(ref gp, ref u)| {
                 markup::Player {
                     color: color::Color::from_str(&gp.color).unwrap().into(),
                     name: u.name.to_owned(),
                 }
             })
        .collect()
}

pub fn markup_html(template: &str, players: &[markup::Player]) -> Result<String> {
    Ok(markup::html(&markup::transform(&markup::from_string(template)
                                            .chain_err(|| "failed to parse template")?
                                            .0,
                                       players)))
}
