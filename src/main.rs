use config::{Config, Environment, File};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize, Default)]
pub struct AppSettings {
    #[serde(default)]
    pub database: DatabaseConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct DatabaseConfig {
    pub connection_string: String,
    #[serde(default)]
    pub host: String,
}

fn main() {
    std::fs::write("config.yaml", "database:\n  connection_string: \"yaml_string\"\n").unwrap();
    std::fs::write(".env", "DATABASE__CONNECTION_STRING=\"env_string\"\n").unwrap();
    dotenvy::from_path_override(".env").unwrap();
    
    let settings = Config::builder()
        .add_source(File::with_name("config").required(false))
        .add_source(Environment::default().separator("__"))
        .build()
        .unwrap();

    let app_settings = settings.try_deserialize::<AppSettings>().unwrap();
    println!("app_settings.database.connection_string = {}", app_settings.database.connection_string);
}
