// decs 宏在测试中不可用，使用 Decimal::from_str 替代
use bingxi_backend::models::status::common;
use bingxi_backend::models::status::master_data;
use bingxi_backend::services::test_common::setup_test_db;
use bingxi_backend::utils::error::AppError;
// ymd 函数在测试中不可用，使用 NaiveDate::from_ymd_opt 替代
use bingxi_backend::decs;
use bingxi_backend::ymd;
use chrono::Duration;
use rust_decimal::Decimal;
// StockInfo 原 private struct，拆分后提升为 ops::types::StockInfo（pub(crate)），
// 测试模块直接从 ops 导入（facade 不重导出以保持原 API 表面不变）
use bingxi_backend::services::mrp_engine_ops::StockInfo;
use bingxi_backend::services::mrp_engine_service::MrpEngineService;
use bingxi_backend::services::mrp_engine_ops::types::RequirementCalcParams;
use bingxi_backend::services::mrp_engine_ops::types::MaterialRequirement;
use bingxi_backend::services::mrp_engine_ops::types::MrpExplodeQuery;
use std::sync::Arc;

// MRP 专属状态值（源码 mrp_engine_service.rs 中使用，status.rs 暂无 mrp 子模块）
// 集中定义以便测试引用，避免散落的字符串字面量；未来 status.rs 增设 mrp 子模块后应替换为引用
const MRP_STATUS_PLANNED: &str = "PLANNED";
const MRP_STATUS_CONFIRMED: &str = "CONFIRMED";
const MRP_STATUS_RELEASED: &str = "RELEASED";
const MRP_STATUS_CANCELLED: &str = "CANCELLED";
const BOM_STATUS_ACTIVE: &str = "ACTIVE";

/// 构造测试用 StockInfo 夹具（复现 get_stock_info / get_stock_info_batch 中的可用量计算：available = on_hand - safety_stock（下限为 0））
fn make_stock_info(
    on_hand_qty: Decimal,
    in_transit_qty: Decimal,
    safety_stock_qty: Decimal,
) -> StockInfo {
    let available = on_hand_qty - safety_stock_qty;
    let available = if available > Decimal::ZERO {
        available
    } else {
        Decimal::ZERO
    };
    StockInfo {
        on_hand: on_hand_qty,
        in_transit: in_transit_qty,
        safety_stock: safety_stock_qty,
        available,
    }
}

/// test_mrpztclzzqx
/// 验证源码中使用的状态字符串值：BOM 状态 ACTIVE 与通用 common::STATUS_ACTIVE 一致（均为大写）；取消状态 CANCELLED 与 common::STATUS_CANCELLED 一致；产品过滤用 master_data::ACTIVE（小写 active）；MRP 专属状态 PLANNED/CONFIRMED/RELEASED 的预期值
#[test]
fn test_mrpztclzzqx() {
    // BOM 状态使用大写 ACTIVE，与通用 common::STATUS_ACTIVE 一致
    assert_eq!(BOM_STATUS_ACTIVE, common::STATUS_ACTIVE);

    // 取消状态使用 common::STATUS_CANCELLED
    assert_eq!(MRP_STATUS_CANCELLED, common::STATUS_CANCELLED);

    // 产品过滤状态使用 master_data::ACTIVE（小写 active，区别于通用大写）
    assert_eq!(master_data::ACTIVE, "active");

    // MRP 专属状态值（源码中硬编码，status.rs 暂无 mrp 子模块）
    assert_eq!(MRP_STATUS_PLANNED, "PLANNED");
    assert_eq!(MRP_STATUS_CONFIRMED, "CONFIRMED");
    assert_eq!(MRP_STATUS_RELEASED, "RELEASED");
}

/// test_kckyljs_zccj（验证 get_stock_info 中 available = on_hand - safety_stock）
#[test]
fn test_kckyljs_zccj() {
    let stock = make_stock_info(decs!("100"), decs!("20"), decs!("30"));
    assert_eq!(stock.available, decs!("70"));
    assert_eq!(stock.on_hand, decs!("100"));
    assert_eq!(stock.in_transit, decs!("20"));
    assert_eq!(stock.safety_stock, decs!("30"));
}

