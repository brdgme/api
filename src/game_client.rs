use hyper::{self, Client as HttpClient};
use hyper_tls::HttpsConnector;
use serde_json;
use failure::{Error, ResultExt};

use brdgme_cmd::api;
use brdgme_game::command::Spec as CommandSpec;

pub fn request(uri: &str, request: &api::Request) -> Result<api::Response, Error> {
    let connector = HttpsConnector::new(4).unwrap();
    let https = HttpClient::with_connector(connector);
    let res = https
        .post(uri)
        .body(&serde_json::to_string(request).context("error converting request to JSON")?)
        .send()
        .context("error getting new game state")?;
    if res.status != hyper::StatusCode::OK {
        bail!("game request failed");
    }
    match serde_json::from_reader::<_, api::Response>(res).context("error parsing JSON response")? {
        //api::Response::UserError { message } => Err(ErrorKind::UserError(message).into()),
        api::Response::SystemError { message } => Err(format_err!("{}", message)),
        default => Ok(default),
    }
}

#[derive(Debug, Clone)]
pub struct RenderResponse {
    pub render: String,
    pub state: String,
    pub command_spec: Option<CommandSpec>,
}

impl From<api::PubRender> for RenderResponse {
    fn from(render: api::PubRender) -> Self {
        Self {
            render: render.render,
            state: render.pub_state,
            command_spec: None,
        }
    }
}

impl From<api::PlayerRender> for RenderResponse {
    fn from(render: api::PlayerRender) -> Self {
        Self {
            render: render.render,
            state: render.player_state,
            command_spec: render.command_spec,
        }
    }
}

pub fn render(uri: &str, game: String, player: Option<usize>) -> Result<RenderResponse, Error> {
    match player {
        Some(p) => player_render(uri, game, p),
        None => pub_render(uri, game),
    }
}

pub fn pub_render(uri: &str, game: String) -> Result<RenderResponse, Error> {
    request(uri, &api::Request::PubRender { game }).and_then(|resp| match resp {
        api::Response::PubRender { render } => Ok(render.into()),
        _ => Err(format_err!("invalid response type")),
    })
}

pub fn player_render(uri: &str, game: String, player: usize) -> Result<RenderResponse, Error> {
    request(uri, &api::Request::PlayerRender { player, game }).and_then(|resp| match resp {
        api::Response::PlayerRender { render } => Ok(render.into()),
        _ => Err(format_err!("invalid response type")),
    })
}
