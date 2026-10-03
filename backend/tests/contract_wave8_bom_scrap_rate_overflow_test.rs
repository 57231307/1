//! CI #4672 e2e/mrp/01:82 契约锁：POST /boms scrap_rate 写边界换算
//!
//! 根因（取证链）：
//! - `bom_items.scrap_rate DECIMAL(5,4)` (migration m0007:41)
//!   p=5, s=4 => 整数位仅 1 位 (p-s=1), 可存 0.0000-9.9999
//! - API 口径为百分比 (10 = 10%)，E2E payload 传 scrap_rate='10.0'
//! - CI HEAD 90ae08ee 的 bom_handler.rs::create_bom 把百分比数值**直插**
//!   DECIMAL(5,4)，整数部分 10 超出 1 位上限 => PG `numeric field overflow` => 500
//! - 修复：handler 写边界经 BomService::scrap_percent_to_ratio
//!   把 10.0 => 0.1000（比率），落库后 DECIMAL(5,4) 可安全容纳
//!
//! 覆盖断言：
//! 1. scrap_percent_to_ratio(Some(10.0)) = Ok(Some(0.1))，落库值与提交值逐位一致
//!    （集成测试：setup_test_db → create → 回查 bom_items.scrap_rate == 0.1000）
//! 2. 越界入参 (>=100 / <0 / 精度超 0.01%) => VALIDATION_ERROR 400 + 可读文案外显
//! 3. None 透传 None，0 透传 Some(0)
//! 4. 读边界 scrap_ratio_to_percent 为写边界的精确逆函数（回显与提交同口径），
//!    树端点序列化亦输出百分比（结构体内部保持存储比率参与计算）
//! 5. 源码扫描锁：写/读边界换算的接线位置（create_bom、update_bom、4 处响应构造、
//!    copy 直通禁止二次换算、update_bom 补 validate）——漏换算/绕过即红

mod test_common;

use bingxi_backend::services::bom_service::{BomService, CreateBomItemRequest, CreateBomRequest};
use rust_decimal::Decimal;
use std::str::FromStr;
use std::sync::Arc;
use test_common::setup_test_db;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

// ============================================================================
// 1. 纯函数 scrap_percent_to_ratio 单元测试（无 DB）
// ============================================================================

/// CI 场景精确复现：scrap_rate=10.0 (百分比) => 0.1000 (存储比率)
#[test]
fn test_scrap_percent_to_ratio_ci_payload() {
    let result = BomService::scrap_percent_to_ratio(Some(dec("10.0")));
    assert!(result.is_ok(), "10% 在合法域内应成功: {result:?}");
    let ratio = result.unwrap().unwrap();
    assert_eq!(ratio, dec("0.1000"), "落库比率应逐位等于 0.1000");
}

/// scrap_rate=5 => 0.05；DECIMAL(5,4) 可容纳整数位 0 + 4 位小数
#[test]
fn test_scrap_percent_to_ratio_five_percent() {
    let result = BomService::scrap_percent_to_ratio(Some(dec("5")));
    assert!(result.is_ok());
    assert_eq!(result.unwrap().unwrap(), dec("0.0500"));
}

/// scrap_rate=0 => 0（无损耗合法值）
#[test]
fn test_scrap_percent_to_ratio_zero() {
    let result = BomService::scrap_percent_to_ratio(Some(Decimal::ZERO));
    assert!(result.is_ok());
    assert_eq!(result.unwrap().unwrap(), Decimal::ZERO);
}

/// scrap_rate=100 => 1.0000（最大合法百分比，整数位 1 恰不超 DECIMAL(5,4) 上限）
#[test]
fn test_scrap_percent_to_ratio_boundary_100() {
    let result = BomService::scrap_percent_to_ratio(Some(dec("100")));
    assert!(result.is_ok());
    assert_eq!(result.unwrap().unwrap(), dec("1.0000"));
}

/// scrap_rate=None => Ok(None)（不指定损耗率合法）
#[test]
fn test_scrap_percent_to_ratio_none() {
    let result = BomService::scrap_percent_to_ratio(None);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), None);
}

