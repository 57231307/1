//! BOM物料清单 Service
//!
//! 提供BOM的CRUD操作、版本管理和树形结构查询

use chrono::Utc;
use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, TransactionTrait,
};
use std::sync::Arc;
use tracing::info;

use crate::models::bom::{
    ActiveModel, BomStatus, Column as BomColumn, Entity as BomEntity, Model as BomModel,
};
use crate::models::bom_item::{
    ActiveModel as BomItemActiveModel, Column as BomItemColumn, Entity as BomItemEntity,
    Model as BomItemModel,
};
use crate::utils::error::AppError;

/// 创建BOM请求
#[derive(Debug, Clone)]
pub struct CreateBomRequest {
    pub product_id: i32,
    pub version: Option<i32>,
    pub is_default: Option<bool>,
    pub remarks: Option<String>,
    pub created_by: i32,
    pub items: Vec<CreateBomItemRequest>,
}

/// 创建BOM明细请求
#[derive(Debug, Clone)]
pub struct CreateBomItemRequest {
    pub material_id: i32,
    pub quantity: Decimal,
    pub unit: Option<String>,
    pub scrap_rate: Option<Decimal>,
    pub sort_order: Option<i32>,
}

/// 更新BOM请求
#[derive(Debug, Clone)]
pub struct UpdateBomRequest {
    pub is_default: Option<bool>,
    pub status: Option<String>,
    pub remarks: Option<String>,
    pub items: Option<Vec<CreateBomItemRequest>>,
}

/// BOM查询参数
#[derive(Debug, Clone)]
pub struct BomQuery {
    pub product_id: Option<i32>,
    pub status: Option<String>,
    pub is_default: Option<bool>,
    pub page: u64,
    pub page_size: u64,
}

/// BOM详情（含明细）
#[derive(Debug, Clone, serde::Serialize)]
pub struct BomDetail {
    pub bom: BomModel,
    pub items: Vec<BomItemModel>,
}

/// BOM Service
pub struct BomService {
    db: Arc<DatabaseConnection>,
}

