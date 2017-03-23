use rustless::framework::client::Client;
use rustless::framework::Namespace;
use rustless::json::JsonValue;
use rustless::backend::HandleResult;
use rustless::Nesting;
use email::MimeMessage;

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
    let parsed = MimeMessage::parse(e).unwrap();
    let bodies = extract_bodies(&parsed);
    println!("{} {:?}", bodies.len(), bodies);
}

fn extract_bodies(mm: &MimeMessage) -> Vec<String> {
    let mut bodies: Vec<String> = vec![mm.body.clone()];
    for c in mm.children.iter() {
        bodies.extend(extract_bodies(c));
    }
    bodies
}
