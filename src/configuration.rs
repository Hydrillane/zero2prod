use std::env;


use config::{ConfigError,File};
use serde::Deserialize;
use serde_aux::prelude::*;
use secrecy::{ExposeSecret, SecretBox};
use sqlx::{postgres::{PgConnectOptions,PgSslMode},ConnectOptions};
use tracing_log::log::LevelFilter;

#[derive(Deserialize)]
pub struct Setting {
    pub database: DatabaseSetting,
    pub application: ApplicationSettings,
   
}

#[derive(Deserialize)]
pub struct ApplicationSettings {
    pub host:String,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port:u16,
}

#[derive(Deserialize,Debug)]
pub struct DatabaseSetting {
    pub username: String,
    pub password: SecretBox<String>,
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub port: u16,
    pub host: String,
    pub database_name:String,
    pub require_ssl:bool,
}

pub fn get_configuration() -> Result<Setting,ConfigError> {

    let mut settings = config::Config::default();
    let base_path = env::current_dir().expect("Failed to get current dir for base_path");
    let configuration_dir = base_path.join("configuration");

    // required set it to be no excuse to not passing the file configuration 
    settings.merge(config::File::from(configuration_dir.join("base")).required(true)).expect(
        "Fails to merge!"
    );

    let environment : Environment = std::env::var("APP_ENVIRONMENT")
        .unwrap_or_else(|_| "local".into())
        .try_into()
        .expect("Failed to parse APP_ENVIRONMENT.");

    settings.merge(
        File::from(configuration_dir.join(environment.as_str())).required(true)
    )?;

    settings.merge(config::Environment::with_prefix("app").separator("__"))?;

    settings.try_into()
}

pub enum Environment {
    Local,
    Production,
}

impl Environment {
    pub fn as_str(&self) -> &'static str {
        match self {
            Environment::Local => "local",
            Environment::Production => "production",
        }
    }
}

impl TryFrom<String> for Environment {
    type Error = String;

    fn try_from(s:String) -> Result<Self,Self::Error> {
        match s.to_lowercase().as_str() {
            "local" => Ok(Self::Local),
            "production" => Ok(Self::Production),
            other => Err(format!(
                    "{} is not a supported environment. Use either 'local' or `production`."
                    ,
                    other
            )),
        }
    }
}

impl DatabaseSetting {

    pub fn connection_string(&self) -> PgConnectOptions {
        let options = self.connection_without_dbname()
            .database(&self.database_name).log_statements(tracing::log::LevelFilter::Trace);
        options
    }

    pub fn connection_without_dbname(&self) -> PgConnectOptions {
        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(&self.password.expose_secret())
            .port(self.port)
    }
}