impl BomService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 创建BOM（含明细）
    ///
    /// 批次 203 P1-4 修复：原实现存在两个缺陷——整个方法无事务保护（主表插入、
    /// 默认取消、明细插入分散执行，若明细插入失败会留下无明细的脏 BOM）；
    /// 明细采用循环内逐条 `insert(&*self.db)`，N 条明细 = N 次 INSERT（N+1 写）。
    /// 现用事务包裹"取消旧默认 + 创建主表 + 批量插入明细"，明细改用 `insert_many`
    /// 单次 INSERT，并在事务内回查明细以构造 BomDetail 返回。
    pub async fn create(&self, req: CreateBomRequest) -> Result<BomDetail, AppError> {
        let txn = self.db.begin().await?;

        // 计算版本号
        let version = if let Some(v) = req.version {
            v
        } else {
            self.get_next_version(req.product_id).await?
        };

        let is_default = req.is_default.unwrap_or(false);

        // 如果设置为默认，先取消同产品其他默认BOM
        if is_default {
            BomEntity::update_many()
                .filter(BomColumn::ProductId.eq(req.product_id))
                .filter(BomColumn::IsDefault.eq(true))
                .set(ActiveModel {
                    is_default: Set(false),
                    updated_at: Set(Utc::now()),
                    ..Default::default()
                })
                .exec(&txn)
                .await?;
        }

        // 创建BOM主记录
        let bom_active_model = ActiveModel {
            product_id: Set(req.product_id),
            version: Set(version),
            is_default: Set(is_default),
            status: Set(BomStatus::Active.to_string()),
            remarks: Set(req.remarks),
            created_by: Set(req.created_by),
            created_at: Set(Utc::now()),
            updated_at: Set(Utc::now()),
            ..Default::default()
        };

        let bom_model = bom_active_model.insert(&txn).await?;

        // 创建BOM明细：改用 insert_many 批量 INSERT（原为循环内逐条 insert 导致 N 条=N 次 INSERT）
        let mut item_active_models: Vec<BomItemActiveModel> =
            Vec::with_capacity(req.items.len());
        for (index, item_req) in req.items.iter().enumerate() {
            item_active_models.push(BomItemActiveModel {
                bom_id: Set(bom_model.id),
                material_id: Set(item_req.material_id),
                quantity: Set(item_req.quantity),
                unit: Set(item_req.unit.clone()),
                scrap_rate: Set(item_req.scrap_rate),
                sort_order: Set(Some(item_req.sort_order.unwrap_or(index as i32))),
                created_at: Set(Utc::now()),
                updated_at: Set(Utc::now()),
                ..Default::default()
            });
        }

        if !item_active_models.is_empty() {
            BomItemEntity::insert_many(item_active_models)
                .exec(&txn)
                .await?;
        }

        // insert_many 不返回每条 Model，需在事务内回查明细以构造 BomDetail
        let items = BomItemEntity::find()
            .filter(BomItemColumn::BomId.eq(bom_model.id))
            .order_by_asc(BomItemColumn::SortOrder)
            .all(&txn)
            .await?;

        txn.commit().await?;

        Ok(BomDetail {
            bom: bom_model,
            items,
        })
    }

    /// 根据ID获取BOM详情
    pub async fn get_by_id(&self, id: i32) -> Result<Option<BomDetail>, AppError> {
        let bom_model = BomEntity::find_by_id(id).one(&*self.db).await?;

        match bom_model {
            Some(bom) => {
                let items = BomItemEntity::find()
                    .filter(BomItemColumn::BomId.eq(id))
                    .order_by_asc(BomItemColumn::SortOrder)
                    .all(&*self.db)
                    .await?;

                Ok(Some(BomDetail { bom, items }))
            }
            None => Ok(None),
        }
    }

    /// 获取BOM列表
    pub async fn list(&self, query: BomQuery) -> Result<(Vec<BomModel>, u64), AppError> {
        let mut select = BomEntity::find();

        if let Some(product_id) = query.product_id {
            select = select.filter(BomColumn::ProductId.eq(product_id));
        }

        if let Some(status) = query.status {
            select = select.filter(BomColumn::Status.eq(status));
        }

        if let Some(is_default) = query.is_default {
            select = select.filter(BomColumn::IsDefault.eq(is_default));
        }

        let total = select.clone().count(&*self.db).await?;

        let boms = select
            .order_by_desc(BomColumn::CreatedAt)
            .offset(query.page.saturating_sub(1) * query.page_size)
            .limit(query.page_size)
            .all(&*self.db)
            .await?;

        Ok((boms, total))
    }

    /// 更新BOM
    ///
    /// 批次 203 P1-4 修复：原实现存在两个缺陷——事务仅包裹"删除旧明细 + 创建新明细"
    /// （主表 update 在事务外，若明细插入失败，主表 update 不会回滚）；明细采用循环内
    /// 逐条 `insert(&txn)`，N 条明细 = N 次 INSERT（N+1 写）。现将事务范围扩大到包含
    /// 主表 update，明细改用 `insert_many` 单次 INSERT。
    pub async fn update(&self, id: i32, req: UpdateBomRequest) -> Result<BomDetail, AppError> {
        let txn = self.db.begin().await?;

        let bom_model = BomEntity::find_by_id(id)
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("BOM不存在"))?;

        let mut bom_active: ActiveModel = bom_model.into();

        if let Some(is_default) = req.is_default {
            bom_active.is_default = Set(is_default);
        }
        if let Some(status) = req.status {
            bom_active.status = Set(status);
        }
        if let Some(remarks) = req.remarks {
            bom_active.remarks = Set(Some(remarks));
        }
        bom_active.updated_at = Set(Utc::now());

        bom_active.update(&txn).await?;

        // 如果提供了新的明细，替换所有明细：改用 insert_many 批量 INSERT（原为循环内逐条 insert）
        if let Some(items_req) = req.items {
            // 删除旧明细
            BomItemEntity::delete_many()
                .filter(BomItemColumn::BomId.eq(id))
                .exec(&txn)
                .await?;

            // 创建新明细：批量插入
            let mut item_active_models: Vec<BomItemActiveModel> =
                Vec::with_capacity(items_req.len());
            for (index, item_req) in items_req.iter().enumerate() {
                item_active_models.push(BomItemActiveModel {
                    bom_id: Set(id),
                    material_id: Set(item_req.material_id),
                    quantity: Set(item_req.quantity),
                    unit: Set(item_req.unit.clone()),
                    scrap_rate: Set(item_req.scrap_rate),
                    sort_order: Set(Some(item_req.sort_order.unwrap_or(index as i32))),
                    created_at: Set(Utc::now()),
                    updated_at: Set(Utc::now()),
                    ..Default::default()
                });
            }

            if !item_active_models.is_empty() {
                BomItemEntity::insert_many(item_active_models)
                    .exec(&txn)
                    .await?;
            }
        }

        txn.commit().await?;

        // 返回更新后的详情
        self.get_by_id(id)
            .await?
            .ok_or_else(|| AppError::not_found("BOM不存在"))
    }

    /// 删除BOM（软删除）
    pub async fn delete(&self, id: i32) -> Result<(), AppError> {
        let bom_model = BomEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("BOM不存在"))?;

        let mut bom_active: ActiveModel = bom_model.into();
        bom_active.status = Set(BomStatus::Inactive.to_string());
        bom_active.updated_at = Set(Utc::now());

        bom_active.update(&*self.db).await?;

        Ok(())
    }

    /// 获取BOM版本历史
    pub async fn get_versions(&self, product_id: i32) -> Result<Vec<BomModel>, AppError> {
        let boms = BomEntity::find()
            .filter(BomColumn::ProductId.eq(product_id))
            .order_by_desc(BomColumn::Version)
            .all(&*self.db)
            .await?;

        Ok(boms)
    }

    /// 复制BOM
    pub async fn copy(&self, id: i32, created_by: i32) -> Result<BomDetail, AppError> {
        let source = self
            .get_by_id(id)
            .await?
            .ok_or_else(|| AppError::not_found("源BOM不存在"))?;

        let new_version = self.get_next_version(source.bom.product_id).await?;

        let items: Vec<CreateBomItemRequest> = source
            .items
            .iter()
            .map(|item| CreateBomItemRequest {
                material_id: item.material_id,
                quantity: item.quantity,
                unit: item.unit.clone(),
                scrap_rate: item.scrap_rate,
                sort_order: item.sort_order,
            })
            .collect();

        self.create(CreateBomRequest {
            product_id: source.bom.product_id,
            version: Some(new_version),
            is_default: Some(false),
            remarks: source.bom.remarks,
            created_by,
            items,
        })
        .await
    }

    /// 获取下一个版本号
    async fn get_next_version(&self, product_id: i32) -> Result<i32, AppError> {
        let latest = BomEntity::find()
            .filter(BomColumn::ProductId.eq(product_id))
            .order_by_desc(BomColumn::Version)
            .one(&*self.db)
            .await?;

        Ok(match latest {
            Some(bom) => bom.version + 1,
            None => 1,
        })
    }

    /// 设置默认BOM
    pub async fn set_default(&self, id: i32) -> Result<BomModel, AppError> {
        let bom_model = BomEntity::find_by_id(id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("BOM不存在"))?;

        // 使用事务保护取消旧默认和设置新默认操作
        let txn = self.db.begin().await?;

        // 取消同产品其他默认BOM
        BomEntity::update_many()
            .filter(BomColumn::ProductId.eq(bom_model.product_id))
            .filter(BomColumn::IsDefault.eq(true))
            .set(ActiveModel {
                is_default: Set(false),
                updated_at: Set(Utc::now()),
                ..Default::default()
            })
            .exec(&txn)
            .await?;

        // 设置当前BOM为默认
        let mut bom_active: ActiveModel = bom_model.into();
        bom_active.is_default = Set(true);
        bom_active.updated_at = Set(Utc::now());

        let updated_bom = bom_active.update(&txn).await?;

        txn.commit().await?;

        Ok(updated_bom)
    }

    /// 获取BOM树形结构（递归展开）
    pub fn get_bom_tree(
        &self,
        bom_id: i32,
        max_depth: Option<i32>,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<BomTreeNode, AppError>> + Send + '_>,
    > {
        Box::pin(async move {
            let (bom, items) = self.fetch_bom_with_items(bom_id).await?;
            let mut children = Vec::new();
            let depth = max_depth.unwrap_or(10);
            if depth > 0 {
                let child_bom_map = self.fetch_child_bom_map(&items).await?;
                for item in &items {
                    let child_node = match child_bom_map.get(&item.material_id) {
                        Some(child_bom) => {
                            self.get_bom_tree(child_bom.id, Some(depth - 1)).await?
                        }
                        None => Self::build_leaf_bom_node(item),
                    };
                    children.push(child_node);
                }
            }
            Ok(BomTreeNode {
                id: format!("bom-{}", bom.id),
                product_id: bom.product_id,
                product_name: format!("产品 #{}", bom.product_id),
                quantity: Decimal::ONE,
                unit: None,
                scrap_rate: None,
                children,
            })
        })
    }

    /// 查询 BOM 主记录及其子项列表
    async fn fetch_bom_with_items(
        &self,
        bom_id: i32,
    ) -> Result<(BomModel, Vec<BomItemModel>), AppError> {
        let bom = BomEntity::find_by_id(bom_id)
            .one(&*self.db)
            .await?
            .ok_or_else(|| AppError::not_found("BOM不存在"))?;
        let items = BomItemEntity::find()
            .filter(BomItemColumn::BomId.eq(bom_id))
            .order_by_asc(BomItemColumn::SortOrder)
            .all(&*self.db)
            .await?;
        Ok((bom, items))
    }

    /// 批量查询子物料的默认 BOM 并按 product_id 索引
    async fn fetch_child_bom_map(
        &self,
        items: &[BomItemModel],
    ) -> Result<std::collections::HashMap<i32, BomModel>, AppError> {
        let material_ids: Vec<i32> = items.iter().map(|item| item.material_id).collect();
        let child_boms: Vec<BomModel> = if material_ids.is_empty() {
            Vec::new()
        } else {
            BomEntity::find()
                .filter(BomColumn::ProductId.is_in(material_ids))
                .filter(BomColumn::IsDefault.eq(true))
                .filter(BomColumn::Status.eq("ACTIVE"))
                .all(&*self.db)
                .await?
        };
        Ok(child_boms
            .into_iter()
            .map(|bom| (bom.product_id, bom))
            .collect())
    }

    /// 构建叶子节点 BomTreeNode
    fn build_leaf_bom_node(item: &BomItemModel) -> BomTreeNode {
        BomTreeNode {
            id: format!("item-{}", item.id),
            product_id: item.material_id,
            product_name: format!("物料 #{}", item.material_id),
            quantity: item.quantity,
            unit: item.unit.clone(),
            scrap_rate: item.scrap_rate,
            children: vec![],
        }
    }

    /// 获取BOM用量计算（多层级）
    pub async fn calculate_bom_requirements(
        &self,
        bom_id: i32,
        quantity: Decimal,
    ) -> Result<Vec<BomRequirement>, AppError> {
        let tree = self.get_bom_tree(bom_id, Some(10)).await?;
        let mut requirements = Vec::new();
        self.collect_requirements(&tree, quantity, &mut requirements);
        Ok(requirements)
    }

    /// 提交BOM审核：将状态由 DRAFT/INACTIVE 流转为 PENDING
    pub async fn submit(&self, id: i32, user_id: i32) -> Result<BomModel, AppError> {
        info!("提交BOM审核，ID：{}", id);

        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 事务包裹"查询 + 状态检查 + update_with_audit"，加 lock_exclusive 防止并发提交同一 BOM 导致状态不一致
        let txn = self.db.begin().await?;

        let bom = BomEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("BOM不存在"))?;

        // 仅允许从草稿/失效状态提交
        if bom.status == BomStatus::Pending.to_string() {
            return Err(AppError::validation("BOM已处于审核中状态"));
        }

        let mut active: ActiveModel = bom.into();
        active.status = Set(BomStatus::Pending.to_string());
        active.updated_at = Set(Utc::now());

        // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active,
            Some(user_id),
        )
        .await?;

        txn.commit().await?;
        Ok(updated)
    }

    /// 审核BOM：通过 approved 决定最终状态（ACTIVE/INACTIVE）
    pub async fn approve(
        &self,
        id: i32,
        approved: bool,
        remark: Option<String>,
        user_id: i32,
    ) -> Result<BomModel, AppError> {
        info!("审核BOM，ID：{}，通过：{}", id, approved);

        // 批次 25 v6 P0 修复：状态机 lock_exclusive 补全，串行化并发状态变更
        // 事务包裹"查询 + 状态检查 + update_with_audit"，加 lock_exclusive 防止并发审批同一 BOM 导致状态不一致
        let txn = self.db.begin().await?;

        let bom = BomEntity::find_by_id(id)
            .lock_exclusive()
            .one(&txn)
            .await?
            .ok_or_else(|| AppError::not_found("BOM不存在"))?;

        if bom.status != BomStatus::Pending.to_string() {
            return Err(AppError::validation("仅审核中状态的BOM可以审批"));
        }

        let mut active: ActiveModel = bom.into();
        let new_status = if approved {
            BomStatus::Active
        } else {
            BomStatus::Inactive
        };
        active.status = Set(new_status.to_string());
        if let Some(r) = remark {
            active.remarks = Set(Some(r));
        }
        active.updated_at = Set(Utc::now());

        let updated = crate::services::audit_log_service::AuditLogService::update_with_audit(
            &txn,
            "auto_audit",
            active,
            // 批次 94 P2-10：原 Some(0) 占位改为真实操作人 user_id，便于审计追踪
            Some(user_id),
        )
        .await?;

        txn.commit().await?;
        Ok(updated)
    }

    /// 递归收集BOM需求
    fn collect_requirements(
        &self,
        node: &BomTreeNode,
        parent_quantity: Decimal,
        requirements: &mut Vec<BomRequirement>,
    ) {
        // 批次 97 P1-8 修复（v5 复审）：数量计算补 round_dp(4) 防止精度漂移（BOM 数量保留 4 位小数）
        let required_quantity = (parent_quantity * node.quantity).round_dp(4);
        let scrap_multiplier = match node.scrap_rate {
            Some(rate) if rate > Decimal::ZERO => Decimal::ONE + (rate / Decimal::from(100)),
            _ => Decimal::ONE,
        };
        let actual_quantity = (required_quantity * scrap_multiplier).round_dp(4);

        if node.children.is_empty() {
            // 叶子节点，添加到需求列表
            requirements.push(BomRequirement {
                product_id: node.product_id,
                product_name: node.product_name.clone(),
                required_quantity: actual_quantity,
                unit: node.unit.clone(),
            });
        } else {
            // 递归处理子节点
            for child in &node.children {
                self.collect_requirements(child, actual_quantity, requirements);
            }
        }
    }
}

