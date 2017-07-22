use hyper::{self, Client as HttpClient};
use hyper::net::HttpsConnector;
use hyper_rustls::TlsClient;
use serde_json;

use brdgme_cmd::cli;
use brdgme_game::command::Spec as CommandSpec;

use errors::*;

pub fn request(uri: &str, request: &cli::Request) -> Result<cli::Response> {
    let connector = HttpsConnector::new(TlsClient::new());
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

#[derive(Debug, Clone)]
pub struct RenderResponse {
    pub render: String,
    pub state: String,
    pub command_spec: Option<CommandSpec>,
}

impl From<cli::PubRender> for RenderResponse {
    fn from(render: cli::PubRender) -> Self {
        Self {
            render: render.render,
            state: render.pub_state,
            command_spec: None,
        }
    }
}

impl From<cli::PlayerRender> for RenderResponse {
    fn from(render: cli::PlayerRender) -> Self {
        Self {
            render: render.render,
            state: render.player_state,
            command_spec: render.command_spec,
        }
    }
}

pub fn render(uri: &str, game: String, player: Option<usize>) -> Result<RenderResponse> {
    match player {
        Some(p) => player_render(uri, game, p),
        None => pub_render(uri, game),
    }
}

pub fn pub_render(uri: &str, game: String) -> Result<RenderResponse> {
    request(uri, &cli::Request::PubRender { game }).and_then(|resp| match resp {
        cli::Response::PubRender { render } => Ok(render.into()),
        _ => Err(ErrorKind::Msg("invalid response type".to_string()).into()),
    })
}

pub fn player_render(uri: &str, game: String, player: usize) -> Result<RenderResponse> {
    request(uri, &cli::Request::PlayerRender { player, game }).and_then(|resp| match resp {
        cli::Response::PlayerRender { render } => Ok(render.into()),
        _ => Err(ErrorKind::Msg("invalid response type".to_string()).into()),
    })
}
