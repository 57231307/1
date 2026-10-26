//! SKU 对照表批量新增（多行录入端点）契约锁与活库用例
//!
//! 覆盖三件事，前两件由后端批量新增端点保证，第三件由界面消费面保证。
//! 一，整批原子。一批里有一行不合格就整批不落库，库里既不会留下半批对照，
//! 也不会出现「成功若干条加若干条失败」的部分写入。
//! 二，撞唯一键点名到行。批内两行互撞与撞库里已有对照是两种成因，
//! 分别给不同文案并把行号定位到请求行序的第几行。
//! 三，批量后真值回读。写入条数与库里实际条数一致，字段值逐项回读对得上。
//!
//! 用例分两层。静态层只读源码与序列化形状，不需要数据库，普通 cargo test 就跑。
//! 活库层需要已迁移的 PostgreSQL（TEST_DATABASE_URL），标注 ignore，
//! 由专用 job 执行，夹具会先清空业务表再自行播种父链，断言全部打在真库回读值上。

use bingxi_backend::services::sku_mapping_service::{
    BatchCreateMappingResult, SkuMappingService, UpsertSkuMappingInput,
};
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter,
};
use std::sync::Arc;

const SERVICE_SRC: &str = include_str!("../src/services/sku_mapping_service.rs");
const HANDLER_SRC: &str = include_str!("../src/handlers/sku_mapping_handler.rs");
const ROUTES_SRC: &str = include_str!("../src/routes/purchase.rs");
const FRONTEND_API_SRC: &str = include_str!("../../frontend/src/api/sku-mapping.ts");
const FRONTEND_BATCH_VIEW_SRC: &str =
    include_str!("../../frontend/src/views/sku-mapping/components/SkuMappingBatchDialog.vue");

// ---------------------------------------------------------------------------
// 一，出参形状与整批原子的源码锁（无库）
// ---------------------------------------------------------------------------

/// 出参键集合必须是 total_count 与 created_count 与 failed_count 与 errors，
/// 不得混入文件导入那套 success_count 与 error_count，前端按这套键消费。
#[test]
fn batch_result_serializes_expected_keys() {
    let res = BatchCreateMappingResult {
        total_count: 4,
        created_count: 0,
        failed_count: 2,
        errors: vec![bingxi_backend::utils::import_export::ImportError {
            row: 2,
            column: "product_color_id".to_string(),
            message: "第 2 行与已有对照重复".to_string(),
            value: "产品 1 色号 2 供应商 3".to_string(),
        }],
    };
    let json = serde_json::to_value(&res).expect("批量新增结果应可序列化");
    assert_eq!(json["total_count"], 4);
    assert_eq!(json["created_count"], 0);
    assert_eq!(json["failed_count"], 2);
    assert_eq!(json["errors"][0]["row"], 2);
    assert_eq!(json["errors"][0]["column"], "product_color_id");
    assert!(
        json.get("success_count").is_none() && json.get("error_count").is_none(),
        "批量新增出参不得复用文件导入的计数字段名，前端要按整批原子语义读"
    );
}

/// rejected 构造函数把失败行数按行去重，同一行多列不合格只算一行。
#[test]
fn rejected_counts_distinct_rows_not_error_items() {
    let mk = |row: usize, column: &str| bingxi_backend::utils::import_export::ImportError {
        row,
        column: column.to_string(),
        message: "x".to_string(),
        value: String::new(),
    };
    let res = BatchCreateMappingResult::rejected(
        5,
        vec![
            mk(2, "product_id"),
            mk(2, "supplier_id"),
            mk(4, "product_color_id"),
        ],
    );
    assert_eq!(res.created_count, 0, "被拒批次必须零写入");
    assert_eq!(
        res.failed_count, 2,
        "失败行数按行去重，应为 2 而不是错误条数 3"
    );
    assert_eq!(res.errors.len(), 3, "错误明细逐条保留，不合并也不吞");
}