/// 越界 scrap_rate=101 => VALIDATION_ERROR 400，文案含实值且可外显
#[test]
fn test_scrap_percent_to_ratio_101_rejected() {
    let result = BomService::scrap_percent_to_ratio(Some(dec("101")));
    assert!(result.is_err(), "101% 超出 0-100 范围应拒绝");
    let err = result.unwrap_err();
    assert_eq!(err.error_code(), "VALIDATION_ERROR");
    // validation_displayable 的 Display 输出包含真实拒绝原因
    let display_msg = err.to_string();
    assert!(
        display_msg.contains("101"),
        "可读文案应包含实值 101，实际: {display_msg}"
    );
}

/// 越界 scrap_rate=-1 => VALIDATION_ERROR 400，文案含实值
#[test]
fn test_scrap_percent_to_ratio_negative_rejected() {
    let result = BomService::scrap_percent_to_ratio(Some(dec("-1")));
    assert!(result.is_err(), "负值应拒绝");
    let err = result.unwrap_err();
    assert_eq!(err.error_code(), "VALIDATION_ERROR");
}

/// 精度超限 scrap_rate=3.333（小数位 > 2，换算后 0.03333 超 4 位）=> VALIDATION_ERROR
#[test]
fn test_scrap_percent_to_ratio_precision_exceeded_rejected() {
    let result = BomService::scrap_percent_to_ratio(Some(dec("3.333")));
    assert!(result.is_err(), "3.333% 小数位超 0.01% 粒度应拒绝");
    let err = result.unwrap_err();
    assert_eq!(err.error_code(), "VALIDATION_ERROR");
}

/// 精度合法 scrap_rate=3.33（小数位 = 2，换算后 0.0333 恰 4 位）=> 通过
#[test]
fn test_scrap_percent_to_ratio_two_decimal_places_ok() {
    let result = BomService::scrap_percent_to_ratio(Some(dec("3.33")));
    assert!(result.is_ok());
    assert_eq!(result.unwrap().unwrap(), dec("0.0333"));
}

// ============================================================================
// 2. 集成测试：真 PG 上 create BOM + 回查落库值逐位一致
//    模拟 handler 调用 scrap_percent_to_ratio 后的完整写入路径
// ============================================================================

/// CI 入参精确场景集成锁：
/// 模拟 handler 边界换算后 (scrap_rate=0.1, quantity=3) 创建 BOM，
/// 回查 bom_items 验证 scrap_rate 落库恰为 0.1000、quantity 恰为 3.0000。
/// 修复前直插 10.0 必触发 PG numeric field overflow（本测试在修复后跑通即证明）。
#[tokio::test]
async fn test_create_bom_ci_payload_stores_correct_ratio() {
    let db = setup_test_db().await;
    let service = BomService::new(Arc::new(db));

    // 模拟 handler 经 scrap_percent_to_ratio(Some(10.0)) 后的存储口径值
    let stored_scrap_rate = BomService::scrap_percent_to_ratio(Some(dec("10.0")))
        .expect("10% 应在合法域内")
        .expect("不应为 None");
    assert_eq!(stored_scrap_rate, dec("0.1000"));

    let req = CreateBomRequest {
        product_id: 99991, // 无 FK 约束，测试隔离用大数
        version: Some(1),
        is_default: Some(false),
        remarks: Some("contract_wave8 overflow pin".to_string()),
        created_by: 1,
        items: vec![
            CreateBomItemRequest {
                material_id: 88881,
                quantity: dec("3.0"),
                unit: Some("千克".to_string()),
                scrap_rate: Some(stored_scrap_rate), // 0.1000
                sort_order: None,
            },
            CreateBomItemRequest {
                material_id: 88882,
                quantity: dec("5.0"),
                unit: Some("米".to_string()),
                scrap_rate: Some(Decimal::ZERO), // 0% 损耗
                sort_order: None,
            },
        ],
    };

    let detail = service
        .create(req)
        .await
        .expect("CI payload 修复后应创建成功，不再 500");

    // 逐位断言：第一条 item
    let item1 = detail
        .items
        .iter()
        .find(|it| it.material_id == 88881)
        .expect("回查应含 material_id=88881 行");
    assert_eq!(
        item1.scrap_rate,
        Some(dec("0.1000")),
        "scrap_rate 落库值应逐位等于 0.1000（API 10% 经换算后）"
    );
    assert_eq!(
        item1.quantity,
        dec("3.0000"),
        "quantity 落库值应逐位等于 3.0000（DECIMAL(12,4)）"
    );

    // 第二条 item
    let item2 = detail
        .items
        .iter()
        .find(|it| it.material_id == 88882)
        .expect("回查应含 material_id=88882 行");
    assert_eq!(
        item2.scrap_rate,
        Some(Decimal::ZERO),
        "scrap_rate=0% 应原样存储为 0"
    );
    assert_eq!(
        item2.quantity,
        dec("5.0000"),
        "quantity 落库值应逐位等于 5.0000"
    );
}

