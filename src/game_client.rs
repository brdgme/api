use hyper::{self, Client as HttpClient};
use hyper::net::HttpsConnector;
use hyper_native_tls::NativeTlsClient;
use serde_json;

use brdgme_cmd::cli;
use brdgme_markup as markup;

use std::str::FromStr;

use errors::*;
use db::models::{GamePlayer, User};
use db::color;

pub fn request(uri: &str, request: &cli::Request) -> Result<cli::Response> {
    let ssl = NativeTlsClient::new()
        .chain_err(|| "unable to get native TLS client")?;
    let connector = HttpsConnector::new(ssl);
    let https = HttpClient::with_connector(connector);
    let res = https
        .post(uri)
        .body(&serde_json::to_string(request)
                   .chain_err(|| "error converting request to JSON")?)
        .send()
        .chain_err(|| "error getting new game state")?;
    if res.status != hyper::Ok {
        bail!("game request failed");
    }
    match serde_json::from_reader::<_, cli::Response>(res)
              .chain_err(|| "error parsing JSON response")? {
        cli::Response::UserError { message } => Err(ErrorKind::UserError(message).into()),
        cli::Response::SystemError { message } => Err(message.into()),
        default => Ok(default),
    }
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