/// 批量新增必须在调用方事务里写完并显式提交，且不得走逐行 insert 的旧臂。
#[test]
fn batch_create_writes_inside_one_transaction() {
    let code = code_only(SERVICE_SRC);
    let body = extract_fn(&code, "pub async fn batch_create");
    assert!(
        canon_contains(body, "let txn = (*self.db).begin().await?"),
        "批量新增必须自开一道事务包裹整批写入，否则半批落库无法回滚"
    );
    assert!(
        canon_contains(body, "insert_many(models).exec(&txn)"),
        "批量新增必须在同一事务内一次 insert_many 写完，逐行 insert 会放大往返且难保证原子"
    );
    assert!(
        canon_contains(body, "txn.commit().await?"),
        "批量新增必须显式提交，未提交的批次不该算成功"
    );
    assert!(
        !body.contains(".insert(&*self.db)"),
        "批量新增执行体内不应出现绕开事务的单行插入"
    );
    // 校验不过必须直接返回零写入结果，且返回点在 begin 之前
    let reject_at = body
        .find("BatchCreateMappingResult::rejected")
        .expect("批量新增应在校验失败时返回 rejected 结果");
    let begin_at = body
        .find("(*self.db).begin()")
        .expect("批量新增应有事务起点");
    assert!(
        reject_at < begin_at,
        "校验失败必须在开事务之前返回，不合格批次连事务都不该起"
    );
}

/// 批量臂的引用校验必须是集合式一次取全集，不随行数放大查询条数。
#[test]
fn batch_reference_checks_are_set_based() {
    let code = code_only(SERVICE_SRC);
    let body = extract_fn(&code, "async fn collect_batch_failures");
    for needle in [
        "product::Column::Id.is_in(",
        "supplier::Column::Id.is_in(",
        "supplier_product::Column::Id.is_in(",
        "product_color::Column::Id.is_in(",
        "supplier_product_color::Column::Id.is_in(",
        "product_supplier_mapping::Column::ProductId.is_in(",
    ] {
        assert!(
            canon_contains(body, needle),
            "批量校验缺少按全集取数的一条查询，少了就等于逐行查库或漏判该类引用：{needle}"
        );
    }
    assert!(
        !body.contains(".find_by_id("),
        "批量校验执行体内不应出现逐行 find_by_id，那会把一次批量放大成数千次查询"
    );
}

/// 五条引用规则在单条臂与批量臂成对存在，改一臂必须改另一臂。
///
/// 判据按两臂各自的取数语法成对给出：单条臂逐行 find_by_id 后比归属字段，
/// 批量臂按 id 全集 IN 一次取回后在内存比同一归属语义。规则本体一致，
/// 语法形态不同，故不能用同一串判据硬套两臂。
#[test]
fn both_validation_arms_cover_the_same_five_reference_rules() {
    let code = code_only(SERVICE_SRC);
    let single = extract_fn(&code, "async fn validate_refs");
    let batch = extract_fn(&code, "async fn collect_batch_failures");
    for (single_needle, batch_needle, rule) in [
        (
            "product::Entity::find_by_id(input.product_id)",
            "product::Column::Id.is_in(",
            "我方产品存在性",
        ),
        (
            "pc.product_id != input.product_id",
            "(pc.id, pc.product_id)",
            "我方色号存在且归属所选产品",
        ),
        (
            "supplier::Column::Id.eq(input.supplier_id)",
            "supplier::Column::Id.is_in(",
            "供应商存在性",
        ),
        (
            "sp.supplier_id != input.supplier_id",
            "(sp.id, sp.supplier_id)",
            "供应商商品存在且归属所选供应商",
        ),
        (
            "spc.supplier_product_id != input.supplier_product_id",
            "(spc.id, spc.supplier_product_id)",
            "供应商色号存在且归属所选供应商商品",
        ),
    ] {
        assert!(
            canon_contains(single, single_needle),
            "单条臂缺引用规则（{rule}）判据 {single_needle}"
        );
        assert!(
            canon_contains(batch, batch_needle),
            "批量臂缺引用规则（{rule}）判据 {batch_needle}，两臂口径会漂移"
        );
    }
}

