use totp_rs::{Algorithm, Secret, TOTP};

fn test() {
    let secret = Secret::generate_secret().to_bytes().unwrap();
    let totp = TOTP::new(
        Algorithm::SHA256,
        6,
        1,
        30,
        secret,
        None,
        "".to_string(),
    ).unwrap();

    let _ = totp.check_current("123456");
    let _ = totp.check_current_with_tolerance("123456", 1);
}
