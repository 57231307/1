//! 验布四分制同码封顶契约锁（三钉）
//!
//! 裁定依据：四分制同码封顶 4 分，按疵点记录 position_yards 向下取整的整数码分桶，
//! 每码最多计 4 分，评级判定与 points_per_100_sq_yards 都使用封顶后的总分；长度单位
//! 全域统一按码。封顶发生在 grade_inspection 聚合处，历史已评级记录不回算
//! （与本仓历史不追算裁定一致）。
//!
//! 锁定的契约面（均为真 PostgreSQL 落库回读证明，非源码文本比对）：
//! 1. 同码封顶：同一整数码放三枚洞疵点，评级后 total_defect_points 等于 4 而不是线性 12，
//!    并校验 p100 与等级都吃封顶后输入（4 分对应 27.27 判首级；线性 12 泄漏则是 81.82 次级）；
//! 2. 跨码散布：10 枚洞疵点散布第 3 码至第 30 码各占一码，总数保持 40 不因封顶改变，
//!    防把整卷误按一个桶封顶的实现把跨码场景做红（那样会得到 4）；
//! 3. 桶内合计判据：一码放 2 分与 2 分与 3 分三枚（桶内 7 封顶为 4）加另一码 1 枚 1 分，
//!    总分等于 5（线性是 8，逐疵点封顶是 8，每码只取最大是 4，每码无条件记满是 8，
//!    这一条断言同时钉死四种错误语义，检测力自证不得简化成只断一个期望值）。
//!
//! 除零与分母非正路径本轮零改动的依据（既有三层拦截，勿再报除零风险）：
//! backend/src/services/fabric_inspection_service.rs:154-156 受检码数 <= 0 显式业务失败并
//! 点名缺项，backend/src/services/fabric_inspection_service.rs:157-159 幅宽 <= 0 同样显式
//! 失败；调用侧前置校验在 backend/src/services/fabric_inspection_service.rs:498-500，
//! 幅宽 NULL 拒评在 backend/src/services/fabric_inspection_service.rs:528-534；
//! 既有单测锁 backend/tests/services_fabric_inspection_service_test.rs:168-176。
//! 上述行号为本支实况，后续如被他支挪位按函数名重取。
//!
//! 夹具形态：真 PostgreSQL setup_test_db（已迁移库 + TRUNCATE 业务表），照抄
//! contract_wave11_defect_row_process_action_test 范式；同测试二进制由 CI 以
//! --test-threads=1 串行执行，逐用例 setup 不互踩。

mod test_common;

use bingxi_backend::models::fabric_inspection_record;
use bingxi_backend::models::status::fabric_grade;
use bingxi_backend::models::status::fabric_inspection;
use bingxi_backend::models::status::fabric_scoring;
use bingxi_backend::services::fabric_inspection_service::{
    CreateDefectRequest, CreateInspectionRequest, FabricDefectService, FabricInspectionService,
    GradeInspectionRequest,
};
use chrono::NaiveDate;
use rust_decimal::Decimal;
use sea_orm::{DatabaseConnection, EntityTrait};
use std::str::FromStr;
use std::sync::Arc;
use test_common::setup_test_db;

