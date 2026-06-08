import sys, re

content = open('src/handlers/auth_handler.rs').read()

# Replace .max_age(axum_extra::extract::cookie::time::Duration::hours(2))
# We will just remove .max_age entirely, or replace with something else.
# But actually it's easier to create the Cookie differently or just use parse.
content = content.replace('.max_age(axum_extra::extract::cookie::time::Duration::hours(2))', '')
content = content.replace('.max_age(axum_extra::extract::cookie::time::Duration::ZERO)', '')

open('src/handlers/auth_handler.rs', 'w').write(content)

