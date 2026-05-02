import re

with open('backend/src/routes/mod.rs', 'r') as f:
    content = f.read()

logistics_routes = '''    // 物流管理路由
    let logistics_routes = Router::new()
        .route("/", get(logistics_handler::list_waybills))
        .route("/", post(logistics_handler::create_waybill))
        .route("/:id", get(logistics_handler::get_waybill))
        .route("/:id", put(logistics_handler::update_waybill_status))
        .route("/:id", delete(logistics_handler::delete_waybill));

    let health_routes = Router::new()'''

content = content.replace('    let health_routes = Router::new()', logistics_routes)

nest_routes = '''        .nest("/api/v1/erp/users", user_routes)
        .nest("/api/v1/erp/logistics", logistics_routes)'''

content = content.replace('        .nest("/api/v1/erp/users", user_routes)', nest_routes)

with open('backend/src/routes/mod.rs', 'w') as f:
    f.write(content)
