fn main() {
    std::env::set_var("MY_VAR", "OLD");
    std::fs::write(".env", "MY_VAR=NEW\n").unwrap();
    dotenvy::dotenv_override().ok();
    println!("MY_VAR={}", std::env::var("MY_VAR").unwrap());
}
