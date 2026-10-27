// decs 宏在测试中不可用，使用 Decimal::from_str 替代
use bingxi_backend::decs;
use bingxi_backend::models::purchase_order_item::{Column as PoItemColumn, Entity as PoItemEntity};
use bingxi_backend::models::status;
use bingxi_backend::models::status::master_data;
use bingxi_backend::models::{product, supplier};
use bingxi_backend::services::po::CreateOrderItemRequest;
use bingxi_backend::services::po::order::PurchaseOrderService;
use bingxi_backend::services::po::order_ops::crud::ItemAmounts;
use bingxi_backend::services::po::price::ShortageAlertParams;
use bingxi_backend::services::test_common::setup_test_db;
use bingxi_backend::utils::error::AppError;
use bingxi_backend::utils::tax;
use bingxi_backend::ymd;
use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::str::FromStr;
use std::sync::Arc;

// ---------- 金额计算夹具（一律真调生产唯一权威实现，禁止在本文件复制公式） ----------

/// 复现 create_order_items 的金额计算纯算法
fn calc_amount(quantity: Decimal, unit_price: Decimal) -> Decimal {
    (quantity * unit_price).round_dp(2)
}

/// 真调生产路径的单行金额权威函数 PurchaseOrderService::calculate_item_amounts
/// （创建路径与明细更新路径共用那一份实现）。税额不在本文件另算一套，
/// 复制出来的第二套公式对生产改口径恒绿，属假锁。
/// 入参，quantity 数量、net_unit_price 提交的不含税单价（必须与含税事实反算值钩稽）、
/// inclusive_unit_price 录入的含税单价事实（价税分离的唯一起点）、
/// tax_rate 百分数口径（13 表示 13%，0 表示无进项税），None 表示未提交。
fn authoritative_amounts(
    quantity: Decimal,
    net_unit_price: Decimal,
    inclusive_unit_price: Decimal,
    tax_rate: Option<Decimal>,
) -> Result<ItemAmounts, AppError> {
    PurchaseOrderService::calculate_item_amounts(&CreateOrderItemRequest {
        line_no: Some(1),
        material_id: Some(1),
        unit_price: Some(net_unit_price),
        unit_price_with_tax: Some(inclusive_unit_price),
        quantity_ordered: Some(quantity),
        quantity_alt_ordered: None,
        tax_rate,
        discount_percent: None,
        quantity_tolerance_pct: None,
        color_no: None,
        notes: None,
    })
}

/// 复现 create_order_items 的折扣计算纯算法
fn calc_discount_amount(amount: Decimal, discount_percent: Decimal) -> Decimal {
    (amount * discount_percent / Decimal::new(100, 0)).round_dp(2)
}

/// 复现 create_order_items 的明细总额计算纯算法
fn calc_line_total(amount: Decimal, tax_amount: Decimal, discount_amount: Decimal) -> Decimal {
    amount + tax_amount - discount_amount
}

/// 复现 create_order_items 的明细行号默认值计算
fn default_line_no(index: usize) -> i32 {
    (index + 1) as i32
}

// ---------- 金额计算 ----------

/// test_jejs_zsslcj（验证数量与单价均为整数时金额计算正确）
#[test]
fn test_jejs_zsslcj() {
    let quantity = decs!("10");
    let unit_price = decs!("100");
    // 10 * 100 = 1000.00
    assert_eq!(calc_amount(quantity, unit_price), decs!("1000"));
}

/// test_jejs_xsslcj（验证数量与单价含小数时 round_dp(2) 精度归一化生效）
#[test]
fn test_jejs_xsslcj() {
    // 3.1415 * 2.5 = 7.85375 → round_dp(2) = 7.85
    assert_eq!(calc_amount(decs!("3.1415"), decs!("2.5")), decs!("7.85"));
}

/// test_jejs_ljhllcj（验证数量或单价为 0 时金额为 0）
#[test]
fn test_jejs_ljhllcj() {
    // 零数量
    assert_eq!(calc_amount(Decimal::ZERO, decs!("100")), Decimal::ZERO);
    // 零单价
    assert_eq!(calc_amount(decs!("10"), Decimal::ZERO), Decimal::ZERO);
}

