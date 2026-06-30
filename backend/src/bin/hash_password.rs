use argon2::{
    password_hash::{rand_core::OsRng, PasswordHasher, SaltString},
    Argon2,
};

fn main() {
    // v9 P1-2 修复：fail-secure 模式，缺失参数时直接退出，禁止 fallback 弱密码
    let password = std::env::args().nth(1).unwrap_or_else(|| {
        eprintln!("错误：未提供密码参数。用法：hash_password <password>");
        eprintln!("fail-secure 模式：禁止使用默认弱密码。");
        std::process::exit(1);
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
