//! MRP 计算执行
//!
//! 批次 490 D10-3b 拆分：从 mrp_engine_service.rs 抽取的计算执行方法。
//! 包含：run_mrp_calculation（单次计算）+ run_mrp_calculation_for_line（按指定行号前缀计算）
//! + batch_calculate（批量计算）
//! + build_main_result_active_model/build_sub_result_active_model（ActiveModel 构建）

use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use sea_orm::{ActiveModelTrait, DatabaseTransaction, Set, TransactionTrait};

use crate::models::mrp_result::{
    ActiveModel as MrpResultActiveModel, Column as MrpResultColumn, Entity as MrpResultEntity,
    Model as MrpResultModel,
};
// 批次 235 v13 P1-1：MRP 结果状态常量接入（规则 0）
use crate::models::status::mrp as mrp_status;
use crate::utils::error::AppError;
use crate::utils::messages::err_msg;
use crate::utils::number_generator::DocumentNumberGenerator;

use super::types::{
    MaterialRequirement, MrpCalculationQuery, MrpCalculationRequest, MrpCalculationSummary,
    MrpExplodeQuery, RequirementCalcParams,
};
use crate::services::mrp_engine_service::MrpEngineService;

/// MRP 计算编号前缀（落库列 mrp_results.calculation_no，
/// DDL 证据 UNIQUE：migration/src/domain/business/m0007_add_mrp_production_bom.rs:105
/// `"calculation_no" VARCHAR(50) NOT NULL UNIQUE`）：
/// - `MRP`：单次计算主行号，即整次计算的单据号（BOM 子行为 `{行号}-{子行序}`）；
/// - `MRPB`：批量计算批次号，批次内行号为 `{批次号}-{行序}`（BOM 子行再追加 `-{子行序}`）。
/// 旧手写格式 `MRP{13位毫秒时间戳}` / `MRPB{13位毫秒时间戳}`（同毫秒并发即重号，
/// 直撞上述 UNIQUE 列 500），新格式统一 `{前缀}{YYYYMMDD}{3位流水}`
/// （前缀常量集中定义，对齐 chemical_ops/requisition.rs 风格）。
pub const MRP_LINE_NO_PREFIX: &str = "MRP";
pub const MRP_BATCH_NO_PREFIX: &str = "MRPB";

/// MRP 行号/批次号 INSERT 撞唯一约束(23505)时的同事务保存点重试上限。
/// 口径与 `utils/number_generator.rs::insert_with_no_retry` 的私有常量
/// `NO_INSERT_MAX_RETRY = 5` 对齐：MRP 落库是「一次取号派生整批行号」的
/// 多行形态（主行 `{基数}`/`{基数}-{行序}`、BOM 子行 `{基数}-{行序}-{子行}`），
/// 单行构建器 `insert_with_no_retry` 无法直接套用，故在本文件按同一判据
/// （保存点内取号→INSERT，23505 回滚本次尝试重新取号；非冲突类 SQL 错误
/// 原样上抛）实现多行版本。
const MRP_NO_CONFLICT_MAX_RETRY: usize = 5;

/// 判据：错误是否为「单据号列唯一约束冲突（SQLSTATE 23505）」。
/// `utils/error.rs` 的 `From<DbErr>` 已把 Exec/Query 两种形态统一归类为
/// `DatabaseError(err_msg::DB_DUPLICATE)`（error.rs:637-657），按常量比对
/// 而非文案子串——与 `services/product_ops/crud.rs::map_product_fk_error`
/// 同款判据。
fn is_calculation_no_conflict(err: &AppError) -> bool {
    matches!(err, AppError::DatabaseError(m) if m == err_msg::DB_DUPLICATE)
}

/// 重试耗尽的终局错误：与 `insert_with_no_retry` 终局同口径
/// （business_displayable，可外显、非 500、非静默成功）。
fn no_conflict_exhausted(prefix: &str) -> AppError {
    tracing::error!(
        prefix,
        "MRP计算单号连续 {} 次插入冲突，停止重试",
        MRP_NO_CONFLICT_MAX_RETRY
    );
    AppError::business_displayable(format!(
        "MRP计算单号连续 {} 次生成冲突，已停止重试，请稍后重新提交",
        MRP_NO_CONFLICT_MAX_RETRY
    ))
}

