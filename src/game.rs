use rustless::framework::client::Client;
use rustless::framework::Namespace;
use rustless::json::{JsonValue, ToJson};
use rustless::backend::HandleResult;
use rustless::Nesting;
use valico::json_dsl;

pub fn namespace(ns: &mut Namespace) {
    ns.get("", |endpoint| {
        endpoint.desc("List games");
        endpoint.handle(index)
    });
    ns.get(":id", |endpoint| {
        endpoint.desc("Show game");
        endpoint.params(|params| { params.req_typed("id", json_dsl::string()); });
        endpoint.handle(show)
    });
}

pub fn index<'a>(client: Client<'a>, params: &JsonValue) -> HandleResult<Client<'a>> {
    client.json(&params.to_json())
}

pub fn show<'a>(client: Client<'a>, params: &JsonValue) -> HandleResult<Client<'a>> {
    client.json(&params.to_json())
}