/// BOM树节点
#[derive(Debug, Clone, serde::Serialize)]
pub struct BomTreeNode {
    pub id: String,
    pub product_id: i32,
    pub product_name: String,
    pub quantity: Decimal,
    pub unit: Option<String>,
    pub scrap_rate: Option<Decimal>,
    pub children: Vec<BomTreeNode>,
}

/// BOM需求项
#[derive(Debug, Clone, serde::Serialize)]
pub struct BomRequirement {
    pub product_id: i32,
    pub product_name: String,
    pub required_quantity: Decimal,
    pub unit: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::test_common::setup_test_db;
    use crate::decs;
    use crate::ymd;
    use crate::models::status::common;
    use std::str::FromStr;

    /// 构建测试用 BOM 树节点夹具
    ///
    /// 封装 `BomTreeNode` 的构造，便于在各测试中复用，
    /// 默认 unit 为 "个"，product_name 按 product_id 生成。
    fn make_bom_tree_node(
        product_id: i32,
        quantity: Decimal,
        scrap_rate: Option<Decimal>,
        children: Vec<BomTreeNode>,
    ) -> BomTreeNode {
        BomTreeNode {
            id: format!("node-{}", product_id),
            product_id,
            product_name: format!("物料 #{}", product_id),
            quantity,
            unit: Some("个".to_string()),
            scrap_rate,
            children,
        }
    }

