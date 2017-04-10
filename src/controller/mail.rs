use rustless::framework::client::Client;
use rustless::framework::Namespace;
use rustless::json::JsonValue;
use rustless::backend::HandleResult;
use rustless::Nesting;

use mail::handle_inbound_email;

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
