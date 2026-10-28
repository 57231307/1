//! 纺织品安全合规域主数据服务（检测项目、限值判定表维护通道、出口市场要求登记）
//!
//! 判定表可维护是裁定的硬要求：限值行的新增修改删除都经本服务做词表与形态校验，
//! 校验失败 fail-visible 拒绝，不给旁路留"先写脏再靠判定器兜底"的空间。
//! 删除防护：主数据被业务行引用时 RESTRICT 会撞库，本服务先行点名计数并给业务错误，
//! 不把 23503 裸抛给调用方。
use crate::models::status::{
    export_market, export_requirement_type, oeko_tex_class, textile_basis_standard,
    textile_check_kind, textile_comparator, textile_item_kind, textile_limit_scope,
};
use crate::models::textile_child_safety_check::Column as CheckColumn;
use crate::models::textile_child_safety_check::Entity as ChildCheck;
use crate::models::textile_export_market_requirement::{
    ActiveModel as ReqActiveModel, Column as ReqColumn, Entity as ExportRequirement,
    Model as ReqModel,
};
use crate::models::textile_limit_rule::{
    ActiveModel as LimitActiveModel, Column as LimitColumn, Entity as LimitRule,
    Model as LimitModel,
};
use crate::models::textile_report_item_result::Column as ResultColumn;
use crate::models::textile_report_item_result::Entity as ReportResult;
use crate::models::textile_safety_submission_item::Column as ListItemColumn;
use crate::models::textile_safety_submission_item::Entity as SubmissionItem;
use crate::models::textile_test_item::{
    ActiveModel as ItemActiveModel, Column as ItemColumn, Entity as TestItem, Model as ItemModel,
};
use crate::utils::error::AppError;
use rust_decimal::Decimal;
use sea_orm::{
    ActiveModelTrait, ActiveValue, ColumnTrait, DatabaseConnection, EntityTrait, Order,
    PaginatorTrait, QueryFilter, QueryOrder,
};
use std::sync::Arc;

/// 检测项目写入参。
pub struct TestItemReq {
    pub item_code: String,
    pub item_name: String,
    pub item_kind: String,
    pub basis_standard: String,
    pub check_kind: String,
    pub method_standard: Option<String>,
    pub unit: Option<String>,
    pub description: String,
}

/// 限值行写入参。
pub struct LimitRuleReq {
    pub item_code: String,
    pub safety_category: String,
    pub product_form: String,
    pub condition_tag: String,
    pub comparator: String,
    pub limit_low: Option<Decimal>,
    pub limit_high: Option<Decimal>,
    pub clause_ref: String,
    pub remark: String,
}

/// 出口要求写入参。
pub struct ExportRequirementReq {
    pub requirement_code: String,
    pub market: String,
    pub requirement_type: String,
    pub oeko_tex_class: Option<String>,
    pub standard_ref: String,
    pub detail: String,
    pub mandatory: bool,
}

pub struct TextileSafetyMasterService {
    db: Arc<DatabaseConnection>,
}