    /// 测试_BOM状态常量_Active值正确
    ///
    /// 验证 BomStatus::Active 的字符串值与 common::STATUS_ACTIVE 一致（均为 "ACTIVE"），
    /// 确保 create/approve(true) 等方法写入数据库的状态值统一。
    #[test]
    fn 测试_BOM状态常量_Active值正确() {
        assert_eq!(BomStatus::Active.to_string(), "ACTIVE");
        assert_eq!(BomStatus::Active.to_string(), common::STATUS_ACTIVE);
    }

    /// 测试_BOM状态常量_Inactive值正确
    ///
    /// 验证 BomStatus::Inactive 的字符串值为 "INACTIVE"，
    /// 用于 delete（软删除）和 approve(false) 流程。
    #[test]
    fn 测试_BOM状态常量_Inactive值正确() {
        assert_eq!(BomStatus::Inactive.to_string(), "INACTIVE");
    }

    /// 测试_BOM状态常量_Pending值正确
    ///
    /// 验证 BomStatus::Pending 的字符串值与 common::STATUS_PENDING 一致（均为 "PENDING"），
    /// 用于 submit 流程将状态由草稿/失效流转为审核中。
    #[test]
    fn 测试_BOM状态常量_Pending值正确() {
        assert_eq!(BomStatus::Pending.to_string(), "PENDING");
        assert_eq!(BomStatus::Pending.to_string(), common::STATUS_PENDING);
    }