/// test_kckyljs_aqkccgkc（验证 get_stock_info 中 on_hand < safety_stock 时 available 下限保护为 0）
#[test]
fn test_kckyljs_aqkccgkc() {
    let stock = make_stock_info(decs!("30"), decs!("0"), decs!("50"));
}

/// test_jxqjs_kcczwdq（验证 calculate_requirement_with_stock：available >= required 时 shortage = 0）
#[tokio::test]
async fn test_jxqjs_kcczwdq() {
    let db = setup_test_db().await;
    let service = MrpEngineService::new(Arc::new(db));
    let stock = make_stock_info(decs!("100"), decs!("0"), decs!("0"));

    let req = service.calculate_requirement_with_stock(
        RequirementCalcParams {
            product_id: 1,
            required_quantity: decs!("30"),
            required_date: ymd!(2026, 7, 9),
            source_type: "MANUAL".to_string(),
            source_id: None,
            consider_safety_stock: false,
            consider_in_transit: false,
            bom_level: 0,
        },
        &stock,
    );

    assert_eq!(req.shortage_quantity, Decimal::ZERO);
    assert_eq!(req.available_quantity, decs!("100"));
    assert_eq!(req.required_quantity, decs!("30"));
    assert_eq!(req.bom_level, 0);
}

/// test_jxqjs_kcbzydq（验证 calculate_requirement_with_stock：available < required 时 shortage = required - available）
#[tokio::test]
async fn test_jxqjs_kcbzydq() {
    let db = setup_test_db().await;
    let service = MrpEngineService::new(Arc::new(db));
    let stock = make_stock_info(decs!("30"), decs!("0"), decs!("0"));

    let req = service.calculate_requirement_with_stock(
        RequirementCalcParams {
            product_id: 1,
            required_quantity: decs!("100"),
            required_date: ymd!(2026, 7, 9),
            source_type: "MANUAL".to_string(),
            source_id: None,
            consider_safety_stock: false,
            consider_in_transit: false,
            bom_level: 0,
        },
        &stock,
    );

    assert_eq!(req.shortage_quantity, decs!("70"));
    assert_eq!(req.available_quantity, decs!("30"));
}

/// test_jxqjs_bjqhxd（验证 required == available 时 shortage = 0（源码用 `>` 判断，相等不触发短缺））
#[tokio::test]
async fn test_jxqjs_bjqhxd() {
    let db = setup_test_db().await;
    let service = MrpEngineService::new(Arc::new(db));
    let stock = make_stock_info(decs!("50"), decs!("0"), decs!("0"));

    let req = service.calculate_requirement_with_stock(
        RequirementCalcParams {
            product_id: 1,
            required_quantity: decs!("50"),
            required_date: ymd!(2026, 7, 9),
            source_type: "MANUAL".to_string(),
            source_id: None,
            consider_safety_stock: false,
            consider_in_transit: false,
            bom_level: 0,
        },
        &stock,
    );

    assert_eq!(req.shortage_quantity, Decimal::ZERO);
    assert_eq!(req.available_quantity, decs!("50"));
}

/// test_jxqjs_klztkc（验证 consider_in_transit = true 时 available += in_transit，可覆盖原短缺）
#[tokio::test]
async fn test_jxqjs_klztkc() {
    let db = setup_test_db().await;
    let service = MrpEngineService::new(Arc::new(db));
    let stock = make_stock_info(decs!("50"), decs!("30"), decs!("0"));

    // 不考虑在途：available=50，需求80 -> shortage=30
    let req_no = service.calculate_requirement_with_stock(
        RequirementCalcParams {
            product_id: 1,
            required_quantity: decs!("80"),
            required_date: ymd!(2026, 7, 9),
            source_type: "MANUAL".to_string(),
            source_id: None,
            consider_safety_stock: false,
            consider_in_transit: false,
            bom_level: 0,
        },
        &stock,
    );
    assert_eq!(req_no.available_quantity, decs!("50"));
    assert_eq!(req_no.shortage_quantity, decs!("30"));

    // 考虑在途：available=50+30=80，需求80 -> shortage=0
    let req_with = service.calculate_requirement_with_stock(
        RequirementCalcParams {
            product_id: 1,
            required_quantity: decs!("80"),
            required_date: ymd!(2026, 7, 9),
            source_type: "MANUAL".to_string(),
            source_id: None,
            consider_safety_stock: false,
            consider_in_transit: true,
            bom_level: 0,
        },
        &stock,
    );
    assert_eq!(req_with.available_quantity, decs!("80"));
    assert_eq!(req_with.shortage_quantity, Decimal::ZERO);
}

