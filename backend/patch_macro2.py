with open("src/utils/crud_macro.rs", "r") as f:
    content = f.read()

content = content.replace("State(", "axum::extract::State(")
content = content.replace("State<", "axum::extract::State<")
content = content.replace("Query(", "axum::extract::Query(")
content = content.replace("Query<", "axum::extract::Query<")
content = content.replace("Path(", "axum::extract::Path(")
content = content.replace("Path<", "axum::extract::Path<")
content = content.replace("Json(", "axum::Json(")
content = content.replace("Json<", "axum::Json<")
content = content.replace("AppState", "crate::utils::app_state::AppState")
content = content.replace("AppError", "crate::utils::error::AppError")
content = content.replace("ApiResponse", "crate::utils::response::ApiResponse")
content = content.replace("crate::utils::app_state::crate::utils::app_state::AppState", "crate::utils::app_state::AppState")
content = content.replace("crate::utils::error::crate::utils::error::AppError", "crate::utils::error::AppError")
content = content.replace("crate::utils::response::crate::utils::response::ApiResponse", "crate::utils::response::ApiResponse")
content = content.replace("params.validate()", "validator::Validate::validate(&params)")
content = content.replace("req.validate()", "validator::Validate::validate(&req)")

with open("src/utils/crud_macro.rs", "w") as f:
    f.write(content)
