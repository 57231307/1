with open("src/handlers/auth_handler.rs", "r") as f:
    content = f.read()

content = content.replace("let key = Key::derive_from", "let key = axum_extra::extract::cookie::Key::derive_from")
content = content.replace("let mut jar = CookieJar::new();\n            jar = jar.private(&key).add(cookie).into_inner();", "let mut jar = axum_extra::extract::cookie::CookieJar::new();\n            jar = jar.add(cookie);")

# Wait, we need it to be private!
# If we do `jar = jar.add(cookie)`, it's not encrypted.
# Actually, the cookie crate provides `CookieJar::private_mut` which adds an encrypted cookie.
# But `axum_extra::extract::cookie::CookieJar` wraps `cookie::CookieJar`.
