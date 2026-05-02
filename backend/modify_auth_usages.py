import re

# Update middleware/auth.rs
with open('src/middleware/auth.rs', 'r') as f:
    content = f.read()

content = content.replace(
    'let key = Key::derive_from(state.jwt_secret.as_bytes());',
    'let key = Key::derive_from(state.cookie_secret.as_bytes());'
)

with open('src/middleware/auth.rs', 'w') as f:
    f.write(content)


# Update handlers/auth_handler.rs
with open('src/handlers/auth_handler.rs', 'r') as f:
    content = f.read()

content = content.replace(
    'let key = Key::derive_from(state.jwt_secret.as_bytes());',
    'let key = Key::derive_from(state.cookie_secret.as_bytes());'
)

with open('src/handlers/auth_handler.rs', 'w') as f:
    f.write(content)
