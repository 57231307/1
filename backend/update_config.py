import re

# Update config.yaml
with open('config.yaml', 'r') as f:
    content = f.read()

if 'cookie_secret:' not in content:
    content = content.replace('jwt_secret: "local-dev-secret-key-change-in-production"', 'jwt_secret: "local-dev-secret-key-change-in-production"\n  cookie_secret: "local-dev-cookie-secret-key-must-be-at-least-32-bytes-long"')

with open('config.yaml', 'w') as f:
    f.write(content)

# Update config/settings.rs
with open('src/config/settings.rs', 'r') as f:
    content = f.read()

if 'pub cookie_secret: Option<String>,' not in content:
    content = content.replace('pub previous_jwt_secret: Option<String>,', 'pub previous_jwt_secret: Option<String>,\n    pub cookie_secret: Option<String>,')

with open('src/config/settings.rs', 'w') as f:
    f.write(content)

# Update utils/app_state.rs
with open('src/utils/app_state.rs', 'r') as f:
    content = f.read()

if 'pub cookie_secret: String,' not in content:
    content = content.replace('pub previous_jwt_secret: Option<String>,', 'pub previous_jwt_secret: Option<String>,\n    pub cookie_secret: String,')
    content = content.replace('jwt_secret: String, previous_jwt_secret: Option<String>) -> Self {', 'jwt_secret: String, previous_jwt_secret: Option<String>, cookie_secret: String) -> Self {')
    content = content.replace('previous_jwt_secret,', 'previous_jwt_secret,\n            cookie_secret,')
    content = content.replace('previous_jwt_secret: None,', 'previous_jwt_secret: None,\n            cookie_secret: "default-cookie-secret-key-for-test-environments-only-32-bytes".to_string(),')

with open('src/utils/app_state.rs', 'w') as f:
    f.write(content)