/// 修复前场景负锁：若绕过 handler 边界直接把百分比 10.0 传入服务，
/// 服务层不再做二次换算（CreateBomItemRequest 文档明确 = 存储口径），
/// 10.0 直插 DECIMAL(5,4) 必触发 DB 层错误。本测试证明该值在 PG 被拒。
#[tokio::test]
async fn test_raw_percent_exceeds_decimal_5_4_and_db_rejects() {
    let db = setup_test_db().await;
    let service = BomService::new(Arc::new(db));

    // 故意不走 scrap_percent_to_ratio，直接传百分比数值 10.0
    let req = CreateBomRequest {
        product_id: 99992,
        version: Some(1),
        is_default: Some(false),
        remarks: None,
        created_by: 1,
        items: vec![CreateBomItemRequest {
            material_id: 88883,
            quantity: dec("1"),
            unit: None,
            scrap_rate: Some(dec("10.0")), // 未换算——修复前的错误路径
            sort_order: None,
        }],
    };

    let result = service.create(req).await;
    // PG 对 DECIMAL(5,4) 写入 10.0 必报 overflow；SeaORM 返回 Err
    assert!(
        result.is_err(),
        "绕过 handler 换算直插百分比应被 PG 拒绝（numeric field overflow），实际: {:?}",
        result.map(|d| format!("Ok(id={})", d.bom.id))
    );
}

// ============================================================================
// 3. 读边界/回显：scrap_ratio_to_percent 是写边界的精确逆函数（同一口径闭环）
// ============================================================================

/// 回显逐值一致：合法百分比 p（≤2 位小数）→ 存储 → 回显 == 原值
#[test]
fn test_scrap_read_boundary_roundtrip_percent() {
    for (percent_str, expected_echo) in [
        ("10.0", dec("10.00")),
        ("0", dec("0.00")),
        ("100", dec("100.00")),
        ("3.33", dec("3.33")),
        ("0.01", dec("0.01")),
    ] {
        let stored = BomService::scrap_percent_to_ratio(Some(dec(percent_str)))
            .unwrap_or_else(|e| panic!("{percent_str}% 应合法: {e}"))
            .expect("Some 入参不应得 None");
        let echo =
            BomService::scrap_ratio_to_percent(Some(stored)).expect("Some 存储值回显不应为 None");
        assert_eq!(
            echo, expected_echo,
            "{percent_str}% 回显应逐位等于 {expected_echo}"
        );
    }
}

/// None 透传 None（未填损耗率不回显 0）
#[test]
fn test_scrap_read_boundary_none() {
    assert_eq!(BomService::scrap_ratio_to_percent(None), None);
}

/// 树端点序列化口径：BomTreeNode 内部字段保持存储比率参与 collect_requirements，
/// 序列化输出为百分比（POST 10 ⇒ /boms/:id/tree 回显 10.00，同一字段两种口径即红）
#[test]
fn test_bom_tree_node_serializes_percent_not_ratio() {
    let stored = BomService::scrap_percent_to_ratio(Some(dec("10.0")))
        .unwrap()
        .unwrap(); // 0.1000 存储比率
    let node = bingxi_backend::services::bom_service::BomTreeNode {
        id: "bom-1".to_string(),
        product_id: 1,
        product_name: None,
        quantity: dec("1"),
        unit: None,
        scrap_rate: Some(stored),
        children: vec![],
    };
    let json = serde_json::to_value(&node).expect("BomTreeNode 序列化失败");
    assert_eq!(
        json["scrap_rate"],
        serde_json::json!("10.00"),
        "树端点出参 scrap_rate 必须是 API 百分比口径（存储 0.1000 ⇒ 10.00），实际: {json}"
    );
}