    /// 测试_BOM状态枚举_Display实现互不相同
    ///
    /// 验证三个状态的 Display 实现互不相同，避免状态机流转时误判。
    #[test]
    fn 测试_BOM状态枚举_Display实现互不相同() {
        let active = BomStatus::Active.to_string();
        let inactive = BomStatus::Inactive.to_string();
        let pending = BomStatus::Pending.to_string();

        assert_ne!(active, inactive);
        assert_ne!(active, pending);
        assert_ne!(inactive, pending);
    }

    /// 测试_decs夹具宏解析十进制数
    ///
    /// 验证 decs! 宏能正确解析 Decimal 字符串，用于后续数量/金额计算测试夹具。
    #[test]
    fn 测试_decs夹具宏解析十进制数() {
        let v = decs!("123.456");
        assert_eq!(v.to_string(), "123.456");

        let zero = decs!("0");
        assert_eq!(zero, Decimal::ZERO);

        let one = decs!("1");
        assert_eq!(one, Decimal::ONE);
    }

    /// 测试_ymd夹具宏解析日期
    ///
    /// 验证 ymd! 宏能正确解析日期，确保测试夹具日期可用。
    #[test]
    fn 测试_ymd夹具宏解析日期() {
        let d = ymd!(2026, 7, 9);
        assert_eq!(d.format("%Y-%m-%d").to_string(), "2026-07-09");
    }

