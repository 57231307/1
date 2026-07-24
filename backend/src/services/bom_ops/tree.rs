//! BOM Service 树形结构与需求计算子模块（bom_ops/tree）
//!
//! 批次 D10 拆分：从原 `bom_service.rs` 迁移。
//! 包含 `BomService` 的 BOM 树形查询与多层级用量计算方法：
//! - `get_bom_tree`：递归展开 BOM 树（返回 Pin<Box<Future>>，借用 self）
//! - `fetch_bom_with_items`：查询 BOM 主记录及其子项（私有 helper）
//! - `fetch_child_bom_map`：批量查询子物料默认 BOM 并按 product_id 索引（私有 helper）
//! - `calculate_bom_requirements`：多层级 BOM 用量计算
//! - `collect_requirements`：递归收集需求（`pub(crate)`，借用 self 仅用于递归；facade 测试模块调用）
//!
//! 跨模块调用：
//! - `get_bom_tree` 调用 facade 纯函数 `Self::build_leaf_bom_node`（`pub(crate)`）
//! - facade 测试模块 `bom_service::tests` 调用 `collect_requirements`（故声明为 `pub(crate)`）
//!
//! 本模块仅查询（无 insert/update/delete/事务），故仅导入查询相关 trait。

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::models::bom::{Column as BomColumn, Entity as BomEntity, Model as BomModel};
use crate::models::bom_item::{
    Column as BomItemColumn, Entity as BomItemEntity, Model as BomItemModel,
};
use crate::services::bom_service::{BomRequirement, BomService, BomTreeNode};
use crate::utils::error::AppError;

impl BomService {
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

    /// 递归收集BOM需求
    pub(crate) fn collect_requirements(
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
