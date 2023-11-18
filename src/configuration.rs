use config::{Config, File, FileFormat};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Settings {
    pub database: DatabaseSettings,
    pub application: ApplicationSettings,
}

#[derive(Deserialize)]
pub struct DatabaseSettings {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub password: String,
    pub instance: String,
}

#[derive(Deserialize)]
pub struct ApplicationSettings {
    pub port: u16,
}

pub fn get_configuration() -> Result<Settings, config::ConfigError> {
    let settings = Config::builder()
        .add_source(File::new("configuration.yaml", FileFormat::Yaml))
        .build()?;

    settings.try_deserialize::<Settings>()
}