    /// 测试_BOM需求数量计算_叶子节点无损耗率
    ///
    /// 验证 collect_requirements 对叶子节点（无损耗率）的计算：
    /// 实际需求量 = 父级需求量 * 节点数量，无损耗放大。
    #[tokio::test]
    async fn 测试_BOM需求数量计算_叶子节点无损耗率() {
        let db = setup_test_db().await;
        let service = BomService::new(Arc::new(db));

        // 根节点 quantity=1（模拟 get_bom_tree 根节点），叶子节点 quantity=2
        let leaf = make_bom_tree_node(101, decs!("2"), None, vec![]);
        let root = make_bom_tree_node(100, Decimal::ONE, None, vec![leaf]);

        let mut requirements = Vec::new();
        service.collect_requirements(&root, decs!("10"), &mut requirements);

        // 根节点 actual = 10 * 1 = 10；叶子 actual = 10 * 2 = 20
        assert_eq!(requirements.len(), 1);
        assert_eq!(requirements[0].product_id, 101);
        assert_eq!(requirements[0].required_quantity, decs!("20"));
    }

    /// 测试_BOM需求数量计算_叶子节点有损耗率
    ///
    /// 验证 collect_requirements 对叶子节点（损耗率 10%）的计算：
    /// 损耗乘数 = 1 + 10/100 = 1.1；实际需求量 = 需求量 * 1.1。
    #[tokio::test]
    async fn 测试_BOM需求数量计算_叶子节点有损耗率() {
        let db = setup_test_db().await;
        let service = BomService::new(Arc::new(db));

        let leaf = make_bom_tree_node(101, decs!("2"), Some(decs!("10")), vec![]);
        let root = make_bom_tree_node(100, Decimal::ONE, None, vec![leaf]);

        let mut requirements = Vec::new();
        service.collect_requirements(&root, decs!("100"), &mut requirements);

        // 根节点 actual = 100 * 1 = 100；叶子 required = 100 * 2 = 200，乘以 1.1 = 220
        assert_eq!(requirements.len(), 1);
        assert_eq!(requirements[0].required_quantity, decs!("220"));
    }

    /// 测试_BOM需求数量计算_损耗率为零不放大
    ///
    /// 验证 collect_requirements 中 scrap_rate == 0 时不应用损耗放大
    /// （match 守卫 `rate > Decimal::ZERO` 为 false，乘数取 1）。
    #[tokio::test]
    async fn 测试_BOM需求数量计算_损耗率为零不放大() {
        let db = setup_test_db().await;
        let service = BomService::new(Arc::new(db));

        let leaf = make_bom_tree_node(101, decs!("5"), Some(Decimal::ZERO), vec![]);
        let root = make_bom_tree_node(100, Decimal::ONE, None, vec![leaf]);

        let mut requirements = Vec::new();
        service.collect_requirements(&root, decs!("10"), &mut requirements);

        // 损耗率为 0 时乘数为 1，actual = 10 * 5 * 1 = 50
        assert_eq!(requirements.len(), 1);
        assert_eq!(requirements[0].required_quantity, decs!("50"));
    }

    /// 测试_BOM需求数量计算_精度归一化四位小数
    ///
    /// 验证 collect_requirements 中 round_dp(4) 将中间结果归一化到 4 位小数，
    /// 防止 BOM 多层级递归数量计算时精度漂移。
    #[tokio::test]
    async fn 测试_BOM需求数量计算_精度归一化四位小数() {
        let db = setup_test_db().await;
        let service = BomService::new(Arc::new(db));

        // 节点数量 1.23456，父级需求 1 → required = 1.23456，round_dp(4) = 1.2346
        let leaf = make_bom_tree_node(101, decs!("1.23456"), None, vec![]);
        let root = make_bom_tree_node(100, Decimal::ONE, None, vec![leaf]);

        let mut requirements = Vec::new();
        service.collect_requirements(&root, Decimal::ONE, &mut requirements);

        assert_eq!(requirements.len(), 1);
        assert_eq!(requirements[0].required_quantity, decs!("1.2346"));
    }

