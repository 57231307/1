import re

with open('backend/src/routes/mod.rs', 'r') as f:
    content = f.read()

old_cost = '''    let cost_routes = Router::new()
        .route("/collections", get(cost_collection_handler::list_collections))
        .route("/collections", post(cost_collection_handler::create_collection));'''

new_cost = '''    let cost_routes = Router::new()
        .route("/collections", get(cost_collection_handler::list_collections))
        .route("/collections", post(cost_collection_handler::create_collection))
        .route("/collections/:id", get(cost_collection_handler::get_collection))
        .route("/collections/:id", put(cost_collection_handler::update_collection))
        .route("/collections/:id", delete(cost_collection_handler::delete_collection));'''

content = content.replace(old_cost, new_cost)

with open('backend/src/routes/mod.rs', 'w') as f:
    f.write(content)
