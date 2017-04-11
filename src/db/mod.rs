pub mod query;
pub mod models;
pub mod color;
pub mod schema;

use r2d2;
use r2d2_diesel;
use diesel;
use std::env;
use errors::*;

lazy_static! {
    pub static ref CONN: Connections = connect_env().unwrap();
}

pub struct Connections {
    pub w: r2d2::Pool<r2d2_diesel::ConnectionManager<diesel::pg::PgConnection>>,
    pub r: r2d2::Pool<r2d2_diesel::ConnectionManager<diesel::pg::PgConnection>>,
}

pub fn connect(w_addr: &str, r_addr: &str) -> Result<Connections> {
    Ok(Connections {
           w: conn(w_addr)?,
           r: conn(r_addr)?,
       })
}

pub fn connect_env() -> Result<Connections> {
    let w_addr = env::var("DATABASE_URL")
        .chain_err(|| "DATABASE_URL not set")?;
    connect(&w_addr,
            &env::var("DATABASE_URL_R").unwrap_or_else(|_| w_addr.to_owned()))
}

fn conn(addr: &str)
        -> Result<r2d2::Pool<r2d2_diesel::ConnectionManager<diesel::pg::PgConnection>>> {
    r2d2::Pool::new(r2d2::Config::default(),
                    r2d2_diesel::ConnectionManager::<diesel::pg::PgConnection>::new(addr))
            .chain_err(|| "unable to connect to database")
}
