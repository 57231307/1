use axum_extra::extract::cookie::{CookieJar, Key};

fn test() {
    let jar = CookieJar::new();
    let key = Key::from(b"1234567890123456789012345678901234567890123456789012345678901234");
    let private_jar = jar.private(&key);
}
