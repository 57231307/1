use axum_extra::extract::cookie::Key;

fn test() {
    let key = Key::derive_from(b"local-dev-secret-key-change-in-production");
}