impl MrpEngineService {
    /// 执行MRP计算并保存结果（批次 413 技术债务清理：签名从 7 参数改为单一参数对象 `MrpCalculationQuery`，；消除 `clippy::too_many_arguments` 警告。）
    ///
    /// 单产品入口：行号即本次计算的单据号（mrp_results.calculation_no UNIQUE 列），
    /// 经 `generate_no_with_txn` 在事务内取号，主行与 BOM 子行的 INSERT 全部落在
    /// 同一事务、advisory lock 持有到提交（对齐 services/inventory_count_service.rs
    /// 在 count_no UNIQUE 列上的同型形态；一次取号覆盖整单主行+派生子行，
    /// 子行号 `{行号}-{序}` 非独立生成号，不适用逐行 insert_with_no_retry，
    /// 改在本文件按同判据做保存点级整单重试，见 `MRP_NO_CONFLICT_MAX_RETRY`）。
    /// 保持销售订单审批（`services/so/order_workflow.rs`）与生产订单创建
    /// （`services/production_order_ops/crud.rs`）两处调用方的行为不变。
    /// 批量场景请走 `run_mrp_calculation_for_line`，由批次调用方统一决定编号。
    pub async fn run_mrp_calculation(
        &self,
        query: MrpCalculationQuery,
    ) -> Result<Vec<MrpResultModel>, AppError> {
        let txn = (*self.db).begin().await?;
        for attempt in 1..=MRP_NO_CONFLICT_MAX_RETRY {
            // SeaORM 对已开启事务的 begin() 发 SAVEPOINT（口径同
            // utils/number_generator.rs::insert_with_no_retry）
            let sp = txn
                .begin()
                .await
                .map_err(|e| AppError::database(format!("创建MRP计算保存点失败: {e}")))?;
            let line_no = match DocumentNumberGenerator::generate_no_with_txn(
                &sp,
                MRP_LINE_NO_PREFIX,
                MrpResultEntity,
                MrpResultColumn::CalculationNo,
            )
            .await
            {
                Ok(no) => no,
                Err(e) => {
                    // 取号失败：先回滚保存点（回滚失败只记日志，不得顶替原始错误），
                    // 原始错误按既有口径包装为可外显业务错误上抛
                    if let Err(re) = sp.rollback().await {
                        tracing::error!(
                            error = %re,
                            original_error = %e,
                            "回滚MRP取号保存点失败（取号原始错误继续上抛）"
                        );
                    }
                    tracing::error!(
                        error = %e,
                        prefix = MRP_LINE_NO_PREFIX,
                        "MRP计算行号取号失败（mrp_engine_ops/calculation.run_mrp_calculation）"
                    );
                    let err = AppError::business_displayable("MRP计算单号生成失败，请稍后重试");
                    return Err(err);
                }
            };
            match self
                .run_mrp_calculation_for_line(&sp, query.clone(), &line_no)
                .await
            {
                Ok((results, _)) => {
                    sp.commit()
                        .await
                        .map_err(|e| AppError::database(format!("释放MRP取号保存点失败: {e}")))?;
                    txn.commit().await?;
                    return Ok(results);
                }
                Err(err) => {
                    if let Err(re) = sp.rollback().await {
                        return Err(AppError::database(format!(
                            "回滚MRP取号保存点失败（原始插入错误: {err}）: {re}"
                        )));
                    }
                    if !is_calculation_no_conflict(&err) {
                        // 与单号无关的 SQL/业务错误（FK/CHECK/计算失败）重试无意义，原样上抛
                        return Err(err);
                    }
                    tracing::warn!(
                        prefix = MRP_LINE_NO_PREFIX,
                        attempt,
                        doc_no = %line_no,
                        error = %err,
                        "MRP计算行号唯一约束冲突(23505)，保存点已回滚，重新取号重试"
                    );
                }
            }
        }
        Err(no_conflict_exhausted(MRP_LINE_NO_PREFIX))
    }

