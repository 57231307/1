use axum_extra::extract::cookie::CookieJar;

fn test() {
    let jar = CookieJar::new();
    let p = jar.private();
}