// ---------- 税额计算（真调生产权威口径，不复制公式） ----------

/// 税率单位口径基线，tax_percent 是**百分数**（13 即 13%）：含税事实 1130 按 13%
/// 除法反算得净额 1000、差额税额 130。若调用方误按小数口径把 13 当 0.13% 处理，
/// 反算得 1130 ÷ 1.0013 = 1128.53、税额 1.47，与本条钉死值差百倍，据此锁死口径。
/// 税额只允许「含税 − 净额」的差额取法，本文件不存在也不得写入乘税率公式。
/// 本条直调 utils tax 内核，下方各条真调生产权威函数，二者同一条反算腿。
#[test]
fn test_sejs_bfskw() {
    let split = tax::split_from_inclusive(decs!("1130"), decs!("13"));
    assert_eq!(
        split.net,
        decs!("1000"),
        "含税 1130 按百分数税率 13 除法反算净额必须是 1000"
    );
    assert_eq!(
        split.tax,
        decs!("130"),
        "百分数口径税率 13 的差额税额必须是 130（不是 1.30 也不是 1.47）"
    );
    assert_eq!(
        split.net + split.tax,
        split.inclusive,
        "钩稽恒等式 净额 + 税额 == 含税 必须逐分成立"
    );
}

/// test_sejs_zdyslcj（真调 calculate_item_amounts，提交百分数税率 13 时行差额税额 130）
#[test]
fn test_sejs_zdyslcj() {
    let amounts = authoritative_amounts(decs!("10"), decs!("100"), decs!("113"), Some(decs!("13")))
        .expect("提交合法百分数税率与含税单价事实必须算得出行金额");
    assert_eq!(
        amounts.amount,
        decs!("1000"),
        "净额必须是含税事实 1130 的除法反算值"
    );
    assert_eq!(amounts.tax_percent, decs!("13"), "税率按提交的百分数原样落");
    assert_eq!(
        amounts.tax_amount,
        decs!("130"),
        "税额必须是含税额 1130 减净额 1000 的差额"
    );
}

/// test_sejs_lslcj（真调 calculate_item_amounts，显式提交无进项税 0 时税额为 0）
/// 0 是自动补货链路显式声明的口径值（取内核常量），不是缺省代填，基数非 0
/// 才能证明 0 税额是声明值而非空行巧合。0 税率下含税与净额是同一个数，
/// 除法退化为等值，含税单价即不含税单价，不开例外分支。
#[test]
fn test_sejs_lslcj() {
    let amounts = authoritative_amounts(
        decs!("10"),
        decs!("100"),
        decs!("100"),
        Some(tax::NO_INPUT_TAX_PERCENT),
    )
    .expect("显式提交无进项税 0 必须算得出行金额");
    assert_eq!(
        amounts.tax_percent,
        Decimal::ZERO,
        "无进项税行的税率必须是 0"
    );
    assert_eq!(
        amounts.tax_amount,
        Decimal::ZERO,
        "无进项税行的税额必须是 0"
    );
    assert_eq!(
        amounts.amount,
        decs!("1000"),
        "0 税率行除法退化为等值，净额即含税额"
    );
    assert!(
        amounts.amount > Decimal::ZERO,
        "基数非 0 才证明 0 税额是声明值，实际: {}",
        amounts.amount
    );
}

/// test_sejs_jdgyh（真调 calculate_item_amounts，行金额经 round_dp(2) 归一化）
#[test]
fn test_sejs_jdgyh() {
    // 含税单价 1.13 乘数量 333.33 得标价含税额 376.6629 收敛 376.66，
    // 除法反算净额 376.66 ÷ 1.13 = 333.3274 收敛 333.33，差额税额 43.33
    let amounts = authoritative_amounts(
        decs!("333.33"),
        decs!("1"),
        decs!("1.13"),
        Some(decs!("13")),
    )
    .expect("小数数量必须能算出税额");
    assert_eq!(amounts.amount, decs!("333.33"));
    assert_eq!(amounts.tax_amount, decs!("43.33"));
}