fn dec(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

fn inspection_service(db: &DatabaseConnection) -> FabricInspectionService {
    FabricInspectionService::new(Arc::new(db.clone()))
}

fn defect_service(db: &DatabaseConnection) -> FabricDefectService {
    FabricDefectService::new(Arc::new(db.clone()))
}

/// 建一张幅宽 44 英寸的四分制验布单并推进到 inspecting（返回落库回读的行）
async fn four_point_inspecting(db: &DatabaseConnection) -> fabric_inspection_record::Model {
    let svc = inspection_service(db);
    let created = svc
        .create(
            CreateInspectionRequest {
                flow_card_id: None,
                dye_lot_no: None,
                product_id: None,
                product_name: None,
                color_no: None,
                inspection_date: NaiveDate::from_ymd_opt(2026, 10, 20).unwrap(),
                inspector_id: None,
                inspector_name: None,
                machine_no: None,
                scoring_system: Some(fabric_scoring::FOUR_POINT.to_string()),
                fabric_width_inches: Some(dec("44")),
                remarks: None,
            },
            1,
        )
        .await
        .expect("创建验布记录必须成功");
    svc.start_inspection(created.id)
        .await
        .expect("pending 推进 inspecting 必须成功");
    fabric_inspection_record::Entity::find_by_id(created.id)
        .one(db)
        .await
        .expect("回读验布记录失败")
        .expect("验布记录必须存在")
}

/// 按验布机码表读数添加一枚疵点（hole 走破洞分支定 4 分，其余按英寸长度定分）
async fn add_defect(
    db: &DatabaseConnection,
    inspection_id: i32,
    defect_type: &str,
    position_yards: &str,
    defect_length_inches: &str,
    is_hole: bool,
) {
    defect_service(db)
        .create(CreateDefectRequest {
            inspection_id,
            defect_type: defect_type.to_string(),
            position_yards: dec(position_yards),
            defect_length_inches: dec(defect_length_inches),
            direction: None,
            is_hole: Some(is_hole),
            is_continuous: None,
            is_half_width: None,
            description: None,
        })
        .await
        .expect("添加疵点明细必须成功");
}

async fn add_hole(db: &DatabaseConnection, inspection_id: i32, position_yards: &str) {
    add_defect(db, inspection_id, "hole", position_yards, "0", true).await;
}

/// 评级后从库回读落库行（断言对象是真实落库列，不只是服务返回值）
async fn grade_and_reread(
    db: &DatabaseConnection,
    inspection_id: i32,
    inspected_yards: &str,
) -> fabric_inspection_record::Model {
    inspection_service(db)
        .grade_inspection(
            inspection_id,
            GradeInspectionRequest {
                inspected_yards: dec(inspected_yards),
                qualification_rate: None,
            },
        )
        .await
        .expect("评级必须成功");
    fabric_inspection_record::Entity::find_by_id(inspection_id)
        .one(db)
        .await
        .expect("回读已评级验布记录失败")
        .expect("验布记录必须存在")
}

// =========================================================
// 钉1：同一码多枚疵点，评级用的总扣分被封顶为 4
// =========================================================

#[tokio::test]
async fn nail1_same_yard_holes_grade_input_capped_to_four() {
    let db = setup_test_db().await;
    let inspection = four_point_inspecting(&db).await;

    // 读数 5.10 与 5.50 与 5.99 向下取整都归第 5 码，三枚 4 分洞疵点线性合计是 12
    add_hole(&db, inspection.id, "5.10").await;
    add_hole(&db, inspection.id, "5.50").await;
    add_hole(&db, inspection.id, "5.99").await;

    let graded = grade_and_reread(&db, inspection.id, "12").await;

    assert_eq!(graded.status, fabric_inspection::GRADED);
    assert_eq!(
        graded.total_defect_points, 4,
        "同一码三枚 4 分洞疵点必须封顶计 4，线性合计 12 泄漏进评级输入即判红"
    );
    assert_eq!(
        graded.points_per_100_sq_yards,
        Some(dec("27.27")),
        "p100 必须用封顶后总分 4 计算（12 码乘 44 寸幅宽得 27.27），得 81.82 即线性 12 进了公式"
    );
    assert_eq!(
        graded.grade.as_deref(),
        Some(fabric_grade::FIRST),
        "封顶后 p100 为 27.27 不超 40 判首级；线性 12 进公式会是 81.82 判次级，本断言即封顶进入评级判定的直接证据"
    );
}

// =========================================================
// 钉2：跨码散布十枚洞疵点，封顶不改变总数
// =========================================================

#[tokio::test]
async fn nail2_holes_scattered_across_yards_total_unaffected() {
    let db = setup_test_db().await;
    let inspection = four_point_inspecting(&db).await;

    // 10 枚洞疵点散布第 3 码至第 30 码各占一码，十个桶都未超上限
    for pos in [
        "3.00", "6.00", "9.00", "12.00", "15.00", "18.00", "21.00", "24.00", "27.00", "30.00",
    ] {
        add_hole(&db, inspection.id, pos).await;
    }

    let graded = grade_and_reread(&db, inspection.id, "50").await;

    assert_eq!(
        graded.total_defect_points, 40,
        "每码各一枚时封顶不改变总数，必须保持 40；得 4 说明实现把整卷误按一个桶封顶"
    );
    assert_eq!(
        graded.points_per_100_sq_yards,
        Some(dec("65.45")),
        "总分 40 对 50 码乘 44 寸幅宽，p100 为 65.45"
    );
    assert_eq!(
        graded.grade.as_deref(),
        Some(fabric_grade::SECOND),
        "p100 65.45 超 40 必须判次级，防止误封顶把总数压低后翻成首级"
    );
}

// =========================================================
// 钉3：桶内合计封顶判据，一条断言钉死四种错误语义
// =========================================================

#[tokio::test]
async fn nail3_bucket_sum_capped_excludes_wrong_semantics() {
    let db = setup_test_db().await;
    let inspection = four_point_inspecting(&db).await;

    // 第 5 码放 4 寸与 4 寸与 7 寸三枚（四分制定分 2 与 2 与 3，桶内合计 7 封顶计 4），
    // 第 6 码放一枚 2 寸（定 1 分照计），总扣分必须是 5
    add_defect(&db, inspection.id, "oil_stain", "5.20", "4.00", false).await;
    add_defect(&db, inspection.id, "oil_stain", "5.40", "4.00", false).await;
    add_defect(&db, inspection.id, "streak", "5.60", "7.00", false).await;
    add_defect(&db, inspection.id, "oil_stain", "6.20", "2.00", false).await;

    let graded = grade_and_reread(&db, inspection.id, "12").await;

    assert_eq!(
        graded.total_defect_points, 5,
        "桶内合计 7 封顶为 4 加另码 1 分等于 5；线性 8、逐疵点封顶 8、每码只取最大 4、每码无条件记满 8 四种错法都被这条断言排除"
    );
    assert_eq!(
        graded.points_per_100_sq_yards,
        Some(dec("34.09")),
        "总分 5 对 12 码乘 44 寸幅宽，p100 为 34.09；得 54.55 即线性 8 进了公式"
    );
    assert_eq!(
        graded.grade.as_deref(),
        Some(fabric_grade::FIRST),
        "封顶后 p100 为 34.09 不超 40 判首级"
    );
}
