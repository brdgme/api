use rustless::framework::client::Client;
use rustless::framework::Namespace;
use rustless::json::JsonValue;
use rustless::backend::HandleResult;
use rustless::Nesting;
use email::MimeMessage;
use lettre::email::SendableEmail;
use lettre::transport::EmailTransport;
use lettre::transport::file::FileEmailTransport;
use lettre::transport::smtp::{SmtpTransportBuilder, SUBMISSION_PORT};

use std::env::temp_dir;

use config::{CONFIG, Mail};
use errors::*;

pub fn namespace(ns: &mut Namespace) {
    ns.post("", |endpoint| {
        endpoint.desc("Handle inbound email");
        endpoint.handle(index)
    });
}

pub fn index<'a>(client: Client<'a>, _params: &JsonValue) -> HandleResult<Client<'a>> {
    match client.request.read_to_end() {
        Ok(Some(s)) => handle_inbound_email(&s),
        _ => panic!("no1"),
    };
    client.empty()
}

pub fn handle_inbound_email(e: &str) {
    // TODO handle error
    let parsed = MimeMessage::parse(e).unwrap();
    let bodies = extract_bodies(&parsed);
    println!("{} {:?}", bodies.len(), bodies);
}

fn extract_bodies(mm: &MimeMessage) -> Vec<String> {
    let mut bodies: Vec<String> = vec![mm.body.clone()];
    for c in &mm.children {
        bodies.extend(extract_bodies(c));
    }
    bodies
}

pub fn html_layout(content: &str) -> String {
    format!("<link href=\"https://fonts.googleapis.com/css?family=Source+Code+Pro\" rel=\"stylesheet\"><pre style=\"background-color: white; color: black; font-family: 'Source Code Pro', monospace;\">{}</pre>",
            content)
}

pub fn send<T: SendableEmail>(email: T) -> Result<()> {
    match CONFIG.mail {
        Mail::File => {
            FileEmailTransport::new(temp_dir())
                .send(email)
                .map(|_| ())
                .chain_err(|| "unable to send email")
        }
        Mail::Smtp {
            ref addr,
            ref user,
            ref pass,
        } => {
            SmtpTransportBuilder::new((addr.as_ref(), SUBMISSION_PORT))
                .chain_err(|| "could not initialise SMTP transport")?
                .encrypt()
                .credentials(user, pass)
                .build()
                .send(email)
                .map(|_| ())
                .chain_err(|| "unable to send email")
        }
    }
}