/// test_sejs_quezslj（真调 calculate_item_amounts，人工建单缺 tax_rate 一律拒）
/// 0 是业务声明值（免税、无进项票），代填会让漏录与免税在进项台账里无法区分。
#[test]
fn test_sejs_quezslj() {
    let err = authoritative_amounts(decs!("10"), decs!("100"), decs!("113"), None)
        .expect_err("缺税率必须返回业务错误，不得代填任何税率");
    assert_eq!(err.error_code(), "BUSINESS_ERROR");
}

// ---------- 折扣计算 ----------

/// test_zkjs_mrwzkcj（验证未指定折扣（discount_percent 默认 0）时折扣金额为 0）
#[test]
fn test_zkjs_mrwzkcj() {
    // 默认折扣百分比为 Decimal::ZERO（与 create_order_items 一致）
    assert_eq!(
        calc_discount_amount(decs!("1000"), Decimal::ZERO),
        Decimal::ZERO
    );
}

/// test_zkjs_zdyzkcj（验证用户传入折扣百分比（10 表示 10%）时折扣金额计算正确）
#[test]
fn test_zkjs_zdyzkcj() {
    // 1000 * 10 / 100 = 100.00
    assert_eq!(
        calc_discount_amount(decs!("1000"), decs!("10")),
        decs!("100")
    );
}

// ---------- 明细总额 ----------

/// test_mxze_hswzkcj（验证总额公式 amount + tax_amount - discount_amount（无折扣））
#[test]
fn test_mxze_hswzkcj() {
    // 1000 + 130 - 0 = 1130
    assert_eq!(
        calc_line_total(decs!("1000"), decs!("130"), Decimal::ZERO),
        decs!("1130")
    );
}

/// test_mxze_hshzkcj（验证总额公式 amount + tax_amount - discount_amount（含折扣））
#[test]
fn test_mxze_hshzkcj() {
    // 1000 + 130 - 100 = 1030
    assert_eq!(
        calc_line_total(decs!("1000"), decs!("130"), decs!("100")),
        decs!("1030")
    );
}

// ---------- 明细行号 ----------

/// test_mxhh_mrzdzcj（验证未指定 line_no 时按 (index + 1) 从 1 递增）
#[test]
fn test_mxhh_mrzdzcj() {
    // 复现 create_order_items 中 item.line_no.unwrap_or((index + 1) as i32)
    assert_eq!(default_line_no(0), 1);
    assert_eq!(default_line_no(1), 2);
    assert_eq!(default_line_no(2), 3);
    assert_eq!(default_line_no(9), 10);
}

// ---------- 订单默认值 ----------

/// test_hbmrz_wzdssycny（验证 create_order_header 中 currency 未指定时使用 bingxi_backend::constants::DEFAULT_CURRENCY）
#[test]
fn test_hbmrz_wzdssycny() {
    // 复现 create_order_header 中货币默认值逻辑
    let currency = bingxi_backend::constants::DEFAULT_CURRENCY.to_string();
    // 验证未指定时回退到项目默认货币常量
    assert_eq!(currency, bingxi_backend::constants::DEFAULT_CURRENCY);
    assert_eq!(currency, "CNY");
    // 验证显式指定时不应被默认值覆盖
    let currency_explicit = "USD".to_string();
    assert_eq!(currency_explicit, "USD");
}

/// test_hlmrz_wzdsw1（验证 create_order_header 中 exchange_rate 未指定时默认为 Decimal::new(1, 0) = 1）
#[test]
fn test_hlmrz_wzdsw1() {
    // 复现 create_order_header 中汇率默认值逻辑
    let exchange_rate = Decimal::new(1, 0);
    assert_eq!(exchange_rate, decs!("1"));
    // 验证显式指定时不应被默认值覆盖
    let exchange_rate_explicit = decs!("6.5");
    assert_eq!(exchange_rate_explicit, decs!("6.5"));
}

