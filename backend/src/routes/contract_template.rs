//! 合同模板域路由
//!
//! 功能 合同模板体系 HTTP 路由登记（模板主数据与条款项与状态跃迁与逐条复核共十四表端点）。
//! 调用方 `routes/mod.rs::build_erp_root_router` 以 merge 挂入 `/api/v1/erp` 根域；
//!   本文件只写相对路径，前缀在 nest 处唯一组合（与 iam catalog analytics system 同族形态）。
//! 入参 无。传给谁 全部指向 `handlers/contract_template_handler.rs`。
//! 存什么 不存；路由与权限键的对应关系 资源名派生为 `contract-templates`
//!   （seg3 即权威资源名，非模块前缀），动作取 read/create/update/delete/approve 五个，
//!   其中 activate、retire、revise 与条款项四端点均由方法与路径派生，
//!   末段不落入动作关键字表的走方法派生（PUT 归 update、POST 归 create），
//!   不新增任何动作关键字。
//! 存哪里 见 handlers 与 services。

use crate::container::AppState;
use crate::handlers::contract_template_handler;
use axum::{
    Router,
    routing::{get, post, put},
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/contract-templates",
            get(contract_template_handler::list_templates)
                .post(contract_template_handler::create_template),
        )
        .route(
            "/contract-templates/{id}",
            get(contract_template_handler::get_template)
                .put(contract_template_handler::update_template)
                .delete(contract_template_handler::delete_template),
        )
        .route(
            "/contract-templates/{id}/revise",
            post(contract_template_handler::revise_template),
        )
        .route(
            "/contract-templates/{id}/approve",
            post(contract_template_handler::approve_template),
        )
        .route(
            "/contract-templates/{id}/activate",
            put(contract_template_handler::activate_template),
        )
        .route(
            "/contract-templates/{id}/retire",
            put(contract_template_handler::retire_template),
        )
        .route(
            "/contract-templates/{id}/clauses",
            get(contract_template_handler::list_clauses)
                .post(contract_template_handler::add_clause),
        )
        .route(
            "/contract-templates/{id}/clauses/{clause_id}",
            put(contract_template_handler::update_clause)
                .delete(contract_template_handler::delete_clause),
        )
        .route(
            "/contract-templates/{id}/clauses/{clause_id}/review",
            put(contract_template_handler::review_clause),
        )
}