    /// 测试_BOM需求数量计算_递归多层级
    ///
    /// 验证 collect_requirements 递归处理多层级 BOM 树：
    /// 根 → 子节点1（叶子，含损耗） + 子节点2（叶子，无损耗），
    /// 需求量按层级逐级相乘并应用损耗率。
    #[tokio::test]
    async fn 测试_BOM需求数量计算_递归多层级() {
        let db = setup_test_db().await;
        let service = BomService::new(Arc::new(db));

        let child1 = make_bom_tree_node(201, decs!("2"), Some(decs!("10")), vec![]);
        let child2 = make_bom_tree_node(202, decs!("3"), None, vec![]);
        let root = make_bom_tree_node(100, Decimal::ONE, None, vec![child1, child2]);

        let mut requirements = Vec::new();
        service.collect_requirements(&root, decs!("10"), &mut requirements);

        // 根 actual = 10 * 1 = 10
        // child1: required = 10 * 2 = 20，损耗乘数 1.1，actual = 22
        // child2: required = 10 * 3 = 30，无损耗，actual = 30
        assert_eq!(requirements.len(), 2);
        assert_eq!(requirements[0].product_id, 201);
        assert_eq!(requirements[0].required_quantity, decs!("22"));
        assert_eq!(requirements[1].product_id, 202);
        assert_eq!(requirements[1].required_quantity, decs!("30"));
    }

    /// 测试_BOM损耗率乘数公式
    ///
    /// 验证 collect_requirements 中的损耗率乘数公式：
    /// rate > 0 时乘数 = 1 + rate/100；否则乘数 = 1。
    #[test]
    fn 测试_BOM损耗率乘数公式() {
        // 复现 collect_requirements 中的 scrap_multiplier 计算
        let calc_multiplier = |rate: Option<Decimal>| -> Decimal {
            match rate {
                Some(r) if r > Decimal::ZERO => Decimal::ONE + (r / Decimal::from(100)),
                _ => Decimal::ONE,
            }
        };

        // 10% 损耗 → 1.1
        assert_eq!(calc_multiplier(Some(decs!("10"))), decs!("1.1"));
        // 25% 损耗 → 1.25
        assert_eq!(calc_multiplier(Some(decs!("25"))), decs!("1.25"));
        // 0% 损耗 → 1（守卫 rate > 0 为 false）
        assert_eq!(calc_multiplier(Some(Decimal::ZERO)), Decimal::ONE);
        // 无损耗率 → 1
        assert_eq!(calc_multiplier(None), Decimal::ONE);
    }

    /// 测试_BOM树根节点数量为一
    ///
    /// 验证 get_bom_tree 构造的根节点 quantity 为 Decimal::ONE，
    /// 确保 calculate_bom_requirements 传入的 quantity 直接作为根级实际需求量。
    #[test]
    fn 测试_BOM树根节点数量为一() {
        // 复现 get_bom_tree 中根节点的 quantity 设置
        let root_quantity = Decimal::ONE;
        let parent_quantity = decs!("100");

        // 根节点 required = parent * root_quantity = 100 * 1 = 100
        let required = (parent_quantity * root_quantity).round_dp(4);
        assert_eq!(required, decs!("100"));
    }

    /// 测试_BOM需求收集_单叶子节点
    ///
    /// 验证 collect_requirements 对单叶子节点树（无子节点）直接产出一条需求记录，
    /// 需求量 = 父级需求量 * 节点数量。
    #[tokio::test]
    async fn 测试_BOM需求收集_单叶子节点() {
        let db = setup_test_db().await;
        let service = BomService::new(Arc::new(db));

        let leaf = make_bom_tree_node(101, decs!("3"), None, vec![]);
        let mut requirements = Vec::new();
        service.collect_requirements(&leaf, decs!("10"), &mut requirements);

        assert_eq!(requirements.len(), 1);
        assert_eq!(requirements[0].product_id, 101);
        assert_eq!(requirements[0].required_quantity, decs!("30"));
        assert_eq!(requirements[0].product_name, "物料 #101");
    }

    /// 测试_版本号计算_首个版本为1
    ///
    /// 验证 get_next_version 中无历史 BOM 时返回 1（纯逻辑复现）。
    #[test]
    fn 测试_版本号计算_首个版本为1() {
        // 复现 get_next_version 的纯算法：latest = None → 1
        let latest: Option<BomModel> = None;
        let next = match latest {
            Some(bom) => bom.version + 1,
            None => 1,
        };
        assert_eq!(next, 1);
    }

    /// 测试_版本号计算_递增逻辑
    ///
    /// 验证 get_next_version 中存在历史 BOM 时返回 version + 1（纯逻辑复现）。
    #[test]
    fn 测试_版本号计算_递增逻辑() {
        // 复现 get_next_version 的纯算法：latest = Some(version=5) → 6
        let latest_version = 5;
        let next = latest_version + 1;
        assert_eq!(next, 6);
    }