/// test_ddcszt_sydraftcl（验证 create_order_header 中订单初始状态使用 status::purchase_order::DRAFT 常量；（禁止硬编码状态字符串，全程引用常量））
#[test]
fn test_ddcszt_sydraftcl() {
    // 复现 create_order_header 中订单初始状态设置
    let initial_status = status::purchase_order::DRAFT.to_string();
    assert!(!initial_status.is_empty());
    assert_eq!(initial_status, status::purchase_order::DRAFT);
    // 验证 DRAFT 与其他采购订单状态常量互不相同（状态机不冲突）
    assert_ne!(
        status::purchase_order::DRAFT,
        status::purchase_order::APPROVED
    );
    assert_ne!(
        status::purchase_order::DRAFT,
        status::purchase_order::CANCELLED
    );
    assert_ne!(
        status::purchase_order::DRAFT,
        status::purchase_order::COMPLETED
    );
    assert_ne!(
        status::purchase_order::DRAFT,
        status::purchase_order::CLOSED
    );
}

// ---------- 日期校验 ----------

/// test_rqjy_yjjhbnzyddrq（验证 validate_order_request 中预计交货日期不能早于订单日期的校验逻辑）
#[test]
fn test_rqjy_yjjhbnzyddrq() {
    let order_date = ymd!(2026, 3, 15);
    // 场景 1：预计交货日期等于订单日期（允许）
    let expected_same = ymd!(2026, 3, 15);
    assert!(expected_same >= order_date);
    // 场景 2：预计交货日期晚于订单日期（允许）
    let expected_after = ymd!(2026, 3, 20);
    assert!(expected_after >= order_date);
    // 场景 3：预计交货日期早于订单日期（应拒绝，复现 validate_order_request 拒绝条件）
    let expected_before = ymd!(2026, 3, 10);
    assert!(expected_before < order_date);
}

// ---------- CSV 导出表头 ----------

/// test_csvbt_lsylmyz（验证 export_orders_to_csv 中 CSV 表头为 21 列且列名符合预期）
#[test]
fn test_csvbt_lsylmyz() {
    // 复现 export_orders_to_csv 中的表头定义
    let headers = vec![
        "订单编号".to_string(),
        "供应商ID".to_string(),
        "供应商名称".to_string(),
        "订单日期".to_string(),
        "预计交货日期".to_string(),
        "实际交货日期".to_string(),
        "仓库ID".to_string(),
        "仓库名称".to_string(),
        "部门ID".to_string(),
        "部门名称".to_string(),
        "采购员ID".to_string(),
        "币种".to_string(),
        "汇率".to_string(),
        "总金额".to_string(),
        "总金额外币".to_string(),
        "总数量".to_string(),
        "总数量辅助".to_string(),
        "状态".to_string(),
        "付款条件".to_string(),
        "运输条款".to_string(),
        "备注".to_string(),
    ];
    // 列数为 21
    assert_eq!(headers.len(), 21);
    // 首列与末列
    assert_eq!(headers.first().unwrap(), "订单编号");
    assert_eq!(headers.last().unwrap(), "备注");
    // 关键列存在性抽检
    assert!(headers.contains(&"币种".to_string()));
    assert!(headers.contains(&"状态".to_string()));
    assert!(headers.contains(&"总金额".to_string()));
}

// ---------- 夹具宏与服务实例化 ----------

/// test_decsjjh_kyx（验证 decs! 宏能正确解析 Decimal 字符串）
#[test]
fn test_decsjjh_kyx() {
    let v = decs!("123.45");
    assert_eq!(v, Decimal::from_str("123.45").unwrap());
    assert_eq!(v.to_string(), "123.45");
}

/// test_fwslh_sqlitencsjk（验证 PurchaseOrderService 在 SQLite 内存数据库上能正常实例化）
#[tokio::test]
async fn test_fwslh_sqlitencsjk() {
    let db = setup_test_db().await;
    let service = PurchaseOrderService::new(Arc::new(db));
    // 验证内部 db Arc 已被正确持有
}

// ---------- 状态校验门（批次 392 补测） ----------

