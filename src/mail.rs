use email::MimeMessage;
use lettre::{
    SendableEmail,
    EmailTransport,
    FileEmailTransport,
};
use lettre::smtp::{
    SmtpTransportBuilder,
    SUBMISSION_PORT,
};
use failure::{Error, ResultExt};

use std::env::temp_dir;
use std::io::Read;

use config::{Mail, CONFIG};

pub fn send<'a, U: Read + 'a, T: SendableEmail<'a, U>>(email: T) -> Result<(), Error> {
    match CONFIG.mail {
        Mail::File => Ok(FileEmailTransport::new(temp_dir())
            .send(email)
            .map(|_| ())
            .context("unable to send email")?),
        Mail::Smtp {
            ref addr,
            ref user,
            ref pass,
        } => Ok(SmtpTransportBuilder::new((addr.as_ref(), SUBMISSION_PORT))
            .context("could not initialise SMTP transport")?
            .encrypt()
            .credentials(user, pass)
            .build()
            .send(email)
            .map(|_| ())
            .context("unable to send email")?),
    }
}

pub fn html_layout(content: &str) -> String {
    format!(
        "
        <link
            href=\"https://fonts.googleapis.com/css?family=Source+Code+Pro:400,700\"
            rel=\"stylesheet\"
        >
        <pre
            style=\"
                background-color: white;
                color: black;
                font-family: 'Source Code Pro', 'Lucida Console', monospace;
            \"
        >{}</pre>",
        content
    )
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
