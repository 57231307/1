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
}

fn main() {
    // Simulate what systemd does
    std::env::set_var("DATABASE__CONNECTION_STRING", "\"postgres://my-user:my-pass@localhost/db\"");

    let settings = Config::builder()
        .add_source(File::with_name("config").required(false))
        .add_source(Environment::default().separator("__"))
        .build()
        .unwrap();

    let app_settings = settings.try_deserialize::<AppSettings>().unwrap();
    println!("app_settings.database.connection_string = {}", app_settings.database.connection_string);
}
