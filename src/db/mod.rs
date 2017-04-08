pub mod query;
pub mod models;
pub mod color;

use r2d2;
use r2d2_postgres::{TlsMode, PostgresConnectionManager};
use std::env;
use errors::*;

pub struct Connections {
    pub w: r2d2::Pool<PostgresConnectionManager>,
    pub r: r2d2::Pool<PostgresConnectionManager>,
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

fn conn(addr: &str) -> Result<r2d2::Pool<PostgresConnectionManager>> {
    r2d2::Pool::new(r2d2::Config::default(),
                    PostgresConnectionManager::new(addr, TlsMode::None)
                        .chain_err(|| "unable to create connection manager")?)
            .chain_err(|| "unable to connect to database")
}

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {}
}
