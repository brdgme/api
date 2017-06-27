use hyper::{self, Client as HttpClient};
use hyper::net::HttpsConnector;
use hyper_rustls::TlsClient;
use serde_json;

use brdgme_cmd::cli;

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