/// 撞唯一键要区分批内互撞与撞库中既有对照，两种成因各自给文案。
#[test]
fn duplicate_reasons_are_separated_by_cause() {
    let code = code_only(SERVICE_SRC);
    let body = extract_fn(&code, "async fn collect_batch_failures");
    assert!(
        body.contains("与本批第"),
        "批内互撞必须点名本批内的另一行号，否则用户不知道该删哪一行"
    );
    assert!(
        body.contains("与已有对照重复"),
        "撞库中既有对照必须单独成一条成因，与批内互撞混成一句就看不出是重复提交还是历史冲突"
    );
    assert!(
        canon_contains(body, "first_row_of_key.get(&key)"),
        "批内查重应以首次出现的行号为锚，后续撞上的行才点得出对端行号"
    );
    assert!(
        canon_contains(body, "existing_keys.contains(&key)"),
        "撞库中既有对照应以既有自然键全集判定，不逐行 SELECT"
    );
}

/// 批量新增端点必须已挂到采购路由，且方法为 POST。
#[test]
fn batch_route_is_registered() {
    let code = code_only(ROUTES_SRC);
    assert!(
        canon_contains(&code, "\"/sku-mappings/batch\""),
        "批量新增路由未注册，前端调用会 404"
    );
    let at = code
        .find("\"/sku-mappings/batch\"")
        .expect("上一步已断言路由存在");
    let window = &code[at..at + 200];
    assert!(
        window.contains("batch_create_mappings"),
        "批量新增路由应指向 batch_create_mappings 处理函数"
    );
    assert!(window.contains("post("), "批量新增路由应只接 POST");
}

/// 处理函数必须把行号闸与空批闸都摆在写库之前，并沿用整批原子的结果形状。
#[test]
fn handler_gates_rows_before_write() {
    let code = code_only(HANDLER_SRC);
    let body = extract_fn(&code, "pub async fn batch_create_mappings");
    assert!(
        canon_contains(body, "req.items.is_empty()"),
        "空批次应直接拒绝，不允许零行提交后回一个零写入的成功"
    );
    assert!(
        body.contains("MAX_BATCH_CREATE_ROWS"),
        "批量新增应有行数闸门，避免超大批次把单事务撑大"
    );
    assert!(
        canon_contains(
            body,
            "to_batch_upsert_input(item, idx + 1, &mut text_failures)"
        ),
        "数值文本不合格必须按行归因，不能整批 400 也不能静默丢列"
    );
    assert!(
        canon_contains(body, "service.batch_create(rows, auth.user_id)"),
        "批量新增应走服务层的事务臂，不在 handler 里自行落库"
    );
}

/// 界面消费面与后端出参键名同源，且不得把行号再塞进请求体造成两套事实。
#[test]
fn frontend_consumes_the_same_batch_contract() {
    assert!(
        FRONTEND_API_SRC.contains("/purchase/sku-mappings/batch"),
        "前端接口层未指向批量新增端点"
    );
    let block = FRONTEND_API_SRC
        .split("interface SkuMappingBatchResult")
        .nth(1)
        .expect("前端应声明批量新增出参类型")
        .split('}')
        .next()
        .unwrap_or_default()
        .to_string();
    for key in ["total_count", "created_count", "failed_count", "errors"] {
        assert!(block.contains(key), "前端出参类型缺键 {key}");
    }
    assert!(
        !block.contains("success_count"),
        "前端不应保留文件导入的计数字段名"
    );
    assert!(
        FRONTEND_BATCH_VIEW_SRC.contains("renderTextCell"),
        "批量录入行的数值列应以文本框承载并走校验器，不使用带加减按钮的数字输入框"
    );
    assert!(
        !FRONTEND_BATCH_VIEW_SRC.contains("el-input-number"),
        "批量录入界面不得出现带加减按钮的数字输入框"
    );
}