    /// 以给定行号前缀执行一次（单产品）MRP 计算并落库（INSERT 全部走调用方事务 `txn`，
    /// 与批次号/行号的取号同事务，禁止改回 `&*self.db` 裸连接提交）。
    ///
    /// `line_no` 即本次计算主物料行的 `calculation_no`，其 BOM 子行为 `{line_no}-{子行序}`；
    /// 由 batch_calculate 统一编号为 `{批次号}-{行序}`，使同一批次内主行/子行互不相同、
    /// 不同批次间因批次号不同而互不相同（`mrp_results.calculation_no` 带 UNIQUE 约束），
    /// 同时让整批行都能以批次号作前缀被检索出来。
    ///
    /// 返回 `(落库行, 需求行)`：需求行 = 父行（bom_level 0）+ `explode_bom` 展开出的
    /// 全部子行（bom_level ≥ 1），与落库行同源。调用方：`run_mrp_calculation`（只取落库行）、
    /// `build_batch_rows`（两份都要——汇总 `requirements` 必须含子行，禁止另行重算第二套公式）。
    pub(crate) async fn run_mrp_calculation_for_line(
        &self,
        txn: &DatabaseTransaction,
        query: MrpCalculationQuery,
        line_no: &str,
    ) -> Result<(Vec<MrpResultModel>, Vec<MaterialRequirement>), AppError> {
        let mut results = Vec::new();

        // 计算主物料需求
        let main_req = self
            .calculate_requirement(RequirementCalcParams {
                product_id: query.product_id,
                required_quantity: query.required_quantity,
                required_date: query.required_date,
                source_type: query.source_type.clone(),
                source_id: query.source_id,
                consider_safety_stock: query.consider_safety_stock,
                consider_in_transit: query.consider_in_transit,
                bom_level: 0,
            })
            .await?;
        let mut requirements = vec![main_req.clone()];

        // 构建并保存主物料结果（与取号同事务）
        let main_active_model = Self::build_main_result_active_model(line_no, &main_req);
        let main_result = main_active_model.insert(txn).await?;
        results.push(main_result);

        // 展开 BOM 获取子物料需求
        let sub_requirements = self
            .explode_bom(MrpExplodeQuery {
                product_id: query.product_id,
                parent_quantity: query.required_quantity,
                required_date: query.required_date,
                source_type: query.source_type,
                source_id: query.source_id,
                consider_safety_stock: query.consider_safety_stock,
                consider_in_transit: query.consider_in_transit,
            })
            .await?;

        // 遍历构建并保存子物料结果；子需求行随落库行一并返回（同一份数据，不再二次计算）
        for (idx, req) in sub_requirements.iter().enumerate() {
            let sub_active_model = Self::build_sub_result_active_model(line_no, idx, req);
            let sub_result = sub_active_model.insert(txn).await?;
            results.push(sub_result);
            requirements.push(req.clone());
        }

        Ok((results, requirements))
    }

