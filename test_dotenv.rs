fn main() {
    std::env::set_var("MY_VAR", "OLD_VALUE");
    std::fs::write(".env", "MY_VAR=NEW_VALUE\n").unwrap();
    dotenvy::dotenv().ok();
    println!("MY_VAR={}", std::env::var("MY_VAR").unwrap());
}