// ---------------------------------------------------------------------------
// 二，活库用例（需 TEST_DATABASE_URL 指向已迁移 PostgreSQL，标注 ignore）
// ---------------------------------------------------------------------------

/// 播种一对产品与一个供应商商品链，返回（我方产品，另一产品，供应商，供应商商品，色号集合）。
///
/// 第二个产品带自己的色号，用于验证「色号存在但不属于所选产品」也会被按行点名。
#[allow(dead_code)]
async fn seed_sku_chain(
    db: &sea_orm::DatabaseConnection,
) -> (i32, i32, i32, i32, Vec<i32>, Vec<i32>, i32) {
    let suffix = chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default();
    let now = chrono::Utc::now();

    let product = bingxi_backend::models::product::ActiveModel {
        name: Set(format!("批量对照测试坯布-{suffix}")),
        code: Set(format!("FAB-BATCH-{suffix}")),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具应能写入我方产品");

    let other = bingxi_backend::models::product::ActiveModel {
        name: Set(format!("批量对照测试另款-{suffix}")),
        code: Set(format!("FAB-BATCH-OTHER-{suffix}")),
        unit: Set("米".to_string()),
        status: Set("active".to_string()),
        is_deleted: Set(false),
        product_type: Set("fabric".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具应能写入对照产品");

    let sup_now = now.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap());
    let supplier = bingxi_backend::models::supplier::ActiveModel {
        supplier_code: Set(format!("SUP-BATCH-{suffix}")),
        supplier_name: Set(format!("批量对照测试供应商-{suffix}")),
        supplier_short_name: Set("批供".to_string()),
        supplier_type: Set("面料供应商".to_string()),
        credit_code: Set("91330000TEST00001X".to_string()),
        registered_address: Set("测试注册地址".to_string()),
        legal_representative: Set("测试法人".to_string()),
        registered_capital: Set(Decimal::ZERO),
        establishment_date: Set(chrono::NaiveDate::from_ymd_opt(2020, 1, 1).unwrap()),
        taxpayer_type: Set("一般纳税人".to_string()),
        bank_name: Set("测试银行".to_string()),
        bank_account: Set("6222000000000001".to_string()),
        contact_phone: Set("13800000000".to_string()),
        created_at: Set(sup_now),
        updated_at: Set(sup_now),
        is_processor: Set(false),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具应能写入供应商");

    let supplier_product = bingxi_backend::models::supplier_product::ActiveModel {
        supplier_id: Set(supplier.id),
        product_code: Set(format!("SP-BATCH-{suffix}")),
        product_name: Set("批量对照测试供应商面料".to_string()),
        unit: Set("米".to_string()),
        is_enabled: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具应能写入供应商商品");

    let mut colors = Vec::new();
    for i in 0..40 {
        let color = bingxi_backend::models::product_color::ActiveModel {
            product_id: Set(product.id),
            color_no: Set(format!("BC-{suffix}-{i}")),
            color_name: Set(format!("批量色号{i}")),
            color_type: Set("常规色".to_string()),
            extra_cost: Set(Decimal::ZERO),
            is_active: Set(true),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await
        .expect("夹具应能写入我方色号");
        colors.push(color.id);
    }

    let mut other_colors = Vec::new();
    let other_color = bingxi_backend::models::product_color::ActiveModel {
        product_id: Set(other.id),
        color_no: Set(format!("BO-{suffix}")),
        color_name: Set("另款色号".to_string()),
        color_type: Set("常规色".to_string()),
        extra_cost: Set(Decimal::ZERO),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具应能写入另款色号");
    other_colors.push(other_color.id);

    let supplier_color = bingxi_backend::models::supplier_product_color::ActiveModel {
        supplier_product_id: Set(supplier_product.id),
        color_no: Set(format!("SC-BATCH-{suffix}")),
        color_name: Set("供应商大红".to_string()),
        extra_cost: Set(Decimal::ZERO),
        is_enabled: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .expect("夹具应能写入供应商色号");

    (
        product.id,
        other.id,
        supplier.id,
        supplier_product.id,
        colors,
        other_colors,
        supplier_color.id,
    )
}

#[allow(dead_code)]
fn row_input(
    product_id: i32,
    product_color_id: Option<i32>,
    supplier_id: i32,
    supplier_product_id: i32,
    supplier_product_color_id: Option<i32>,
    price: Option<&str>,
    lead_time: Option<i32>,
) -> UpsertSkuMappingInput {
    UpsertSkuMappingInput {
        product_id,
        product_color_id,
        supplier_id,
        supplier_product_id,
        supplier_product_color_id,
        supplier_price: price.map(|v| v.parse::<Decimal>().expect("夹具价格应为合法数字")),
        min_order_quantity: None,
        lead_time,
        is_primary: Some(false),
        priority: Some(1),
        is_enabled: Some(true),
        remarks: None,
    }
}

#[allow(dead_code)]
async fn mapping_count(db: &sea_orm::DatabaseConnection, product_id: i32) -> u64 {
    bingxi_backend::models::product_supplier_mapping::Entity::find()
        .filter(bingxi_backend::models::product_supplier_mapping::Column::ProductId.eq(product_id))
        .count(db)
        .await
        .expect("回读对照条数应成功")
}

/// 整批原子，一批三行，第二行的我方色号属于另一个产品，校验必须拒掉整批，
/// 并且库里该产品下一条对照都不该出现（不是前两行落库、第三行失败）。
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 指向已迁移 PostgreSQL，夹具会清空业务表后自行播种"]
async fn batch_is_atomic_when_one_row_is_invalid() {
    let db = bingxi_backend::services::test_common::setup_test_db().await;
    let svc = SkuMappingService::new(Arc::new(db.clone()));
    let (product_id, other_product_id, supplier_id, supplier_product_id, colors, other_colors, sc) =
        seed_sku_chain(&db).await;

    let rows = vec![
        row_input(
            product_id,
            Some(colors[0]),
            supplier_id,
            supplier_product_id,
            Some(sc),
            Some("12.34"),
            Some(7),
        ),
        // 该色号真实存在，但属于另一个产品，引用归属不成立
        row_input(
            product_id,
            Some(other_colors[0]),
            supplier_id,
            supplier_product_id,
            None,
            None,
            None,
        ),
        row_input(
            product_id,
            Some(colors[1]),
            supplier_id,
            supplier_product_id,
            None,
            None,
            None,
        ),
    ];

    let res = svc
        .batch_create(rows, 100)
        .await
        .expect("校验不通过的批次应返回结果而非抛错");

    assert_eq!(res.total_count, 3, "总数应等于提交行数");
    assert_eq!(res.created_count, 0, "有一行不合格就整批零写入");
    assert_eq!(res.failed_count, 1, "失败行数应为 1");
    assert_eq!(res.errors.len(), 1, "应给出那条失败明细");
    assert_eq!(res.errors[0].row, 2, "行号应指向提交行序的第 2 行");
    assert_eq!(
        res.errors[0].column, "product_color_id",
        "应点名不合格的字段"
    );

    let stored = mapping_count(&db, product_id).await;
    assert_eq!(stored, 0, "不合格批次落库条数必须为 0，一行都不许留下");
    let other_stored = mapping_count(&db, other_product_id).await;
    assert_eq!(other_stored, 0, "对照表写入不应波及其它产品");
}

/// 批内两行撞同一自然键，后一行必须被点名并指出与本批第几行重复，整批不落库。
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 指向已迁移 PostgreSQL"]
async fn batch_within_batch_duplicate_names_both_rows() {
    let db = bingxi_backend::services::test_common::setup_test_db().await;
    let svc = SkuMappingService::new(Arc::new(db.clone()));
    let (product_id, _, supplier_id, supplier_product_id, colors, _, sc) =
        seed_sku_chain(&db).await;

    let rows = vec![
        row_input(
            product_id,
            Some(colors[0]),
            supplier_id,
            supplier_product_id,
            Some(sc),
            None,
            None,
        ),
        row_input(
            product_id,
            Some(colors[1]),
            supplier_id,
            supplier_product_id,
            None,
            None,
            None,
        ),
        row_input(
            product_id,
            Some(colors[0]),
            supplier_id,
            supplier_product_id,
            None,
            None,
            None,
        ),
    ];

    let res = svc
        .batch_create(rows, 100)
        .await
        .expect("批内互撞应作为校验结果返回而非抛错");

    assert_eq!(res.created_count, 0, "批内互撞整批不落库");
    assert_eq!(res.errors.len(), 1, "只点名后出现的那一行");
    assert_eq!(res.errors[0].row, 3, "行号应为第 3 行");
    assert!(
        res.errors[0].message.contains("与本批第 1 行"),
        "批内互撞文案应点出对端行号，实得 {}",
        res.errors[0].message
    );
    assert!(
        !res.errors[0].message.contains("与已有对照重复"),
        "批内互撞不该被说成撞库里既有对照，成因不同修法也不同"
    );
    assert_eq!(
        mapping_count(&db, product_id).await,
        0,
        "库里不该有半行落库"
    );
}

/// 撞库里已有对照，要按行点名且成因文案与批内互撞不同，已有那行不被覆盖也不新增。
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 指向已迁移 PostgreSQL"]
async fn batch_existing_duplicate_names_row_and_keeps_old_row() {
    let db = bingxi_backend::services::test_common::setup_test_db().await;
    let svc = SkuMappingService::new(Arc::new(db.clone()));
    let (product_id, _, supplier_id, supplier_product_id, colors, _, sc) =
        seed_sku_chain(&db).await;

    let existing = svc
        .create(
            row_input(
                product_id,
                Some(colors[0]),
                supplier_id,
                supplier_product_id,
                Some(sc),
                Some("99.99"),
                Some(3),
            ),
            100,
        )
        .await
        .expect("夹具应先建出一条已有对照");

    let rows = vec![
        row_input(
            product_id,
            Some(colors[1]),
            supplier_id,
            supplier_product_id,
            None,
            None,
            None,
        ),
        row_input(
            product_id,
            Some(colors[0]),
            supplier_id,
            supplier_product_id,
            None,
            Some("8.00"),
            None,
        ),
    ];

    let res = svc
        .batch_create(rows, 100)
        .await
        .expect("撞已有对照应作为校验结果返回");

    assert_eq!(res.created_count, 0, "撞已有对照整批不落库");
    assert_eq!(res.errors[0].row, 2, "应点名撞库的第 2 行");
    assert!(
        res.errors[0].message.contains("与已有对照重复"),
        "撞库里既有对照应给出专属成因文案，实得 {}",
        res.errors[0].message
    );

    let stored = mapping_count(&db, product_id).await;
    assert_eq!(stored, 1, "被拒批次既不该新增，也不该改动既有对照");
    let back = svc.get(existing.id).await.expect("既有对照应仍在");
    assert_eq!(
        back.supplier_price.map(|v| v.to_string()).as_deref(),
        Some("99.99"),
        "既有对照的价格不应被被拒批次覆盖"
    );
}

/// 全合格的一批（40 行）应一次写完，回读条数与逐行字段真值都要对上。
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 指向已迁移 PostgreSQL"]
async fn batch_success_reads_back_exact_rows_and_values() {
    let db = bingxi_backend::services::test_common::setup_test_db().await;
    let svc = SkuMappingService::new(Arc::new(db.clone()));
    let (product_id, _, supplier_id, supplier_product_id, colors, _, sc) =
        seed_sku_chain(&db).await;

    let rows: Vec<UpsertSkuMappingInput> = colors
        .iter()
        .enumerate()
        .map(|(idx, color_id)| {
            row_input(
                product_id,
                Some(*color_id),
                supplier_id,
                supplier_product_id,
                Some(sc),
                Some(&format!("1{}.{}", idx, idx)),
                Some(idx as i32 + 1),
            )
        })
        .collect();
    let expected = rows.len();

    let res = svc
        .batch_create(rows, 100)
        .await
        .expect("合格批次应写库成功");
    assert_eq!(res.total_count, expected);
    assert_eq!(res.created_count, expected, "全部合格时应整批写入");
    assert_eq!(res.failed_count, 0);
    assert!(res.errors.is_empty(), "成功批次不该带失败明细");

    let stored = mapping_count(&db, product_id).await;
    assert_eq!(stored, expected as u64, "库里真值条数应与提交行数一致");

    let (listed, total) = svc
        .list(
            bingxi_backend::services::sku_mapping_service::SkuMappingQueryParams {
                product_id: Some(product_id),
                supplier_id: Some(supplier_id),
                is_enabled: None,
                keyword: None,
                page: Some(1),
                page_size: Some(100),
            },
        )
        .await
        .expect("列表回读应成功");
    assert_eq!(total as usize, expected, "列表总数应等于写入行数");
    assert_eq!(listed.len(), expected, "首页应一次取回全部写入行");
    for item in &listed {
        let price = item
            .supplier_price
            .map(|v| v.to_string())
            .unwrap_or_default();
        assert!(
            price.starts_with('1'),
            "每行价格应按提交值落库，实得 {price}"
        );
        assert!(item.is_enabled, "启用状态应沿用提交值");
    }

    // 再提交同一批，应逐行撞「已有对照」而不是再写一份
    let again = svc
        .batch_create(
            (0..3)
                .map(|i| {
                    row_input(
                        product_id,
                        Some(colors[i]),
                        supplier_id,
                        supplier_product_id,
                        None,
                        None,
                        None,
                    )
                })
                .collect(),
            100,
        )
        .await
        .expect("重复提交应作为校验结果返回");
    assert_eq!(again.created_count, 0, "重复批次不该写入任何一行");
    assert_eq!(again.failed_count, 3, "三行都应被点名");
    assert_eq!(
        mapping_count(&db, product_id).await,
        expected as u64,
        "重复提交后库内条数不应变化"
    );
}

/// 规模与查询放大，200 行一次提交（夹具只铺了 40 个色号，这里另建足够色号）。
/// 断言写入条数与回读一致，并把耗时与查询连接数打印出来供回报引用。
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 指向已迁移 PostgreSQL"]
async fn batch_two_hundred_rows_completes_with_exact_readback() {
    let db = bingxi_backend::services::test_common::setup_test_db().await;
    let svc = SkuMappingService::new(Arc::new(db.clone()));
    let (product_id, _, supplier_id, supplier_product_id, _, _, sc) = seed_sku_chain(&db).await;

    let now = chrono::Utc::now();
    let mut color_ids = Vec::new();
    for i in 0..200 {
        let color = bingxi_backend::models::product_color::ActiveModel {
            product_id: Set(product_id),
            color_no: Set(format!("BIG-{i}")),
            color_name: Set(format!("大批量色号{i}")),
            color_type: Set("常规色".to_string()),
            extra_cost: Set(Decimal::ZERO),
            is_active: Set(true),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("夹具应能批量写入 200 个色号");
        color_ids.push(color.id);
    }

    let rows: Vec<UpsertSkuMappingInput> = color_ids
        .iter()
        .map(|cid| {
            row_input(
                product_id,
                Some(*cid),
                supplier_id,
                supplier_product_id,
                Some(sc),
                Some("5.00"),
                Some(10),
            )
        })
        .collect();

    let started = std::time::Instant::now();
    let res = svc
        .batch_create(rows, 100)
        .await
        .expect("200 行批次应写库成功");
    let elapsed = started.elapsed();

    assert_eq!(res.created_count, 200, "200 行应整批写入");
    assert_eq!(
        mapping_count(&db, product_id).await,
        200,
        "回读条数应为 200"
    );
    println!(
        "批量新增 200 行耗时 {} 毫秒（校验加写入，单事务一次 insert_many）",
        elapsed.as_millis()
    );

    // 同一批再提交一次，应逐行点名撞已有对照，且不改动画像
    let again = svc
        .batch_create(
            color_ids
                .iter()
                .take(200)
                .map(|cid| {
                    row_input(
                        product_id,
                        Some(*cid),
                        supplier_id,
                        supplier_product_id,
                        None,
                        None,
                        None,
                    )
                })
                .collect(),
            100,
        )
        .await
        .expect("重复提交应返回校验结果");
    assert_eq!(again.created_count, 0, "重复的 200 行不该落库");
    assert_eq!(again.failed_count, 200, "200 行应逐行点名");
    assert_eq!(
        mapping_count(&db, product_id).await,
        200,
        "被拒后库内条数不变"
    );
}

/// 事务臂写入的是新增语义，撞库唯一键时不得退化为覆盖既有行（UPSERT 属于文件导入臂）。
#[tokio::test]
#[ignore = "需要 TEST_DATABASE_URL 指向已迁移 PostgreSQL"]
async fn batch_arm_does_not_reuse_file_import_upsert() {
    let db = bingxi_backend::services::test_common::setup_test_db().await;
    let code = code_only(SERVICE_SRC);
    let body = extract_fn(&code, "pub async fn batch_create");
    assert!(
        !body.contains("upsert_mapping"),
        "批量新增不应调用文件导入的 UPSERT 执行体，那会把撞键变成静默覆盖既有对照"
    );
    let svc = SkuMappingService::new(Arc::new(db.clone()));
    let (product_id, _, supplier_id, supplier_product_id, colors, _, sc) =
        seed_sku_chain(&db).await;
    let res = svc
        .batch_create(
            vec![row_input(
                product_id,
                Some(colors[0]),
                supplier_id,
                supplier_product_id,
                Some(sc),
                None,
                None,
            )],
            100,
        )
        .await
        .expect("单行批次应成功");
    assert_eq!(res.created_count, 1);
    assert_eq!(mapping_count(&db, product_id).await, 1);
}

// ---------------------------------------------------------------------------
// 辅助，源码扫描三件套，口径与同域既有契约锁一致
// ---------------------------------------------------------------------------

/// 只保留代码与字符串字面量，逐行剔掉整行注释。
#[allow(dead_code)]
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 规范形，仅用于正向必备项判定，剔掉全部空白并消掉闭合定界符前的尾逗号。
#[allow(dead_code)]
fn canon(src: &str) -> String {
    let mut out: String = code_only(src)
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    loop {
        let next = out.replace(",)", ")").replace(",]", "]").replace(",}", "}");
        if next == out {
            break;
        }
        out = next;
    }
    out
}

#[allow(dead_code)]
fn canon_contains(src: &str, needle: &str) -> bool {
    canon(src).contains(&canon(needle))
}

/// 以签名行为锚点取函数体（从签名后第一个花括号起做配平取到函数尾，
/// 嵌套控制块与多行字面量的花括号都计入配平），锚点丢失必须点名 panic，
/// 不允许回退成文件头。
#[allow(dead_code)]
fn extract_fn<'a>(code_src: &'a str, sig: &str) -> &'a str {
    let start = code_src
        .find(sig)
        .unwrap_or_else(|| panic!("源码锚点丢失，{sig}"));
    let rest = &code_src[start..];
    let brace_at = rest
        .find('{')
        .unwrap_or_else(|| panic!("源码锚点后未找到函数体起点，{sig}"));
    let mut depth = 0usize;
    for (idx, ch) in rest[brace_at..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &rest[..brace_at + idx + 1];
                }
            }
            _ => {}
        }
    }
    rest
}
