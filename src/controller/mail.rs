use mail::handle_inbound_email;
use rocket::Data;

use std::io::Read;

use errors::*;

#[post("/", data = "<data>")]
pub fn index(data: Data) -> Result<()> {
    let mut buffer = String::new();
    data.open().read_to_string(&mut buffer)?;
    handle_inbound_email(&buffer);
    Ok(())
}