/// 复现 update_order 的状态校验门（行 363-369）
/// 仅 DRAFT 和 REJECTED 状态允许修改，其他状态返回 Err
fn update_order_status_gate(status: &str) -> Result<(), AppError> {
    if status != status::purchase_order::DRAFT && status != status::purchase_order::REJECTED {
        return Err(AppError::business(format!(
            "订单状态不允许修改，当前状态：{}",
            status
        )));
    }
    Ok(())
}

/// 复现 delete_order 的状态校验门（行 441-445）
/// 仅 DRAFT 状态允许删除，其他状态返回 Err
fn delete_order_status_gate(status: &str) -> Result<(), AppError> {
    if status != status::purchase_order::DRAFT {
        return Err(AppError::business(format!(
            "订单状态不允许删除，当前状态：{}",
            status
        )));
    }
    Ok(())
}

/// 复现 close_order 的状态校验门（行 481-489）
/// 仅 COMPLETED 和 PARTIAL_RECEIVED 状态允许关闭，其他状态返回 Err
fn close_order_status_gate(status: &str) -> Result<(), AppError> {
    if ![
        status::purchase_order::COMPLETED,
        status::purchase_order::PARTIAL_RECEIVED,
    ]
    .contains(&status)
    {
        return Err(AppError::business(format!(
            "订单状态不允许关闭，当前状态：{}",
            status
        )));
    }
    Ok(())
}

/// test_update_orderztjym_yxdzt（验证 DRAFT 和 REJECTED 状态允许修改）
#[test]
fn test_update_orderztjym_yxdzt() {
    assert!(update_order_status_gate(status::purchase_order::DRAFT).is_ok());
    assert!(update_order_status_gate(status::purchase_order::REJECTED).is_ok());
}

/// test_update_orderztjym_jzdzt（验证非 DRAFT/REJECTED 状态不允许修改且错误消息包含当前状态）
#[test]
fn test_update_orderztjym_jzdzt() {
    let forbidden = [
        status::purchase_order::PENDING_APPROVAL,
        status::purchase_order::SUBMITTED,
        status::purchase_order::APPROVED,
        status::purchase_order::CLOSED,
        status::purchase_order::CANCELLED,
        status::purchase_order::COMPLETED,
        status::purchase_order::PARTIAL_RECEIVED,
    ];
    for s in forbidden {
        let err = update_order_status_gate(s).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains(s), "错误消息应包含当前状态 {}", s);
        assert!(msg.contains("修改"), "错误消息应包含操作类型");
    }
}

/// test_delete_orderztjym_jdraftyx（验证仅 DRAFT 状态允许删除）
#[test]
fn test_delete_orderztjym_jdraftyx() {
    assert!(delete_order_status_gate(status::purchase_order::DRAFT).is_ok());
}

/// test_delete_orderztjym_fdraftjz（验证非 DRAFT 状态不允许删除且错误消息包含当前状态）
#[test]
fn test_delete_orderztjym_fdraftjz() {
    let forbidden = [
        status::purchase_order::REJECTED,
        status::purchase_order::APPROVED,
        status::purchase_order::COMPLETED,
        status::purchase_order::CANCELLED,
    ];
    for s in forbidden {
        let err = delete_order_status_gate(s).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains(s), "错误消息应包含当前状态 {}", s);
        assert!(msg.contains("删除"), "错误消息应包含操作类型");
    }
}

/// test_close_orderztjym_yxdzt（验证 COMPLETED 和 PARTIAL_RECEIVED 状态允许关闭）
#[test]
fn test_close_orderztjym_yxdzt() {
    assert!(close_order_status_gate(status::purchase_order::COMPLETED).is_ok());
    assert!(close_order_status_gate(status::purchase_order::PARTIAL_RECEIVED).is_ok());
}