impl TextileSafetyMasterService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    // ===== 检测项目主数据 =====

    pub async fn list_test_items(&self, basis: Option<&str>) -> Result<Vec<ItemModel>, AppError> {
        let mut query = TestItem::find();
        if let Some(basis) = basis {
            if !textile_basis_standard::is_valid(basis) {
                return Err(AppError::validation_displayable(format!(
                    "标准分组 {} 不在词表内（合法值 {}）",
                    basis,
                    textile_basis_standard::ALL.join(",")
                )));
            }
            query = query.filter(ItemColumn::BasisStandard.eq(basis));
        }
        Ok(query
            .order_by(ItemColumn::ItemCode, Order::Asc)
            .all(self.db.as_ref())
            .await?)
    }

    pub async fn create_test_item(&self, req: TestItemReq) -> Result<ItemModel, AppError> {
        Self::validate_test_item(&req)?;
        let dup = TestItem::find()
            .filter(ItemColumn::ItemCode.eq(req.item_code.as_str()))
            .count(self.db.as_ref())
            .await?;
        if dup > 0 {
            return Err(AppError::business(format!(
                "项目编码 {} 已存在，主数据编码是唯一查找键，改值请走修改通道",
                req.item_code
            )));
        }
        let now = chrono::Utc::now();
        let active = ItemActiveModel {
            item_code: ActiveValue::Set(req.item_code),
            item_name: ActiveValue::Set(req.item_name),
            item_kind: ActiveValue::Set(req.item_kind),
            basis_standard: ActiveValue::Set(req.basis_standard),
            check_kind: ActiveValue::Set(req.check_kind),
            method_standard: ActiveValue::Set(req.method_standard),
            unit: ActiveValue::Set(req.unit),
            description: ActiveValue::Set(req.description),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
            ..Default::default()
        };
        Ok(active.insert(self.db.as_ref()).await?)
    }

    pub async fn update_test_item(&self, id: i32, req: TestItemReq) -> Result<ItemModel, AppError> {
        Self::validate_test_item(&req)?;
        let model = TestItem::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::not_found(format!("检测项目 {} 不存在", id)))?;
        // 编码是判定链与清单的查找键，修改即断链：编码封账，其余字段可修
        if req.item_code != model.item_code {
            return Err(AppError::business(format!(
                "项目编码是稳定查找键（{} 不可改为 {}），确需换编码请停用旧项新建",
                model.item_code, req.item_code
            )));
        }
        let mut active: ItemActiveModel = model.into();
        active.item_name = ActiveValue::Set(req.item_name);
        active.item_kind = ActiveValue::Set(req.item_kind);
        active.basis_standard = ActiveValue::Set(req.basis_standard);
        active.check_kind = ActiveValue::Set(req.check_kind);
        active.method_standard = ActiveValue::Set(req.method_standard);
        active.unit = ActiveValue::Set(req.unit);
        active.description = ActiveValue::Set(req.description);
        active.updated_at = ActiveValue::Set(chrono::Utc::now());
        Ok(active.update(self.db.as_ref()).await?)
    }

    /// 删除检测项目：被任何业务面引用即拒绝并点名引用数（FK RESTRICT 的先行人道版）。
    pub async fn delete_test_item(&self, id: i32) -> Result<(), AppError> {
        let limits = LimitRule::find()
            .filter(LimitColumn::ItemId.eq(id))
            .count(self.db.as_ref())
            .await?;
        let list_rows = SubmissionItem::find()
            .filter(ListItemColumn::ItemId.eq(id))
            .count(self.db.as_ref())
            .await?;
        let results = ReportResult::find()
            .filter(ResultColumn::ItemId.eq(id))
            .count(self.db.as_ref())
            .await?;
        let checks = ChildCheck::find()
            .filter(CheckColumn::ItemId.eq(id))
            .count(self.db.as_ref())
            .await?;
        if limits + list_rows + results + checks > 0 {
            return Err(AppError::business(format!(
                "检测项目 {} 被引用（限值行 {}、送检清单行 {}、报告结果行 {}、童装核查行 {}），禁止删除；要停用请删除其限值行使其不可判定并显式报错",
                id, limits, list_rows, results, checks
            )));
        }
        TestItem::delete_by_id(id).exec(self.db.as_ref()).await?;
        Ok(())
    }

    // ===== 限值判定表 =====

    pub async fn list_limit_rules(
        &self,
        item_id: Option<i32>,
        safety_category: Option<&str>,
    ) -> Result<Vec<LimitModel>, AppError> {
        let mut query = LimitRule::find();
        if let Some(item_id) = item_id {
            query = query.filter(LimitColumn::ItemId.eq(item_id));
        }
        if let Some(cat) = safety_category {
            if !textile_limit_scope::CATEGORIES.contains(&cat) {
                return Err(AppError::validation_displayable(format!(
                    "安全类别 {} 不在判定表取值域（含通配 any）",
                    cat
                )));
            }
            query = query.filter(LimitColumn::SafetyCategory.eq(cat));
        }
        Ok(query
            .order_by(LimitColumn::ItemId, Order::Asc)
            .order_by(LimitColumn::SafetyCategory, Order::Asc)
            .all(self.db.as_ref())
            .await?)
    }

    /// 新增限值行（四维定位加比较器唯一；形态完备性先行校验）。
    pub async fn create_limit_rule(&self, req: LimitRuleReq) -> Result<LimitModel, AppError> {
        let item = Self::require_item_by_code(self.db.as_ref(), &req.item_code).await?;
        Self::validate_limit_dims(&req)?;
        Self::validate_limit_shape(&req)?;
        let dup = LimitRule::find()
            .filter(LimitColumn::ItemId.eq(item.id))
            .filter(LimitColumn::SafetyCategory.eq(req.safety_category.as_str()))
            .filter(LimitColumn::ProductForm.eq(req.product_form.as_str()))
            .filter(LimitColumn::ConditionTag.eq(req.condition_tag.as_str()))
            .filter(LimitColumn::Comparator.eq(req.comparator.as_str()))
            .count(self.db.as_ref())
            .await?;
        if dup > 0 {
            return Err(AppError::business(format!(
                "同四维加比较器的限值行已存在（项目 {}），修订请走更新通道",
                req.item_code
            )));
        }
        let now = chrono::Utc::now();
        let active = LimitActiveModel {
            item_id: ActiveValue::Set(item.id),
            safety_category: ActiveValue::Set(req.safety_category),
            product_form: ActiveValue::Set(req.product_form),
            condition_tag: ActiveValue::Set(req.condition_tag),
            comparator: ActiveValue::Set(req.comparator),
            limit_low: ActiveValue::Set(req.limit_low),
            limit_high: ActiveValue::Set(req.limit_high),
            clause_ref: ActiveValue::Set(req.clause_ref),
            remark: ActiveValue::Set(req.remark),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
            ..Default::default()
        };
        Ok(active.insert(self.db.as_ref()).await?)
    }

    /// 修订限值行（按行 id 定位，改后仍受同一套形态与唯一性校验）。
    pub async fn update_limit_rule(
        &self,
        id: i32,
        req: LimitRuleReq,
    ) -> Result<LimitModel, AppError> {
        let row = LimitRule::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::not_found(format!("限值行 {} 不存在", id)))?;
        let item = Self::require_item_by_code(self.db.as_ref(), &req.item_code).await?;
        Self::validate_limit_dims(&req)?;
        Self::validate_limit_shape(&req)?;
        let dup = LimitRule::find()
            .filter(LimitColumn::ItemId.eq(item.id))
            .filter(LimitColumn::SafetyCategory.eq(req.safety_category.as_str()))
            .filter(LimitColumn::ProductForm.eq(req.product_form.as_str()))
            .filter(LimitColumn::ConditionTag.eq(req.condition_tag.as_str()))
            .filter(LimitColumn::Comparator.eq(req.comparator.as_str()))
            .filter(LimitColumn::Id.ne(id))
            .count(self.db.as_ref())
            .await?;
        if dup > 0 {
            return Err(AppError::business(
                "目标四维加比较器组合已被另一行占用，判定表四维定位必须唯一",
            ));
        }
        let mut active: LimitActiveModel = row.into();
        active.item_id = ActiveValue::Set(item.id);
        active.safety_category = ActiveValue::Set(req.safety_category);
        active.product_form = ActiveValue::Set(req.product_form);
        active.condition_tag = ActiveValue::Set(req.condition_tag);
        active.comparator = ActiveValue::Set(req.comparator);
        active.limit_low = ActiveValue::Set(req.limit_low);
        active.limit_high = ActiveValue::Set(req.limit_high);
        active.clause_ref = ActiveValue::Set(req.clause_ref);
        active.remark = ActiveValue::Set(req.remark);
        active.updated_at = ActiveValue::Set(chrono::Utc::now());
        Ok(active.update(self.db.as_ref()).await?)
    }

    pub async fn delete_limit_rule(&self, id: i32) -> Result<(), AppError> {
        LimitRule::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::not_found(format!("限值行 {} 不存在", id)))?;
        LimitRule::delete_by_id(id).exec(self.db.as_ref()).await?;
        Ok(())
    }

    // ===== 出口市场要求登记 =====

    pub async fn list_export_requirements(
        &self,
        market: Option<&str>,
    ) -> Result<Vec<ReqModel>, AppError> {
        let mut query = ExportRequirement::find();
        if let Some(market) = market {
            // 登记面只收出口四市场（内销 dom 不挂出口要求，与 CHECK 取值域同源）
            if !export_market::is_valid(market) || market == export_market::DOM {
                return Err(AppError::validation_displayable(format!(
                    "市场 {} 不是可登记的出口市场（合法值 eu、us、jp、other）",
                    market
                )));
            }
            query = query.filter(ReqColumn::Market.eq(market));
        }
        Ok(query
            .order_by(ReqColumn::Market, Order::Asc)
            .order_by(ReqColumn::RequirementCode, Order::Asc)
            .all(self.db.as_ref())
            .await?)
    }

    pub async fn create_export_requirement(
        &self,
        req: ExportRequirementReq,
    ) -> Result<ReqModel, AppError> {
        Self::validate_requirement(&req)?;
        let dup = ExportRequirement::find()
            .filter(ReqColumn::RequirementCode.eq(req.requirement_code.as_str()))
            .count(self.db.as_ref())
            .await?;
        if dup > 0 {
            return Err(AppError::business(format!(
                "要求编码 {} 已登记，登记面编码唯一",
                req.requirement_code
            )));
        }
        let now = chrono::Utc::now();
        let active = ReqActiveModel {
            requirement_code: ActiveValue::Set(req.requirement_code),
            market: ActiveValue::Set(req.market),
            requirement_type: ActiveValue::Set(req.requirement_type),
            oeko_tex_class: ActiveValue::Set(req.oeko_tex_class),
            standard_ref: ActiveValue::Set(req.standard_ref),
            detail: ActiveValue::Set(req.detail),
            mandatory: ActiveValue::Set(req.mandatory),
            created_at: ActiveValue::Set(now),
            updated_at: ActiveValue::Set(now),
            ..Default::default()
        };
        Ok(active.insert(self.db.as_ref()).await?)
    }

    pub async fn update_export_requirement(
        &self,
        id: i32,
        req: ExportRequirementReq,
    ) -> Result<ReqModel, AppError> {
        Self::validate_requirement(&req)?;
        let row = ExportRequirement::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::not_found(format!("出口要求 {} 不存在", id)))?;
        let mut active: ReqActiveModel = row.into();
        active.requirement_code = ActiveValue::Set(req.requirement_code);
        active.market = ActiveValue::Set(req.market);
        active.requirement_type = ActiveValue::Set(req.requirement_type);
        active.oeko_tex_class = ActiveValue::Set(req.oeko_tex_class);
        active.standard_ref = ActiveValue::Set(req.standard_ref);
        active.detail = ActiveValue::Set(req.detail);
        active.mandatory = ActiveValue::Set(req.mandatory);
        active.updated_at = ActiveValue::Set(chrono::Utc::now());
        Ok(active.update(self.db.as_ref()).await?)
    }

    pub async fn delete_export_requirement(&self, id: i32) -> Result<(), AppError> {
        ExportRequirement::find_by_id(id)
            .one(self.db.as_ref())
            .await?
            .ok_or_else(|| AppError::not_found(format!("出口要求 {} 不存在", id)))?;
        ExportRequirement::delete_by_id(id)
            .exec(self.db.as_ref())
            .await?;
        Ok(())
    }

    // ===== 校验组（词表单源，逐字符白名单，不夹紧不兜底） =====

    fn validate_test_item(req: &TestItemReq) -> Result<(), AppError> {
        if req.item_code.trim().is_empty() || req.item_name.trim().is_empty() {
            return Err(AppError::validation_displayable("项目编码与名称不能为空"));
        }
        if !textile_item_kind::is_valid(&req.item_kind) {
            return Err(AppError::validation_displayable(format!(
                "测量类型 {} 不在词表内（合法值 {}）",
                req.item_kind,
                textile_item_kind::ALL.join(",")
            )));
        }
        if !textile_basis_standard::is_valid(&req.basis_standard) {
            return Err(AppError::validation_displayable(format!(
                "标准分组 {} 不在词表内（合法值 {}）",
                req.basis_standard,
                textile_basis_standard::ALL.join(",")
            )));
        }
        if !textile_check_kind::is_valid(&req.check_kind) {
            return Err(AppError::validation_displayable(format!(
                "核查方式 {} 不在词表内（合法值 {}）",
                req.check_kind,
                textile_check_kind::ALL.join(",")
            )));
        }
        Ok(())
    }

    fn validate_limit_dims(req: &LimitRuleReq) -> Result<(), AppError> {
        if !textile_limit_scope::CATEGORIES.contains(&req.safety_category.as_str()) {
            return Err(AppError::validation_displayable(format!(
                "安全类别维 {} 不在判定表取值域（{}）",
                req.safety_category,
                textile_limit_scope::CATEGORIES.join(",")
            )));
        }
        if !textile_limit_scope::FORMS.contains(&req.product_form.as_str()) {
            return Err(AppError::validation_displayable(format!(
                "产品形态维 {} 不在判定表取值域（{}）",
                req.product_form,
                textile_limit_scope::FORMS.join(",")
            )));
        }
        if !textile_limit_scope::CONDITION_TAGS.contains(&req.condition_tag.as_str()) {
            return Err(AppError::validation_displayable(format!(
                "条件标志维 {} 不在判定表取值域（{}）",
                req.condition_tag,
                textile_limit_scope::CONDITION_TAGS.join(",")
            )));
        }
        if !textile_comparator::is_valid(&req.comparator) {
            return Err(AppError::validation_displayable(format!(
                "比较器 {} 不在词表内（合法值 {}）",
                req.comparator,
                textile_comparator::ALL.join(",")
            )));
        }
        Ok(())
    }

    /// 比较器与上下界的形态完备性（数据进表前先自洽，判定器撞"数据不完整"是最后防线不是常规）。
    fn validate_limit_shape(req: &LimitRuleReq) -> Result<(), AppError> {
        use textile_comparator as cmp;
        match req.comparator.as_str() {
            cmp::LTE if req.limit_high.is_none() => Err(AppError::validation_displayable(
                "lte 行必须给出 limit_high 上限",
            )),
            cmp::GTE if req.limit_low.is_none() => Err(AppError::validation_displayable(
                "gte 行必须给出 limit_low 下限（等级型给半级编码数值）",
            )),
            cmp::BETWEEN if req.limit_low.is_none() || req.limit_high.is_none() => Err(
                AppError::validation_displayable("between 行必须同时给出上下界"),
            ),
            cmp::FORBIDDEN | cmp::MUST_NONE | cmp::NOT_APPLICABLE => {
                if req.limit_low.is_some() || req.limit_high.is_some() {
                    return Err(AppError::validation_displayable(
                        "禁用、必须为无、不考核三型不消费数值界，给了只会误导维护",
                    ));
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn validate_requirement(req: &ExportRequirementReq) -> Result<(), AppError> {
        if req.requirement_code.trim().is_empty() || req.detail.trim().is_empty() {
            return Err(AppError::validation_displayable(
                "要求编码与细则说明不能为空",
            ));
        }
        if !export_market::is_valid(&req.market) || req.market == export_market::DOM {
            return Err(AppError::validation_displayable(format!(
                "市场 {} 非法；内销 dom 不挂出口要求（合法登记市场 eu、us、jp、other）",
                req.market
            )));
        }
        if !export_requirement_type::is_valid(&req.requirement_type) {
            return Err(AppError::validation_displayable(format!(
                "要求类型 {} 不在词表内（合法值 {}）",
                req.requirement_type,
                export_requirement_type::ALL.join(",")
            )));
        }
        if let Some(class) = req.oeko_tex_class.as_deref() {
            if !oeko_tex_class::is_valid(class) {
                return Err(AppError::validation_displayable(format!(
                    "OEKO-TEX 分级 {} 不在词表内（合法值 I、II、III、IV）",
                    class
                )));
            }
        }
        Ok(())
    }

    async fn require_item_by_code(
        db: &DatabaseConnection,
        code: &str,
    ) -> Result<ItemModel, AppError> {
        TestItem::find()
            .filter(ItemColumn::ItemCode.eq(code))
            .one(db)
            .await?
            .ok_or_else(|| {
                AppError::validation_displayable(format!(
                    "项目编码 {} 不在检测项目主数据内，限值行必须有主数据项目挂靠",
                    code
                ))
            })
    }
}
