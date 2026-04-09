use std::env;
fn main() {
    env::set_var("DATABASE__CONNECTION_STRING", "postgres://test");
    println!("val: {}", env::var("DATABASE__CONNECTION_STRING").unwrap());
}
