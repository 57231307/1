use argon2::{
    password_hash::{
        rand_core::OsRng,
        PasswordHasher, SaltString,
    },
    Argon2,
};

fn main() {
    let password = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("Warning: No password provided, using default 'admin123' (DEVELOPMENT ONLY)");
        "admin123".to_string()
    });
    
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::new(
        argon2::Algorithm::Argon2id,
        argon2::Version::V0x13,
        argon2::Params::new(65536, 3, 4, None).expect("Argon2 params should be valid"),
    );
    
    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .expect("Password hashing should succeed")
        .to_string();
    
    // Output hash to stdout for piping
    println!("{}", password_hash);
    eprintln!("Password hash generated successfully.");
    eprintln!("Note: Do not log or store the plain text password.");
}
