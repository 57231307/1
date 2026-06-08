content = open('src/main.rs').read()

# Replace `(\n                create_init_router()` with `create_init_router()`
content = content.replace('(\n                create_init_router()', 'create_init_router()')

# And remove `None,\n            )`
content = content.replace('None,\n            )', '')

open('src/main.rs', 'w').write(content)