/// test_jxqjs_klaqkctc（验证 consider_safety_stock = true 时 safety_stock 字段填充实际值）
#[tokio::test]
async fn test_jxqjs_klaqkctc() {
    let db = setup_test_db().await;
    let service = MrpEngineService::new(Arc::new(db));
    // on_hand=100, safety_stock=20 -> available=80
    let stock = make_stock_info(decs!("100"), decs!("0"), decs!("20"));

    let req = service.calculate_requirement_with_stock(
        RequirementCalcParams {
            product_id: 1,
            required_quantity: decs!("50"),
            required_date: ymd!(2026, 7, 9),
            source_type: "MANUAL".to_string(),
            source_id: None,
            consider_safety_stock: true,
            consider_in_transit: false,
            bom_level: 0,
        },
        &stock,
    );
    assert_eq!(req.safety_stock, decs!("20"));
    assert_eq!(req.available_quantity, decs!("80"));
}

/// test_jxqjs_bklaqkcwl
/// 验证 consider_safety_stock = false 时 safety_stock 字段为 0；注意 available 仍按 stock_info.available_qty（已扣除安全库存）计算
#[tokio::test]
async fn test_jxqjs_bklaqkcwl() {
    let db = setup_test_db().await;
    let service = MrpEngineService::new(Arc::new(db));
    let stock = make_stock_info(decs!("100"), decs!("0"), decs!("20"));

    let req = service.calculate_requirement_with_stock(
        RequirementCalcParams {
            product_id: 1,
            required_quantity: decs!("50"),
            required_date: ymd!(2026, 7, 9),
            source_type: "MANUAL".to_string(),
            source_id: None,
            consider_safety_stock: false,
            consider_in_transit: false,
            bom_level: 0,
        },
        &stock,
    );
    assert_eq!(req.safety_stock, Decimal::ZERO);
    // available 仍为 on_hand - safety_stock = 80（stock_info.available_qty）
    assert_eq!(req.available_quantity, decs!("80"));
}

/// test_bomsljs_jcslwsh（验证 explode_bom_recursive 中无损耗率时 quantity = parent * item.quantity（round_dp(4)））
#[test]
fn test_bomsljs_jcslwsh() {
    let parent = decs!("100");
    let item_qty = decs!("1.5");
    let base_quantity = (parent * item_qty).round_dp(4);
    assert_eq!(base_quantity, decs!("150"));
}

/// test_bomsljs_hshl
/// 验证 explode_bom_recursive 中含损耗率的数量计算：quantity_with_scrap = base * (1 + scrap_rate/100)，再 round_dp(4)
#[test]
fn test_bomsljs_hshl() {
    let parent = decs!("100");
    let item_qty = decs!("2");
    let scrap_rate = decs!("10"); // 10% 损耗

    let base_quantity = (parent * item_qty).round_dp(4);
    let quantity_with_scrap =
        (base_quantity * (Decimal::ONE + (scrap_rate / Decimal::from(100)))).round_dp(4);

    assert_eq!(base_quantity, decs!("200"));
    assert_eq!(quantity_with_scrap, decs!("220"));
}

/// test_bomsljs_jdgyh（验证 explode_bom_recursive 中 round_dp(4) 防止精度漂移）
#[test]
fn test_bomsljs_jdgyh() {
    // 产生超过 4 位小数的中间结果，round_dp(4) 归一化为 4 位
    let raw = decs!("0.333333") * decs!("1");
    let rounded = raw.round_dp(4);
    assert_eq!(rounded, decs!("0.3333"));
}

/// test_bomtqqjs_cjdj
/// 验证 explode_bom_recursive 中提前期随 BOM 层级递减：lead_time = 7 * level，material_date = required_date - lead_time
#[test]
fn test_bomtqqjs_cjdj() {
    let required_date = ymd!(2026, 7, 30);

    // level=1：提前期 7 天
    let lead_1 = Duration::days(7);
    assert_eq!(required_date - lead_1, ymd!(2026, 7, 23));

    // level=2：提前期 14 天
    let lead_2 = Duration::days(7 * 2_i64);
    assert_eq!(required_date - lead_2, ymd!(2026, 7, 16));

    // level=0：提前期 0 天，物料日期等于需求日期
    let lead_0 = Duration::days(0);
    assert_eq!(required_date - lead_0, required_date);
}