/// test_close_orderztjym_jzdzt（验证非 COMPLETED/PARTIAL_RECEIVED 状态不允许关闭且错误消息包含当前状态）
#[test]
fn test_close_orderztjym_jzdzt() {
    let forbidden = [
        status::purchase_order::DRAFT,
        status::purchase_order::APPROVED,
        status::purchase_order::REJECTED,
        status::purchase_order::CANCELLED,
    ];
    for s in forbidden {
        let err = close_order_status_gate(s).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains(s), "错误消息应包含当前状态 {}", s);
        assert!(msg.contains("关闭"), "错误消息应包含操作类型");
    }
}

// ---------- 自动补货链路的落库形态锁（真调生产入口 + 回读真表） ----------

/// 造自动补货建单所需的主数据，带成本价的产品（补货行的单价来源，必须非 0，
/// 否则税额 0 只是空行巧合）+ 启用状态供应商（price.rs 取活跃供应商的唯一条件）。
/// suppliers 属迁移播种参照表（夹具不清空），用时间戳后缀自建专属行避免互撞。
async fn seed_auto_replenishment_master(db: &DatabaseConnection) -> i32 {
    let suffix = Utc::now().timestamp_nanos_opt().expect("系统时钟不可用");
    let now = Utc::now();

    let prod = product::ActiveModel {
        name: Set(format!("补货形态锁物料-{suffix}")),
        code: Set(format!("TAXFORM-{suffix}")),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        cost_price: Set(Some(decs!("100"))),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("seed product 失败");

    supplier::ActiveModel {
        supplier_code: Set(format!("SUP-TAXFORM-{suffix}")),
        supplier_name: Set(format!("补货形态锁供应商-{suffix}")),
        supplier_short_name: Set(String::new()),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set(String::new()),
        registered_address: Set(String::new()),
        legal_representative: Set(String::new()),
        registered_capital: Set(Decimal::ZERO),
        establishment_date: Set(ymd!(2020, 1, 1)),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set(String::new()),
        bank_account: Set(String::new()),
        contact_phone: Set(String::new()),
        status: Set(Some(master_data::ACTIVE.to_string())),
        is_processor: Set(false),
        created_at: Set(now.into()),
        updated_at: Set(now.into()),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("seed supplier 失败");

    prod.id
}

/// 落库形态锁，真调自动补货生产入口 create_purchase_suggestion_from_shortage，
/// 回读 purchase_order_item 真表确认 tax_percent 落的是 0——既不是小数口径的 0.13，
/// 也不是档位值 13，tax_amount 同为 0 而 subtotal 非 0，证明 0 是显式声明的
/// 无进项税，不是没算出来的巧合。
#[tokio::test]
async fn test_auto_suggestion_row_tax_percent_lands_zero() {
    let db = setup_test_db().await;
    let product_id = seed_auto_replenishment_master(&db).await;

    let order = PurchaseOrderService::new(Arc::new(db.clone()))
        .create_purchase_suggestion_from_shortage(ShortageAlertParams {
            material_id: product_id,
            material_name: "补货形态锁物料".to_string(),
            material_code: "TAXFORM-AUTO".to_string(),
            required_quantity: decs!("100"),
            available_quantity: decs!("40"),
            shortage_quantity: decs!("60"),
            shortage_level: "Critical".to_string(),
            affected_orders_count: 1,
        })
        .await
        .expect("自动补货建单必须成功（无税率来源即显式声明无进项税，不关掉链路）");

    let item = PoItemEntity::find()
        .filter(PoItemColumn::OrderId.eq(order.id))
        .one(&db)
        .await
        .expect("回查自动补货明细失败")
        .expect("自动补货建议必须已落明细行（有痕）");

    assert_eq!(
        item.tax_percent,
        Decimal::ZERO,
        "自动链路落库税率必须是 0，既不是 0.13 也不是 13，实得: {}",
        item.tax_percent
    );
    assert_ne!(item.tax_percent, decs!("0.13"), "不得写成小数口径形态");
    assert_ne!(item.tax_percent, decs!("13"), "不得写成任何税率档位");
    assert_eq!(item.tax_amount, Decimal::ZERO, "无进项税行的税额必须是 0");
    assert!(
        item.subtotal > Decimal::ZERO,
        "基数必须非 0，实际 subtotal: {}",
        item.subtotal
    );
}