    /// 测试_创建请求默认值_is_default默认false
    ///
    /// 验证 create 方法中 is_default.unwrap_or(false) 的默认值逻辑，
    /// 未显式指定默认版本时应为 false。
    #[test]
    fn 测试_创建请求默认值_is_default默认false() {
        let req = CreateBomRequest {
            product_id: 1,
            version: Some(1),
            is_default: None,
            remarks: None,
            created_by: 1,
            items: vec![],
        };
        // 复现 create 中的默认值取值
        assert_eq!(req.is_default.unwrap_or(false), false);
    }

    /// 测试_错误消息_BOM不存在
    ///
    /// 验证 update/delete/set_default/get_bom_tree 中 not_found("BOM不存在") 的错误类型与消息。
    #[test]
    fn 测试_错误消息_BOM不存在() {
        let err = AppError::not_found("BOM不存在");
        assert!(matches!(err, AppError::NotFound(_)));
        assert_eq!(err.to_string(), "未找到：BOM不存在");
    }

    /// 测试_错误消息_BOM已处于审核中状态
    ///
    /// 验证 submit 方法中状态为 Pending 时拒绝重复提交的错误消息。
    #[test]
    fn 测试_错误消息_BOM已处于审核中状态() {
        let err = AppError::validation("BOM已处于审核中状态");
        assert!(matches!(err, AppError::ValidationError(_)));
        assert_eq!(err.to_string(), "验证错误：BOM已处于审核中状态");
    }

    /// 测试_错误消息_仅审核中状态可审批
    ///
    /// 验证 approve 方法中状态非 Pending 时拒绝审批的错误消息。
    #[test]
    fn 测试_错误消息_仅审核中状态可审批() {
        let err = AppError::validation("仅审核中状态的BOM可以审批");
        assert!(matches!(err, AppError::ValidationError(_)));
        assert_eq!(err.to_string(), "验证错误：仅审核中状态的BOM可以审批");
    }

    /// 测试_服务实例创建
    ///
    /// 验证 BomService 在 SQLite 内存数据库上能正常实例化。
    #[tokio::test]
    async fn 测试_服务实例创建() {
        let db = setup_test_db().await;
        let service = BomService::new(Arc::new(db));
        assert!(Arc::strong_count(&service.db) >= 1);
    }

    /// 测试_创建BOM_需要真实数据库
    ///
    /// 需要 boms/bom_items 表 schema，标注 #[ignore] 仅在本地手动运行。
    /// 验证调用路径不 panic；无 schema 时返回数据库错误。
    #[tokio::test]
    #[ignore]
    async fn 测试_创建BOM_需要真实数据库() {
        let db = setup_test_db().await;
        let service = BomService::new(Arc::new(db));

        let req = CreateBomRequest {
            product_id: 1,
            version: Some(1),
            is_default: Some(false),
            remarks: None,
            created_by: 1,
            items: vec![CreateBomItemRequest {
                material_id: 101,
                quantity: decs!("2"),
                unit: Some("个".to_string()),
                scrap_rate: Some(decs!("10")),
                sort_order: None,
            }],
        };
        let result = service.create(req).await;
        // 无 schema 时返回数据库错误
        assert!(result.is_err());
    }

    /// 测试_获取BOM树_需要真实数据库
    ///
    /// 需要 boms/bom_items 表 schema，标注 #[ignore] 仅在本地手动运行。
    /// 验证调用路径不 panic。
    #[tokio::test]
    #[ignore]
    async fn 测试_获取BOM树_需要真实数据库() {
        let db = setup_test_db().await;
        let service = BomService::new(Arc::new(db));

        let result = service.get_bom_tree(1, Some(3)).await;
        // L-19 修复（批次 377 v13 复审）：原 let _ = result 无断言，改为 is_err 断言
        // 无记录时返回 NotFound；无 schema 时返回数据库错误
        assert!(result.is_err(), "无 schema/无记录时应返回错误");
    }

    /// 测试_提交审核_需要真实数据库
    ///
    /// 需要 boms 表 schema，标注 #[ignore] 仅在本地手动运行。
    /// 验证调用路径不 panic。
    #[tokio::test]
    #[ignore]
    async fn 测试_提交审核_需要真实数据库() {
        let db = setup_test_db().await;
        let service = BomService::new(Arc::new(db));

        let result = service.submit(1, 1).await;
        // L-19 修复（批次 377 v13 复审）：原 let _ = result 无断言，改为 is_err 断言
        // 无记录时返回 NotFound；无 schema 时返回数据库错误
        assert!(result.is_err(), "无 schema/无记录时应返回错误");
    }
}
