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
        argon2::Params::new(65536, 3, 4, None)
            .expect("不变量：Argon2id 参数（m=64MiB, t=3, p=4）为静态合法值，Params::new 永远成功"),
    );

    // P2-4 修复：超长密码（>16MiB）会导致 hash_password 返回 Err，采用 fail-secure 模式
    let password_hash = match argon2.hash_password(password.as_bytes(), &salt) {
        Ok(hash) => hash.to_string(),
        Err(e) => {
            eprintln!("错误：密码哈希失败：{}", e);
            eprintln!("提示：密码长度可能超过 Argon2 限制（16 MiB）。");
            std::process::exit(1);
        }
    };

    // 输出哈希到 stdout，便于管道使用
    println!("{}", password_hash);
    eprintln!("密码哈希生成成功。");
    eprintln!("注意：请勿记录或存储明文密码。");
}
