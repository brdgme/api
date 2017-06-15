use brdgme_markup as markup;

use std::str::FromStr;

use errors::*;
use db::models::{GamePlayerTypeUser, PublicGamePlayerTypeUser};
use db::color;

pub fn public_game_players_to_markup_players(
    game_players: &[PublicGamePlayerTypeUser],
) -> Result<Vec<markup::Player>> {
    game_players
        .iter()
        .map(|gpu| {
            Ok(markup::Player {
                color: color::Color::from_str(&gpu.game_player.color)?.into(),
                name: gpu.user.name.to_owned(),
            })
        })
        .collect()
}

pub fn game_players_to_markup_players(
    game_players: &[GamePlayerTypeUser],
) -> Result<Vec<markup::Player>> {
    public_game_players_to_markup_players(&game_players
        .iter()
        .map(|gptu| gptu.to_owned().into_public())
        .collect::<Vec<PublicGamePlayerTypeUser>>())
}

pub fn markup_html(template: &str, players: &[markup::Player]) -> Result<String> {
    Ok(markup::html(&markup::transform(
        &markup::from_string(template)
            .chain_err(|| "failed to parse template")?
            .0,
        players,
    )))
}
