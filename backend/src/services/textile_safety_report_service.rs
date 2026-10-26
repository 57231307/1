#![allow(dead_code)]
//! 第三方检测报告服务（登记、完整性门、逐项目实测结果承载、报告级判定）
//!
//! 完整性门在受理侧执行：CMA 与 CNAS 资质位、六项要素（样品名称、材质成分、色号、
//! 检验依据、检测项目与结果、判定结论）、有效期、按批次一致性，任一不满足落 rejected
//! 并在 rejection_detail 逐条点名缺项代号（逗号串，取值域 `report_element_code`），
//! 不允许"先收后进"式静默通过。
//! 检验依据只做存在性与格式校验（完整标准号加类别形态，如 "GB 18401-2010 B类"），
//! 不反向写产品侧 execution_standard 自由文本列（裁定 5）。
use crate::models::status::{
    report_acceptance_status, report_element_code, submission_status, textile_fastness_grade,
    textile_flag_value, textile_item_kind, textile_safety_category, textile_verdict,
};
use crate::models::textile_report_item_result::{
    ActiveModel, Column, Entity as ReportResult, Model,
};
use crate::models::textile_safety_submission::{
    ActiveModel as SubmissionActiveModel, Column as SubColumn, Entity as Submission,
};
use crate::models::textile_safety_submission_item::{
    Column as ListItemColumn, Entity as SubmissionItem,
};
use crate::models::textile_test_item::{Column as ItemCodeColumn, Entity as TestItem, Model as TestItemModel};
use crate::models::textile_third_party_report::{
    ActiveModel as ReportActiveModel, Column as ReportColumn, Entity as ThirdPartyReport,
    Model as ReportModel,
};
use crate::services::textile_safety_limit_judge::{JudgeDims, MeasuredInput};
use crate::services::textile_safety_submission_service::TextileSafetySubmissionService;
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use std::sync::Arc;

/// 报告登记入参（六项要素与资质位与日期；受理态由门产出，请求体不得直达 accepted）。
pub struct RegisterReportReq {
    pub report_no: String,
    pub submission_id: i32,
    pub inspection_agency: String,
    pub has_cma: bool,
    pub has_cnas: bool,
    pub sample_name: Option<String>,
    pub material_composition: Option<String>,
    pub color_no: Option<String>,
    pub process_no: Option<String>,
    pub test_basis: Option<String>,
    pub conclusion: Option<String>,
    pub oeko_tex_class: Option<String>,
    pub issue_date: chrono::NaiveDate,
    pub valid_until: Option<chrono::NaiveDate>,
    pub report_file_url: Option<String>,
    pub created_by: i32,
}

/// 逐项目实测结果一行（结构化承载：存值、单位、检测方法；等级用 token 单源录入）。
pub struct ResultRowReq {
    pub item_code: String,
    /// 数值型实测值
    pub measured_value: Option<Decimal>,
    /// 等级型 token（"3-4" 等九档），service 按词表换算为半级编码数值存列；
    /// 与 measured_value 互斥，双给即拒
    pub measured_token: Option<String>,
    /// 感官与布尔 flag（none、detected）
    pub measured_text: Option<String>,
    pub measured_unit: Option<String>,
    pub test_method: Option<String>,
}

pub struct TextileSafetyReportService {
    db: Arc<DatabaseConnection>,
}

