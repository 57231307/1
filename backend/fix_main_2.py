import sys
content = open('src/main.rs').read()

content = content.replace('let (app, grpc_db_opt) = match db_result {', 'let app = match db_result {')
content = content.replace('let grpc_db = db.clone();', '')
content = content.replace('(app, Some(grpc_db))', 'app')
content = content.replace('(app, None)', 'app')

open('src/main.rs', 'w').write(content)
