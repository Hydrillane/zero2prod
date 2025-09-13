use std::env;

use config::{Config, ConfigError,File};
use serde::Deserialize;
use secrecy::{ExposeSecret, SecretBox};

#[derive(Deserialize)]
pub struct Setting {
    pub database: DatabaseSetting,
    pub application: ApplicationSettings,
   
}

#[derive(Deserialize)]
pub struct ApplicationSettings {
    pub host:String,
    pub port:u16,
}

#[derive(Deserialize)]
pub struct DatabaseSetting {
    pub username: String,
    pub password: SecretBox<String>,
    pub port: u16,
    pub host: String,
    pub database_name:String,
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
    pub fn connection_string(&self) -> SecretBox<String> {
        SecretBox::new(Box::new(format!(
                    "postgres://{}:{}@{}:{}/{}",
                    self.username,
                    self.password.expose_secret(),
                    self.host,
                    self.port,
                    self.database_name
        )))
    }

    pub fn connection_without_dbname(&self) -> SecretBox<String> {
        SecretBox::new(Box::new(format!("postgres://{}:{}@{}:{}",
                    self.username,
                    self.password.expose_secret(),
                    self.host,
                    self.port)))
    }
}
