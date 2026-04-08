use config::{Config, File};
fn main() {
    let settings = Config::builder()
        .add_source(File::with_name(".env").required(false))
        .build().unwrap();
    println!("{:?}", settings.try_deserialize::<serde_json::Value>());
}
