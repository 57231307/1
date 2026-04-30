use crate::models::{
    inventory_stock, inventory_transaction, purchase_receipt_item, sales_delivery_item,
};
use crate::utils::fabric_five_dimension::FabricFiveDimension;
use sea_orm::{ColumnTrait, QueryFilter};

/// 五维查询服务
/// 提供统一的五维查询接口，支持精确查询、模糊查询和统计查询
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct FiveDimensionQueryService;

impl Default for FiveDimensionQueryService {
    fn default() -> Self {
        Self
    }
}

#[allow(dead_code)]
impl FiveDimensionQueryService {
    /// 创建查询构建器
    pub fn new() -> Self {
        Self
    }

    /// 为库存表应用五维过滤条件
    pub fn apply_to_inventory(
        query: sea_orm::Select<inventory_stock::Entity>,
        dimension: &FabricFiveDimension,
    ) -> sea_orm::Select<inventory_stock::Entity> {
        let mut query = query
            .filter(inventory_stock::Column::ProductId.eq(dimension.product_id))
            .filter(inventory_stock::Column::BatchNo.eq(&dimension.batch_no))
            .filter(inventory_stock::Column::ColorNo.eq(&dimension.color_no))
            .filter(inventory_stock::Column::Grade.eq(&dimension.grade));

        if let Some(ref dl) = dimension.dye_lot_no {
            query = query.filter(inventory_stock::Column::DyeLotNo.eq(dl));
        }

        query
    }

    /// 为库存表应用部分五维过滤条件（支持模糊查询）
    pub fn apply_partial_to_inventory(
        query: sea_orm::Select<inventory_stock::Entity>,
        product_id: Option<i32>,
        batch_no: Option<String>,
        color_no: Option<String>,
        dye_lot_no: Option<String>,
        grade: Option<String>,
    ) -> sea_orm::Select<inventory_stock::Entity> {
        let mut query = query;

        if let Some(pid) = product_id {
            query = query.filter(inventory_stock::Column::ProductId.eq(pid));
        }

        if let Some(batch) = batch_no {
            query = query.filter(inventory_stock::Column::BatchNo.like(format!("%{}%", batch)));
        }

        if let Some(color) = color_no {
            query = query.filter(inventory_stock::Column::ColorNo.like(format!("%{}%", color)));
        }

        if let Some(dye_lot) = dye_lot_no {
            query = query.filter(inventory_stock::Column::DyeLotNo.like(format!("%{}%", dye_lot)));
        }

        if let Some(g) = grade {
            query = query.filter(inventory_stock::Column::Grade.eq(g));
        }

        query
    }

    /// 为库存流水表应用五维过滤条件
    pub fn apply_to_transaction(
        query: sea_orm::Select<inventory_transaction::Entity>,
        dimension: &FabricFiveDimension,
    ) -> sea_orm::Select<inventory_transaction::Entity> {
        let mut query = query
            .filter(inventory_transaction::Column::ProductId.eq(dimension.product_id))
            .filter(inventory_transaction::Column::BatchNo.eq(&dimension.batch_no))
            .filter(inventory_transaction::Column::ColorNo.eq(&dimension.color_no))
            .filter(inventory_transaction::Column::Grade.eq(&dimension.grade));

        if let Some(ref dl) = dimension.dye_lot_no {
            query = query.filter(inventory_transaction::Column::DyeLotNo.eq(dl));
        }

        query
    }

    /// 为采购收货表应用五维过滤条件
    pub fn apply_to_purchase_receipt(
        query: sea_orm::Select<purchase_receipt_item::Entity>,
        dimension: &FabricFiveDimension,
    ) -> sea_orm::Select<purchase_receipt_item::Entity> {
        query.filter(purchase_receipt_item::Column::ProductId.eq(dimension.product_id))
    }

    /// 为销售发货表应用五维过滤条件
    pub fn apply_to_sales_delivery(
        query: sea_orm::Select<sales_delivery_item::Entity>,
        dimension: &FabricFiveDimension,
    ) -> sea_orm::Select<sales_delivery_item::Entity> {
        query.filter(sales_delivery_item::Column::ProductId.eq(dimension.product_id))
    }

    /// 生成五维 ID
    pub fn generate_five_dimension_id(
        product_id: i32,
        batch_no: &str,
        color_no: &str,
        dye_lot_no: Option<&str>,
        grade: &str,
    ) -> String {
        let dye_lot = dye_lot_no.unwrap_or("N");
        format!(
            "P{}|B{}|C{}|D{}|G{}",
            product_id, batch_no, color_no, dye_lot, grade
        )
    }

    /// 从五维 ID 解析查询条件
    pub fn parse_five_dimension_id(five_dim_id: &str) -> Option<FabricFiveDimension> {
        FabricFiveDimension::from_unique_id(five_dim_id).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_five_dimension_id() {
        let id = FiveDimensionQueryService::generate_five_dimension_id(
            100,
            "20240101",
            "001",
            Some("20240101001"),
            "一等品",
        );
        assert_eq!(id, "P100|B20240101|C001|D20240101001|G一等品");
    }

    #[test]
    fn test_generate_five_dimension_id_without_dye_lot() {
        let id = FiveDimensionQueryService::generate_five_dimension_id(
            100,
            "20240101",
            "001",
            None,
            "一等品",
        );
        assert_eq!(id, "P100|B20240101|C001|DN|G一等品");
    }

    #[test]
    fn test_parse_five_dimension_id() {
        let id = "P100|B20240101|C001|D20240101001|G一等品";
        let dim = FiveDimensionQueryService::parse_five_dimension_id(id).unwrap();

        assert_eq!(dim.product_id, 100);
        assert_eq!(dim.batch_no, "20240101");
        assert_eq!(dim.color_no, "001");
        assert_eq!(dim.dye_lot_no, Some("20240101001".to_string()));
        assert_eq!(dim.grade, "一等品");
    }
}