    /// 构建主物料 MRP 结果 ActiveModel
    fn build_main_result_active_model(
        line_no: &str,
        main_req: &MaterialRequirement,
    ) -> MrpResultActiveModel {
        MrpResultActiveModel {
            calculation_no: Set(line_no.to_string()),
            product_id: Set(main_req.product_id),
            required_quantity: Set(main_req.required_quantity),
            required_date: Set(Some(main_req.required_date)),
            source_type: Set(main_req.source_type.clone()),
            source_id: Set(main_req.source_id),
            planned_order_quantity: Set(Some(main_req.shortage_quantity)),
            planned_order_date: Set(Some(main_req.required_date - Duration::days(14))),
            status: Set(mrp_status::PLANNED.to_string()),
            remarks: Set(Some(format!(
                "BOM Level: 0, On Hand: {}, Shortage: {}",
                main_req.on_hand_quantity, main_req.shortage_quantity
            ))),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
    }

    /// 构建子物料 MRP 结果 ActiveModel
    fn build_sub_result_active_model(
        line_no: &str,
        idx: usize,
        req: &MaterialRequirement,
    ) -> MrpResultActiveModel {
        MrpResultActiveModel {
            calculation_no: Set(format!("{}-{}", line_no, idx + 1)),
            product_id: Set(req.product_id),
            required_quantity: Set(req.required_quantity),
            required_date: Set(Some(req.required_date)),
            source_type: Set(req.source_type.clone()),
            source_id: Set(req.source_id),
            planned_order_quantity: Set(Some(req.shortage_quantity)),
            planned_order_date: Set(Some(
                req.required_date - Duration::days(7 * req.bom_level as i64),
            )),
            status: Set(mrp_status::PLANNED.to_string()),
            remarks: Set(Some(format!(
                "BOM Level: {}, On Hand: {}, In Transit: {}, Shortage: {}",
                req.bom_level, req.on_hand_quantity, req.in_transit_quantity, req.shortage_quantity
            ))),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        }
    }

    /// 批量MRP计算
    ///
    /// 批次号取号与整批行 INSERT 落在同一保存点事务内：若派生行号撞
    /// `calculation_no` UNIQUE（23505，旁路/人工输入重复号），回滚本批、
    /// 重新取批次号重试（口径同 `utils/number_generator.rs::insert_with_no_retry`，
    /// 多行形态在本文件实现，见 `MRP_NO_CONFLICT_MAX_RETRY`）。
    pub async fn batch_calculate(
        &self,
        request: MrpCalculationRequest,
    ) -> Result<MrpCalculationSummary, AppError> {
        // 一次批量计算 = 一个批次：批次号既作为响应返回，也作为批内所有行号的前缀，
        // 使 /mrp/results?calculation_no=<批次号> 能按前缀检索到整批行。
        // 批次号经生成器在事务内取号（advisory lock 持有到提交），批内全部行
        // INSERT 同事务——原「MRPB{毫秒时间戳}」直撞 calculation_no UNIQUE
        // （DDL 见文件头前缀常量注释）的写法废除，禁止退化回时间戳。
        let txn = (*self.db).begin().await?;

        for attempt in 1..=MRP_NO_CONFLICT_MAX_RETRY {
            // SeaORM 对已开启事务的 begin() 发 SAVEPOINT（口径同
            // utils/number_generator.rs::insert_with_no_retry）
            let sp = txn
                .begin()
                .await
                .map_err(|e| AppError::database(format!("创建MRP批次保存点失败: {e}")))?;
            let calculation_no = match DocumentNumberGenerator::generate_no_with_txn(
                &sp,
                MRP_BATCH_NO_PREFIX,
                MrpResultEntity,
                MrpResultColumn::CalculationNo,
            )
            .await
            {
                Ok(no) => no,
                Err(e) => {
                    // 取号失败：先回滚保存点（回滚失败只记日志，不得顶替原始错误），
                    // 原始错误按既有口径包装为可外显业务错误上抛
                    if let Err(re) = sp.rollback().await {
                        tracing::error!(
                            error = %re,
                            original_error = %e,
                            "回滚MRP批次取号保存点失败（取号原始错误继续上抛）"
                        );
                    }
                    tracing::error!(
                        error = %e,
                        prefix = MRP_BATCH_NO_PREFIX,
                        "MRP计算批次号取号失败（mrp_engine_ops/calculation.batch_calculate）"
                    );
                    let err = AppError::business_displayable("MRP计算批次号生成失败，请稍后重试");
                    return Err(err);
                }
            };
            match self.build_batch_rows(&sp, &request, &calculation_no).await {
                Ok(summary) => {
                    sp.commit()
                        .await
                        .map_err(|e| AppError::database(format!("释放MRP批次保存点失败: {e}")))?;
                    // 批次号取号与整批行 INSERT 同事务，提交时释放 advisory lock
                    txn.commit().await?;
                    return Ok(summary);
                }
                Err(err) => {
                    if let Err(re) = sp.rollback().await {
                        return Err(AppError::database(format!(
                            "回滚MRP批次取号保存点失败（原始插入错误: {err}）: {re}"
                        )));
                    }
                    if !is_calculation_no_conflict(&err) {
                        // 与单号无关的 SQL/业务错误（FK/CHECK/计算失败）重试无意义，原样上抛
                        return Err(err);
                    }
                    tracing::warn!(
                        prefix = MRP_BATCH_NO_PREFIX,
                        attempt,
                        doc_no = %calculation_no,
                        error = %err,
                        "MRP批次行号唯一约束冲突(23505)，保存点已回滚，重新取批次号重试"
                    );
                }
            }
        }
        Err(no_conflict_exhausted(MRP_BATCH_NO_PREFIX))
    }

    /// 在调用方（保存点）事务内完成一整批行号派生 + 计算 + INSERT + 汇总，
    /// 供 `batch_calculate` 的取号冲突重试整批重放；失败时整批随保存点回滚，
    /// 不残留半批数据。
    ///
    /// 行计算/汇总需求全部复用 `run_mrp_calculation_for_line`（其内部经 `explode_bom`
    /// 完成多级展开与损耗用量口径），本方法不得再写第二套展开或需求公式：
    /// 入参 `request.items` 逐行派生 `{批次号}-{行序}` 前缀，产出的落库行与需求行
    /// （父行 + BOM 子行）同源收集；汇总写入 `MrpCalculationSummary`，由
    /// `handlers/mrp_handler.rs::calculate_mrp` 序列化出参、落库行存 `mrp_results` 表。
    async fn build_batch_rows(
        &self,
        txn: &DatabaseTransaction,
        request: &MrpCalculationRequest,
        calculation_no: &str,
    ) -> Result<MrpCalculationSummary, AppError> {
        let mut all_results = Vec::new();
        let mut all_requirements = Vec::new();

        for (line_idx, item) in request.items.iter().enumerate() {
            let (rows, requirements) = self
                .run_mrp_calculation_for_line(
                    txn,
                    MrpCalculationQuery {
                        product_id: item.product_id,
                        required_quantity: item.required_quantity,
                        required_date: item.required_date,
                        source_type: request.source_type.clone(),
                        source_id: request.source_id,
                        consider_safety_stock: request.consider_safety_stock,
                        consider_in_transit: request.consider_in_transit,
                    },
                    &format!("{calculation_no}-{line_idx}"),
                )
                .await?;

            all_results.extend(rows);
            all_requirements.extend(requirements);
        }

        let items_with_shortage = all_requirements
            .iter()
            .filter(|r| r.shortage_quantity > Decimal::ZERO)
            .count() as i32;

        Ok(MrpCalculationSummary {
            calculation_no: calculation_no.to_string(),
            total_items: all_results.len() as i32,
            items_with_shortage,
            results: all_results,
            requirements: all_requirements,
        })
    }
}