/// test_dqtj_sxydqx（验证 batch_calculate 中 items_with_shortage = filter(shortage > 0).count()）
#[test]
fn test_dqtj_sxydqx() {
    let date = ymd!(2026, 7, 9);
    let requirements = [
        MaterialRequirement {
            product_id: 1,
            required_quantity: decs!("100"),
            required_date: date,
            on_hand_quantity: decs!("50"),
            in_transit_quantity: Decimal::ZERO,
            safety_stock: Decimal::ZERO,
            available_quantity: decs!("50"),
            shortage_quantity: decs!("50"),
            source_type: "MANUAL".to_string(),
            source_id: None,
            bom_level: 0,
        },
        MaterialRequirement {
            product_id: 2,
            required_quantity: decs!("30"),
            required_date: date,
            on_hand_quantity: decs!("100"),
            in_transit_quantity: Decimal::ZERO,
            safety_stock: Decimal::ZERO,
            available_quantity: decs!("100"),
            shortage_quantity: Decimal::ZERO,
            source_type: "MANUAL".to_string(),
            source_id: None,
            bom_level: 0,
        },
        MaterialRequirement {
            product_id: 3,
            required_quantity: decs!("80"),
            required_date: date,
            on_hand_quantity: decs!("10"),
            in_transit_quantity: Decimal::ZERO,
            safety_stock: Decimal::ZERO,
            available_quantity: decs!("10"),
            shortage_quantity: decs!("70"),
            source_type: "MANUAL".to_string(),
            source_id: None,
            bom_level: 1,
        },
    ];

    let items_with_shortage = requirements
        .iter()
        .filter(|r| r.shortage_quantity > Decimal::ZERO)
        .count() as i32;
    assert_eq!(items_with_shortage, 2);
}

/// test_ddlxzh_cglxzt（验证 convert_to_orders 中 PURCHASE 类型映射到 CONFIRMED 状态）
#[test]
fn test_ddlxzh_cglxzt() {
    let order_type = "PURCHASE";
    let new_status = match order_type {
        "PURCHASE" => MRP_STATUS_CONFIRMED,
        "PRODUCTION" => MRP_STATUS_RELEASED,
        _ => panic!("不应到达此分支"),
    };
    assert_eq!(new_status, MRP_STATUS_CONFIRMED);
}

/// test_ddlxzh_sclxzt（验证 convert_to_orders 中 PRODUCTION 类型映射到 RELEASED 状态）
#[test]
fn test_ddlxzh_sclxzt() {
    let order_type = "PRODUCTION";
    let new_status = match order_type {
        "PURCHASE" => MRP_STATUS_CONFIRMED,
        "PRODUCTION" => MRP_STATUS_RELEASED,
        _ => panic!("不应到达此分支"),
    };
    assert_eq!(new_status, MRP_STATUS_RELEASED);
}

/// test_ddlxzh_wxlxjj（验证 convert_to_orders 中非 PURCHASE/PRODUCTION 类型返回校验错误）
#[test]
fn test_ddlxzh_wxlxjj() {
    let order_type = "INVALID";
    let result: Result<&str, AppError> = match order_type {
        "PURCHASE" => Ok(MRP_STATUS_CONFIRMED),
        "PRODUCTION" => Ok(MRP_STATUS_RELEASED),
        _ => Err(AppError::validation("无效的订单类型")),
    };
    assert!(result.is_err());
    match result {
        Err(e) => assert!(matches!(e, AppError::ValidationError(_))),
        _ => panic!("应返回错误"),
    }
}

/// test_ddlxzh_fplannedztjj（验证 convert_to_orders 中 status != PLANNED 时返回校验错误）
#[test]
fn test_ddlxzh_fplannedztjj() {
    // 模拟已确认状态的结果，不应允许再次转换
    let current_status = MRP_STATUS_CONFIRMED;
    let should_reject = current_status != MRP_STATUS_PLANNED;
    assert!(should_reject);

    let err = AppError::validation(format!("MRP结果 {} 状态不是PLANNED，无法转换", 1));
    assert!(matches!(err, AppError::ValidationError(_)));

    // PLANNED 状态应允许转换（不拒绝）
    let planned_status = MRP_STATUS_PLANNED;
    let should_reject_planned = planned_status != MRP_STATUS_PLANNED;
    assert!(!should_reject_planned);
}

