use argon2::{password_hash::{rand_core::OsRng, PasswordHasher, SaltString}, Argon2};
fn main() {
    let argon2 = Argon2::default();
    let salt = SaltString::generate(&mut OsRng);
    let hash = argon2.hash_password(b"password123", &salt).unwrap().to_string();
    println!("{}", hash);
}
