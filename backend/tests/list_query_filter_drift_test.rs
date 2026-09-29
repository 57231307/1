//! 列表筛选「前端下发 → 后端 DTO 接收 → service 真实过滤」三处一致性的防漂移回归。
//!
//! 背景缺陷类：列表页有筛选控件并把 query 参数发出去，但后端查询 DTO 缺该字段，
//! serde 默认忽略未知字段 → 参数被静默丢弃，界面「看起来能筛」结果永不变。既不报错
//! 也不变红，属最危险的「功能没真实接入」。
//!
//! 已收口端点（本测试锁死）：
//! - GET /api/v1/erp/production/production-orders/orders       order_no  （模糊）
//! - GET /api/v1/erp/production/cost-collections               collection_no（模糊）+ status（等值）
//!
//! 取舍：CI 常规集成测试连空库（无表），SQL 层断言需已迁移库且方言绑 PG，脆弱；
//! 而「DTO 是否有字段 / handler 是否把字段透传进 query / service 是否对该 Column 加
//! 真实 filter」都是编译期可见的静态事实，源码文本扫描确定、零依赖、每次必跑。
//! 断言均为「包含」型，规避「不含某串」易被注释自败的坑。

use std::fs;
use std::path::{Path, PathBuf};

fn src(p: &str) -> String {
    let path: PathBuf = Path::new(env!("CARGO_MANIFEST_DIR")).join(p);
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {:?} 失败: {}", path, e))
}

/// 生产订单列表：DTO 收 order_no → handler 透传进 query → service 真实按 OrderNo 列过滤。
#[test]
fn production_order_no_filter_is_wired_end_to_end() {
    let handler = src("src/handlers/production_order_handler.rs");
    // DTO 字段存在
    assert!(
        handler.contains("pub order_no: Option<String>"),
        "ListProductionOrdersQuery 必须声明 order_no 字段，否则前端下发的该参数被 serde 丢弃"
    );
    // handler 把 order_no 透传进 service query（列表 + 导出两处）
    assert!(
        handler.contains("order_no: query.order_no"),
        "handler 必须把 query.order_no 透传进 ProductionOrderQuery（否则等于收到不用）"
    );

    let types = src("src/services/production_order_ops/types.rs");
    assert!(
        types.contains("pub order_no: Option<String>"),
        "内部查询结构 ProductionOrderQuery 必须含 order_no 字段"
    );

    let crud = src("src/services/production_order_ops/crud.rs");
    // service 对 OrderNo 列真实加 like 过滤（模糊），且用 safe_like_pattern 转义通配符
    assert!(
        crud.contains("Column::OrderNo.like("),
        "service.list 必须对 production_order.order_no 列做 like 过滤，防止补了字段却忘了 filter"
    );
    assert!(
        crud.contains("safe_like_pattern("),
        "order_no 模糊过滤必须经 safe_like_pattern 转义 LIKE 通配符（防 % _ 变通配）"
    );
}

/// 成本归集列表：DTO 收 collection_no + status → service 真实按对应列过滤。
#[test]
fn cost_collection_no_and_status_filter_is_wired_end_to_end() {
    let handler = src("src/handlers/cost_collection_handler.rs");
    assert!(
        handler.contains("pub collection_no: Option<String>"),
        "CostCollectionQuery 必须声明 collection_no 字段"
    );
    assert!(
        handler.contains("pub status: Option<String>"),
        "CostCollectionQuery 必须声明 status 字段"
    );

    let service = src("src/services/cost_collection_service.rs");
    assert!(
        service.contains("Column::CollectionNo.like("),
        "get_list 必须对 cost_collection.collection_no 列做 like 过滤"
    );
    // 状态用等值过滤（下拉枚举），锁定新代码的 trimmed 绑定，避免与既有按常量过滤混淆
    assert!(
        service.contains("Column::Status.eq(trimmed)"),
        "get_list 必须对传入的 status 值做等值过滤（Column::Status.eq），而非忽略"
    );
}
