use std::env;

use errors::*;

lazy_static! {
  pub static ref CONFIG: Config = from_env().unwrap();
}

pub struct Config {
    pub database_url: String,
    pub database_url_r: Option<String>,
    pub smtp_addr: String,
    pub smtp_user: String,
    pub smtp_pass: String,
    pub mail_from: String,
}

fn from_env() -> Result<Config> {
    Ok(Config {
           database_url: env::var("DATABASE_URL").chain_err(|| "DATABASE_URL must be set")?,
           database_url_r: env::var("DATABASE_URL_R").ok(),
           smtp_addr: env::var("SMTP_ADDR").chain_err(|| "SMTP_ADDR must be set")?,
           smtp_user: env::var("SMTP_USER").chain_err(|| "SMTP_USER must be set")?,
           smtp_pass: env::var("SMTP_PASS").chain_err(|| "SMTP_PASS must be set")?,
           mail_from: env::var("MAIL_FROM").unwrap_or_else(|_| "play@brdg.me".to_string()),
       })
}
