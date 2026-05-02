use axum_extra::extract::cookie::{CookieJar, Key, Cookie};

fn test() {
    let jar = CookieJar::new();
    let key = Key::generate();
    let mut private_jar = jar.private(&key);
    let cookie = Cookie::new("jwt", "token");
    private_jar.add(cookie);
}