impl TextileSafetyReportService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 登记报告：新建一份第三方报告，受理态恒落 pending，送检单从 sent 自动推进 reported。
    ///
    /// 判据：受理态不接受请求体直填（pending 是唯一合法初值，accepted 只能由完整性门产出）；
    /// 挂单前提为送检单处于 sent 或 reported（复检追加第二份报告允许）；
    /// oeko_tex_class 若给值必须命中分级词表（登记位也要白名单，脏 token 拒收）。
    pub async fn register(&self, req: RegisterReportReq) -> Result<ReportModel, AppError> {
        let submission = Submission::find_by_id(req.submission_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| {
                AppError::not_found(format!("送检单 {} 不存在，报告必须挂真实送检单", req.submission_id))
            })?;
        if submission.status != submission_status::SENT
            && submission.status != submission_status::REPORTED
        {
            return Err(AppError::business(format!(
                "送检单 {} 处于 {} 态，仅已送检或已收报告态可登记报告",
                req.submission_id, submission.status
            )));
        }
        if req.report_no.trim().is_empty() {
            return Err(AppError::validation_displayable("报告编号不能为空"));
        }
        if let Some(class) = req.oeko_tex_class.as_deref() {
            if !crate::models::status::oeko_tex_class::is_valid(class) {
                return Err(AppError::validation_displayable(format!(
                    "OEKO-TEX 分级 {} 不在词表内（合法值 I、II、III、IV）",
                    class
                )));
            }
        }
        let now = chrono::Utc::now();
        let active = ReportActiveModel {
            report_no: ActiveValue::Set(req.report_no),
            submission_id: ActiveValue::Set(req.submission_id),
            inspection_agency: ActiveValue::Set(req.inspection_agency),
            has_cma: ActiveValue::Set(req.has_cma),
            has_cnas: ActiveValue::Set(req.has_cnas),
            sample_name: ActiveValue::Set(req.sample_name),
            material_composition: ActiveValue::Set(req.material_composition),
            color_no: ActiveValue::Set(req.color_no),
            process_no: ActiveValue::Set(req.process_no),
            test_basis: ActiveValue::Set(req.test_basis),
            conclusion: ActiveValue::Set(req.conclusion),
            oeko_tex_class: ActiveValue::Set(req.oeko_tex_class),
            issue_date: ActiveValue::Set(req.issue_date),
            valid_until: ActiveValue::Set(req.valid_until),
            acceptance_status: ActiveValue::Set(report_acceptance_status::PENDING.to_string()),
            rejection_detail: ActiveValue::Set(String::new()),
            report_file_url: ActiveValue::Set(req.report_file_url),
            created_by: ActiveValue::Set(req.created_by),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
            ..Default::default()
        };
        let saved = active.insert(self.db.as_ref()).await?;
        if submission.status == submission_status::SENT {
            let mut sub_active: SubmissionActiveModel = submission.into();
            sub_active.status = ActiveValue::Set(submission_status::REPORTED.to_string());
            sub_active.updated_at = ActiveValue::Set(now);
            sub_active.update(self.db.as_ref()).await?;
        }
        tracing::info!(
            report_id = saved.id,
            submission_id = saved.submission_id,
            operator = saved.created_by,
            "第三方报告已登记，待完整性门核对"
        );
        Ok(saved)
    }

    /// 修正报告要素：改后受理态复位 pending 并清空拒绝明细（要素变了必须重新过门）。
    pub async fn update(&self, id: i32, req: RegisterReportReq) -> Result<ReportModel, AppError> {
        let report = self.get(id).await?;
        let submission = Submission::find_by_id(report.submission_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::internal(format!("报告 {} 的送检单引用悬空", id)))?;
        if [
            submission_status::JUDGED,
            submission_status::CLOSED,
            submission_status::WITHDRAWN,
        ]
        .contains(&submission.status.as_str())
        {
            return Err(AppError::business(format!(
                "送检单 {} 已 {}，报告要素封账不可改（错误报告请走撤单重送）",
                submission.id, submission.status
            )));
        }
        if let Some(class) = req.oeko_tex_class.as_deref() {
            if !crate::models::status::oeko_tex_class::is_valid(class) {
                return Err(AppError::validation_displayable(format!(
                    "OEKO-TEX 分级 {} 不在词表内（合法值 I、II、III、IV）",
                    class
                )));
            }
        }
        let mut active: ReportActiveModel = report.into();
        active.report_no = ActiveValue::Set(req.report_no);
        active.inspection_agency = ActiveValue::Set(req.inspection_agency);
        active.has_cma = ActiveValue::Set(req.has_cma);
        active.has_cnas = ActiveValue::Set(req.has_cnas);
        active.sample_name = ActiveValue::Set(req.sample_name);
        active.material_composition = ActiveValue::Set(req.material_composition);
        active.color_no = ActiveValue::Set(req.color_no);
        active.process_no = ActiveValue::Set(req.process_no);
        active.test_basis = ActiveValue::Set(req.test_basis);
        active.conclusion = ActiveValue::Set(req.conclusion);
        active.oeko_tex_class = ActiveValue::Set(req.oeko_tex_class);
        active.issue_date = ActiveValue::Set(req.issue_date);
        active.valid_until = ActiveValue::Set(req.valid_until);
        active.report_file_url = ActiveValue::Set(req.report_file_url);
        // 复位受理态：改后的要素没有重新过门前不得携带 accepted 结论存活
        active.acceptance_status = ActiveValue::Set(report_acceptance_status::PENDING.to_string());
        active.rejection_detail = ActiveValue::Set(String::new());
        active.updated_at = ActiveValue::Set(chrono::Utc::now());
        Ok(active.update(self.db.as_ref()).await?)
    }

    pub async fn get(&self, id: i32) -> Result<ReportModel, AppError> {
        ThirdPartyReport::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::not_found(format!("第三方报告 {} 不存在", id)))
    }

    /// 按送检单列报告（新登记在前）。
    pub async fn list_by_submission(
        &self,
        submission_id: i32,
    ) -> Result<Vec<ReportModel>, AppError> {
        Ok(ThirdPartyReport::find()
            .filter(ReportColumn::SubmissionId.eq(submission_id))
            .order_by(ReportColumn::CreatedAt, QueryOrder::Desc)
            .all(self.db.as_ref())
            .await?)
    }

    pub async fn list_results(&self, report_id: i32) -> Result<Vec<Model>, AppError> {
        Ok(ReportResult::find()
            .filter(Column::ReportId.eq(report_id))
            .order_by(Column::ItemId, QueryOrder::Asc)
            .all(self.db.as_ref())
            .await?)
    }

    /// 完整性门：逐条点名缺项，全过落 accepted，任一缺落 rejected 并写 rejection_detail。
    ///
    /// 门的形状（与裁定六面对齐，缺项代号取值域 `report_element_code::ALL`）：
    /// 1. 资质位：has_cma 缺记 cma、has_cnas 缺记 cnas；
    /// 2. 六项要素：样品名称、材质成分、色号、检验依据（存在且格式为完整标准号加类别形态）、
    ///    检测项目与结果（结果行非空且每行都有存值或 flag）、判定结论；
    /// 3. 有效期：valid_until 缺失（无法核验即拒）或今天已晚于有效期记 expired；
    /// 4. 按批次：报告色号与工艺必须与送检单批次口径一致，报告侧工艺缺失即无法对账，
    ///    记 batch_mismatch；色号缺失已按要素记 color_no，不重复记。
    /// 传给谁：handler accept 端点（PUT /textile-reports/{id}/acceptance）。
    /// 返回：落库后的报告行；缺项非空即 rejected，缺项清单同时回给调用方展示。
    pub async fn run_acceptance_gate(
        &self,
        id: i32,
        operator: i32,
    ) -> Result<(ReportModel, Vec<String>), AppError> {
        let report = self.get(id).await?;
        let submission = Submission::find_by_id(report.submission_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::internal(format!("报告 {} 的送检单引用悬空", id)))?;
        let mut missing: Vec<String> = Vec::new();
        if !report.has_cma {
            missing.push(report_element_code::CMA.to_string());
        }
        if !report.has_cnas {
            missing.push(report_element_code::CNAS.to_string());
        }
        let blank = |v: &Option<String>| v.as_deref().map(|s| !s.trim().is_empty()) == Some(true);
        if !blank(&report.sample_name) {
            missing.push(report_element_code::SAMPLE_NAME.to_string());
        }
        if !blank(&report.material_composition) {
            missing.push(report_element_code::MATERIAL_COMPOSITION.to_string());
        }
        if !blank(&report.color_no) {
            missing.push(report_element_code::COLOR_NO.to_string());
        }
        if !report
            .test_basis
            .as_deref()
            .map(|s| test_basis_format_ok(s))
            .unwrap_or(false)
        {
            missing.push(report_element_code::TEST_BASIS.to_string());
        }
        let results = self.list_results(id).await?;
        if results.is_empty() {
            missing.push(report_element_code::ITEM_RESULTS.to_string());
        } else if results.iter().any(|r| {
            r.measured_value.is_none()
                && r.measured_text.as_deref().map(|t| !t.trim().is_empty()) != Some(true)
        }) {
            // 结果行存在但某行既无存值又无 flag，"检测项目与结果"要素实质不完整
            missing.push(report_element_code::ITEM_RESULTS.to_string());
        }
        if !blank(&report.conclusion) {
            missing.push(report_element_code::CONCLUSION.to_string());
        }
        match report.valid_until {
            None => missing.push(report_element_code::EXPIRED.to_string()),
            Some(vu) => {
                let today = chrono::Utc::now().date_naive();
                if today > vu {
                    missing.push(report_element_code::EXPIRED.to_string());
                }
            }
        }
        let color_claimed = blank(&report.color_no);
        let process_claimed = blank(&report.process_no);
        // 批次口径判据（成文，不留多重嵌套）：报告侧工艺必须给出且与送检单一致，
        // 否则无法按批次对账，点名 batch_mismatch；色号在报告"声称给出"时必须与送检单一致，
        // 色号整体缺失的情形已由六项要素门点名 color_no，批次维不重复记同一缺项。
        let process_value = report.process_no.as_deref().unwrap_or("").trim();
        let process_ok = process_claimed && process_value == submission.process_no.trim();
        let color_ok = if color_claimed {
            report.color_no.as_deref().unwrap_or("").trim() == submission.color_no.trim()
        } else {
            true
        };
        if !process_ok || !color_ok {
            missing.push(report_element_code::BATCH_MISMATCH.to_string());
        }
        let accepted = missing.is_empty();
        let mut active: ReportActiveModel = report.into();
        let now = chrono::Utc::now();
        active.acceptance_status = ActiveValue::Set(if accepted {
            report_acceptance_status::ACCEPTED.to_string()
        } else {
            report_acceptance_status::REJECTED.to_string()
        });
        active.rejection_detail = ActiveValue::Set(missing.join(","));
        active.updated_at = ActiveValue::Set(now);
        let saved = active.update(self.db.as_ref()).await?;
        tracing::info!(
            report_id = id,
            accepted,
            missing = missing.join("|"),
            operator,
            "第三方报告完整性门核对完成"
        );
        Ok((saved, missing))
    }

    /// 逐项目实测结果 upsert（结构化承载存值、单位、检测方法；按项目测量类型强校验）。
    ///
    /// 判据：项目必须命中主数据且在送检清单内（清单是"该检什么"的依据，报告结果不许夹带
    /// 清单外项目）；numeric 要 measured_value；grade 要 measured_token 并按词表换算半级
    /// 编码数值（token 与 value 双给即拒，单源）；sensory 与 boolean 要词表内 flag；
    /// 重录实测即复位 verdict 为 pending，旧结论不得跨输入版本存活。
    /// 传给谁：handler upsert_results 端点（POST /textile-reports/{id}/results）。
    pub async fn upsert_results(
        &self,
        report_id: i32,
        rows: Vec<ResultRowReq>,
        operator: i32,
    ) -> Result<Vec<Model>, AppError> {
        let report = self.get(report_id).await?;
        let list_items = SubmissionItem::find()
            .filter(ListItemColumn::SubmissionId.eq(report.submission_id))
            .all(self.db.as_ref())
            .await?;
        let mut out = Vec::new();
        for req in rows {
            let item = TestItem::find()
                .filter(ItemCodeColumn::ItemCode.eq(req.item_code.as_str()))
                .one(self.db.as_ref())
                .await?
                .ok_or_else(|| {
                    AppError::validation_displayable(format!(
                        "项目编码 {} 不在检测项目主数据内，结果行不许携带无主项目",
                        req.item_code
                    ))
                })?;
            if !list_items.iter().any(|l| l.item_id == item.id) {
                return Err(AppError::business(format!(
                    "项目 {}（{}）不在该送检单清单内；要检它先把它列入清单，结果行不许夹带",
                    item.item_code, item.item_name
                )));
            }
            let (value, text) = Self::normalize_measured(&item, &req)?;
            let existing = ReportResult::find()
                .filter(Column::ReportId.eq(report_id))
                .filter(Column::ItemId.eq(item.id))
                .one(self.db.as_ref())
                .await?;
            let now = chrono::Utc::now();
            let saved = match existing {
                Some(row) => {
                    let mut active: ActiveModel = row.into();
                    active.measured_value = ActiveValue::Set(value);
                    active.measured_text = ActiveValue::Set(text);
                    active.measured_unit = ActiveValue::Set(req.measured_unit);
                    active.test_method = ActiveValue::Set(req.test_method);
                    active.verdict = ActiveValue::Set(textile_verdict::PENDING.to_string());
                    active.judged_at = ActiveValue::Set(None);
                    active.updated_at = ActiveValue::Set(now);
                    active.update(self.db.as_ref()).await?
                }
                None => {
                    let active = ActiveModel {
                        report_id: ActiveValue::Set(report_id),
                        item_id: ActiveValue::Set(item.id),
                        measured_value: ActiveValue::Set(value),
                        measured_text: ActiveValue::Set(text),
                        measured_unit: ActiveValue::Set(req.measured_unit),
                        test_method: ActiveValue::Set(req.test_method),
                        verdict: ActiveValue::Set(textile_verdict::PENDING.to_string()),
                        judged_at: ActiveValue::Set(None),
                        created_at: ActiveValue::Set(now),
                        updated_at: ActiveValue::Set(now),
                        ..Default::default()
                    };
                    active.insert(self.db.as_ref()).await?
                }
            };
            out.push(saved);
        }
        tracing::info!(
            report_id,
            rows = out.len(),
            operator,
            "第三方报告逐项目实测结果已录入"
        );
        Ok(out)
    }

    /// 报告级判定：对全部结果行按限值矩阵出三态并写 verdict（na 行不需要实测输入）。
    ///
    /// 判据：报告必须已受理（rejected 或 pending 的报告不配产结论，先过门再判定）；
    /// 每行走判定器 `resolve_item_verdict`（四维取行、合取、分层），输入不足即整批报错
    /// （fail-visible，已判行保持，重跑幂等）。
    /// 传给谁：handler judge_report 端点（PUT /textile-reports/{id}/judgement）。
    pub async fn judge_report(&self, report_id: i32, operator: i32) -> Result<Vec<Model>, AppError> {
        let report = self.get(report_id).await?;
        if report.acceptance_status != report_acceptance_status::ACCEPTED {
            return Err(AppError::business(format!(
                "报告 {} 受理态为 {}，完整性门未放行前不得产出逐项目判定",
                report_id, report.acceptance_status
            )));
        }
        let submission = Submission::find_by_id(report.submission_id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::internal(format!("报告 {} 的送检单引用悬空", report_id)))?;
        let tags = TextileSafetySubmissionService::split_exemption_tags(
            submission.exemption_tags.as_deref(),
        )?;
        let dims = JudgeDims {
            safety_category: &submission.safety_category,
            product_form: &submission.product_form,
            condition_tags: &tags,
        };
        let rows = self.list_results(report_id).await?;
        if rows.is_empty() {
            return Err(AppError::business(format!(
                "报告 {} 没有结果行可判，请先录入逐项目实测结果",
                report_id
            )));
        }
        let sub_service = TextileSafetySubmissionService::new(self.db.clone());
        let mut out = Vec::new();
        for row in rows {
            let item = TestItem::find_by_id(row.item_id)
                .one(self.db.as_ref())
                .await?
                .ok_or_else(|| AppError::internal(format!("结果行 {} 的项目主数据缺失", row.id)))?;
            let measured = MeasuredInput {
                value: row.measured_value,
                text: row.measured_text.clone(),
            };
            let verdict = sub_service.resolve_item_verdict(&item, &dims, &measured).await?;
            let now = chrono::Utc::now();
            let mut active: ActiveModel = row.into();
            active.verdict = ActiveValue::Set(verdict);
            active.judged_at = ActiveValue::Set(Some(now));
            active.updated_at = ActiveValue::Set(now);
            out.push(active.update(self.db.as_ref()).await?);
        }
        tracing::info!(
            report_id,
            judged = out.len(),
            operator,
            "第三方报告逐项目判定完成"
        );
        Ok(out)
    }

    /// 按项目测量类型把录入请求归一为（存值、flag）二元组，校验全部前置在此。
    fn normalize_measured(
        item: &TestItemModel,
        req: &ResultRowReq,
    ) -> Result<(Option<Decimal>, Option<String>), AppError> {
        match item.item_kind.as_str() {
            textile_item_kind::NUMERIC => {
                if req.measured_token.is_some() {
                    return Err(AppError::validation_displayable(format!(
                        "项目 {}（{}）是数值型，不接受等级 token，请直接给实测数值",
                        item.item_code, item.item_name
                    )));
                }
                req.measured_value.ok_or_else(|| {
                    AppError::validation_displayable(format!(
                        "项目 {}（{}）需要实测数值，当前为空",
                        item.item_code, item.item_name
                    ))
                })?;
                Ok((req.measured_value, normalized_flag(req.measured_text.as_deref())?))
            }
            textile_item_kind::GRADE => {
                if req.measured_value.is_some() {
                    return Err(AppError::validation_displayable(format!(
                        "项目 {}（{}）是等级型，请录入等级 token（如 3-4），数值由词表半级编码换算，不许手填编码值",
                        item.item_code, item.item_name
                    )));
                }
                let token = req.measured_token.as_deref().ok_or_else(|| {
                    AppError::validation_displayable(format!(
                        "项目 {}（{}）需要等级 token（合法 {}）",
                        item.item_code,
                        item.item_name,
                        textile_fastness_grade::ALL.join(",")
                    ))
                })?;
                if !textile_fastness_grade::is_valid(token) {
                    return Err(AppError::validation_displayable(format!(
                        "等级 token {} 不在取值域（1 到 5 含四个半级：{}）",
                        token,
                        textile_fastness_grade::ALL.join(",")
                    )));
                }
                let encoded = textile_fastness_grade::encode(token)
                    .ok_or_else(|| AppError::internal("等级词表与编码表漂移"))?;
                let value: Decimal = encoded
                    .parse()
                    .map_err(|_| AppError::internal(format!("等级编码 {} 无法解析为数值", encoded)))?;
                Ok((Some(value), None))
            }
            textile_item_kind::SENSORY | textile_item_kind::BOOLEAN => {
                if req.measured_value.is_some() || req.measured_token.is_some() {
                    return Err(AppError::validation_displayable(format!(
                        "项目 {}（{}）是感官或布尔证据型，只接受 flag（{} 或 {}），不接受数值",
                        item.item_code,
                        item.item_name,
                        textile_flag_value::NONE,
                        textile_flag_value::DETECTED
                    )));
                }
                let flag = normalized_flag(req.measured_text.as_deref())?.ok_or_else(|| {
                    AppError::validation_displayable(format!(
                        "项目 {}（{}）需要实测 flag（{} 或 {}），当前为空",
                        item.item_code,
                        item.item_name,
                        textile_flag_value::NONE,
                        textile_flag_value::DETECTED
                    ))
                })?;
                if flag != textile_flag_value::NONE && flag != textile_flag_value::DETECTED {
                    return Err(AppError::validation_displayable(format!(
                        "项目 {}（{}）的 flag 只能是 {} 或 {}（其它文字是备注不是判定输入）",
                        item.item_code,
                        item.item_name,
                        textile_flag_value::NONE,
                        textile_flag_value::DETECTED
                    )));
                }
                Ok((None, Some(flag)))
            }
            other => Err(AppError::internal(format!(
                "项目 {} 的测量类型 {} 不在词表内，主数据被旁路写坏",
                item.item_code, other
            ))),
        }
    }
}

