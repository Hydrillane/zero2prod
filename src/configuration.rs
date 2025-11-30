use std::{env, time::Duration};


use config::{ConfigError,File};
use serde::Deserialize;
use serde_aux::prelude::*;
use secrecy::{ExposeSecret, SecretString};
use sqlx::{postgres::{PgConnectOptions,PgSslMode},ConnectOptions};

use crate::{domain::SubscriberEmail, email_client::EmailClient, startup::HmacSecret};

#[derive(Deserialize,Clone)]
pub struct Setting {
    pub database: DatabaseSetting,
    pub application: ApplicationSettings,
    pub email_client: EmailClientSettings,
    pub redis_uri: SecretString,
   
}


#[derive(Deserialize,Clone)]
pub struct EmailClientSettings {
    pub base_url: String,
    pub sender_email: String,
    pub authorization_token: SecretString,
    pub timeout_milliseconds:u64,
}

impl EmailClientSettings {
    pub fn client(&self) -> EmailClient {
        let sender_email = self.sender().expect("Invalid sender email address");
        let timeout = self.timeout();
        let base_url = self.base_url.clone();
        let token = self.authorization_token.clone();
        EmailClient::new(base_url, sender_email, token, timeout)
    }
    pub fn sender(&self) -> Result<SubscriberEmail,String> {
        SubscriberEmail::parse(self.sender_email.clone())
    }

    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout_milliseconds)
    }
}

#[derive(Deserialize,Clone)]
pub struct ApplicationSettings {
    #[serde(deserialize_with = "deserialize_number_from_string")]
    pub host:String,
    pub port:u16,
    pub base_url: String,
    pub hmac_secret : HmacSecret,
}

#[derive(Deserialize,Debug,Clone)]
pub struct DatabaseSetting {
    pub username: String,
    pub password: SecretString,
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
        let ssl_mode = if self.require_ssl {
            PgSslMode::Require
        } else {
            PgSslMode::Prefer
        };
        PgConnectOptions::new()
            .host(&self.host)
            .username(&self.username)
            .password(&self.password.expose_secret())
            .port(self.port)
            .ssl_mode(ssl_mode)
    }
}