// ============================================================================
// 4. 源码扫描锁：换算链边界接线位置（写边界漏换算 / 读边界漏换算 / copy 二次
//    换算 / update 漏 validate 的回归都会撞红）
// ============================================================================

#[test]
fn scrap_boundary_wiring_source_scan_lock() {
    let handler = include_str!("../src/handlers/bom_handler.rs");

    // 写边界：create_bom 与 update_bom 两个入口都必须经 scrap_percent_to_ratio
    assert_eq!(
        handler
            .matches("scrap_rate: BomService::scrap_percent_to_ratio(item.scrap_rate)?")
            .count(),
        2,
        "写边界换算必须且只允许挂在 create_bom/update_bom 两处（漏一处=百分比直插 500 回潮）"
    );
    // 读边界：create/get/update/copy 四处响应构造都必须经 scrap_ratio_to_percent
    assert_eq!(
        handler
            .matches("scrap_rate: BomService::scrap_ratio_to_percent(item.scrap_rate)")
            .count(),
        4,
        "回显换算必须覆盖 create_bom/get_bom/update_bom/copy_bom 四处响应（缺一处=同一字段两种口径）"
    );
    // update_bom 必须在进入服务（事务写入）前真正执行 DTO 校验，且拒绝原样冒泡
    let update_body = handler
        .split("pub async fn update_bom")
        .nth(1)
        .expect("update_bom 定义缺失")
        .split("/// 删除BOM")
        .next()
        .expect("update_bom 函数体边界缺失");
    assert!(
        update_body.contains("payload.validate().map_err(AppError::from)?;"),
        "update_bom 缺失 validate() 调用或拒绝未原样冒泡（声明的约束恒不生效）"
    );
    assert!(
        !update_body.contains("map_err(AppError::internal"),
        "回潮棘轮：update_bom 把校验/业务拒绝拍平成 internal 500"
    );
    // 明细级约束必须真正参与校验：两个 payload 的 items 均挂 nested
    assert_eq!(
        handler.matches("#[validate(nested)]").count() + handler.matches(", nested)]").count(),
        2,
        "CreateBomPayload.items 与 UpdateBomPayload.items 都必须 nested（明细级约束不下钻=恒不生效）"
    );

    // copy 内部流转 = 存储口径直通，禁止二次换算
    let crud = include_str!("../src/services/bom_ops/crud.rs");
    let copy_body = crud
        .split("pub async fn copy")
        .nth(1)
        .expect("copy 定义缺失")
        .split("/// 获取下一个版本号")
        .next()
        .expect("copy 函数体边界缺失");
    assert!(
        copy_body.contains("scrap_rate: item.scrap_rate,"),
        "copy 必须原样搬运存储口径 scrap_rate（不换算）"
    );
    assert_eq!(
        copy_body.matches("percent_to_ratio").count(),
        0,
        "回潮棘轮：copy 出现百分比↔比率换算（服务层内部二次换算=同一值放大 100 倍/缩小 100 倍）"
    );

    // 内部计算链按存储比率 (1 + rate) 消费，禁止残留 /100 二次缩小
    let tree = include_str!("../src/services/bom_ops/tree.rs");
    let mrp_bom = include_str!("../src/services/mrp_engine_ops/bom.rs");
    assert_eq!(
        tree.matches("rate / Decimal::from(100)").count(),
        0,
        "collect_requirements 不得再把存储比率当百分比 /100（同一乘数两种口径）"
    );
    assert_eq!(
        mrp_bom.matches("scrap_rate / Decimal::from(100)").count(),
        0,
        "calculate_quantity_with_scrap 不得再把存储比率当百分比 /100"
    );
}