/// 空白 flag 归一（空串按未给处理，非空逐字符透传给类型分派校验）。
fn normalized_flag(raw: Option<&str>) -> Result<Option<String>, AppError> {
    Ok(match raw {
        Some(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
        _ => None,
    })
}

/// 检验依据格式判据：必须含完整国标号（GB 18401-2010 或 GB 31701-2015，带空格的原文形态），
/// 凡引用 GB 18401 必须同时给出类别形态（A类、B类、C类，来自词表 display_label 换算）。
///
/// 判据来源是裁定 5："按 GB 18401-2010 B类 这类完整标准号加类别形态做存在性与格式校验"；
/// 只做本列自身的形态校验，不反向写产品侧 execution_standard 自由文本列。
pub fn test_basis_format_ok(raw: &str) -> bool {
    let s = raw.trim();
    if s.is_empty() {
        return false;
    }
    let cites_18401 = s.contains("GB 18401-2010");
    let cites_31701 = s.contains("GB 31701-2015");
    if !cites_18401 && !cites_31701 {
        return false;
    }
    if cites_18401 {
        // 引用基本安全标准必须带安全类别展示形态（展示文本由稳定码经词表换算，同一字典）
        let has_class = [
            textile_safety_category::CLASS_A,
            textile_safety_category::CLASS_B,
            textile_safety_category::CLASS_C,
        ]
        .iter()
        .any(|code| {
            textile_safety_category::display_label(code)
                .map(|label| s.contains(label))
                .unwrap_or(false)
        });
        if !has_class {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::test_basis_format_ok;

    /// 正向：完整标准号加类别形态（裁定原文示例）必须放行。
    #[test]
    fn accepts_full_basis_with_class() {
        assert!(test_basis_format_ok("GB 18401-2010 B类"));
        assert!(test_basis_format_ok("检验依据 GB 18401-2010 A类、GB 31701-2015"));
    }

    /// 反向：缺类别、裸标准号、缺年份、错空格形态一律拒绝（钉校验强度而非恒真）。
    #[test]
    fn rejects_incomplete_basis() {
        assert!(!test_basis_format_ok("GB 18401-2010"));
        assert!(!test_basis_format_ok("GB18401 B类"));
        assert!(!test_basis_format_ok("GB 18401 B类"));
        assert!(!test_basis_format_ok(""));
        assert!(!test_basis_format_ok("企标 Q/XX 01-2020"));
    }

    /// 仅引用 GB 31701（童装强制标准本身无类别档）放行，钉住两标准差异化判据。
    #[test]
    fn accepts_31701_alone() {
        assert!(test_basis_format_ok("GB 31701-2015"));
    }
}