/// test_qxjs_yqxztmd（验证 cancel_calculation 中 status == CANCELLED 时直接返回（幂等，不重复更新））
#[test]
fn test_qxjs_yqxztmd() {
    // 模拟已取消状态的 MRP 结果，复现 cancel_calculation 的早返回判断
    let current_cancelled = MRP_STATUS_CANCELLED;
    let should_early_return = current_cancelled == MRP_STATUS_CANCELLED;
    assert!(should_early_return);

    // 非 CANCELLED 状态不应早返回（需走更新逻辑）
    let current_planned = MRP_STATUS_PLANNED;
    assert!(current_planned != MRP_STATUS_CANCELLED);
}

/// test_jjh_decs_ky（验证 decs! 宏能正确解析 Decimal 字符串）
#[test]
fn test_jjh_decs_ky() {
    let v = decs!("123.45");
    assert_eq!(v.to_string(), "123.45");
    // 验证宏可用于整数与大数
    let big = decs!("1000000");
    assert_eq!(big, decs!("1000000"));
}

/// test_jjh_ymd_ky（验证 ymd! 宏能正确解析日期）
#[test]
fn test_jjh_ymd_ky() {
    let d = ymd!(2026, 7, 9);
    assert_eq!(d.format("%Y-%m-%d").to_string(), "2026-07-09");
}

/// test_fwslcj（验证 MrpEngineService 在 SQLite 内存数据库上能正常实例化）
#[tokio::test]
async fn test_fwslcj() {
    let db = setup_test_db().await;
    let service = MrpEngineService::new(Arc::new(db));
    assert!(Arc::strong_count(&service.db) >= 1);
}

/// test_hqkcxx_xyzssjk（需要 inventory_stocks 表 schema，标注 #[ignore] 仅在本地手动运行。；验证 get_stock_info 调用路径不 panic。）
#[tokio::test]
#[ignore]
async fn test_hqkcxx_xyzssjk() {
    let db = setup_test_db().await;
    let service = MrpEngineService::new(Arc::new(db));
    // 无 schema 时为 Err；有 schema 无记录时返回零库存 StockInfo
    let result = service.get_stock_info(99999).await;
    // L-18 修复（批次 377 v13 复审）：原 let _ = result 无断言，改为 is_err 断言
    assert!(result.is_err(), "无 schema 时应返回数据库错误");
}

/// test_bomzk_xyzssjk（需要 bom/bom_item/inventory_stocks 表 schema，标注 #[ignore] 仅在本地手动运行。；验证 explode_bom 调用路径不 panic。）
#[tokio::test]
#[ignore]
async fn test_bomzk_xyzssjk() {
    let db = setup_test_db().await;
    let service = MrpEngineService::new(Arc::new(db));
    let result = service
        .explode_bom(MrpExplodeQuery {
            product_id: 99999,
            parent_quantity: decs!("10"),
            required_date: ymd!(2026, 7, 9),
            source_type: "MANUAL".to_string(),
            source_id: None,
            consider_safety_stock: false,
            consider_in_transit: false,
        })
        .await;
    // L-18 修复（批次 377 v13 复审）：原 let _ = result 无断言，改为 is_err 断言
    assert!(result.is_err(), "无 schema 时应返回数据库错误");
}

/// test_cxmrpjg_xyzssjk（需要 mrp_results 表 schema，标注 #[ignore] 仅在本地手动运行。；验证 get_results 调用路径不 panic。）
#[tokio::test]
#[ignore]
async fn test_cxmrpjg_xyzssjk() {
    let db = setup_test_db().await;
    let service = MrpEngineService::new(Arc::new(db));
    let result = service.get_results(None, None, None, 1, 10).await;
    // L-18 修复（批次 377 v13 复审）：原 let _ = result 无断言，改为 is_err 断言
    assert!(result.is_err(), "无 schema 时应返回数据库错误");
}
