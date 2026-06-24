use crate::utils::fabric_five_dimension::FabricFiveDimension;

/// 五维查询服务
/// 提供统一的五维查询接口，支持精确查询、模糊查询和统计查询
#[derive(Debug, Clone)]
pub struct FiveDimensionQueryService;

impl Default for FiveDimensionQueryService {
    fn default() -> Self {
        Self
    }
}

impl FiveDimensionQueryService {
    /// 创建查询构建器
    pub fn new() -> Self {
        Self
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
        let dim =
            FiveDimensionQueryService::parse_five_dimension_id(id).expect("应该能解析有效的五维ID");

        assert_eq!(dim.product_id, 100);
        assert_eq!(dim.batch_no, "20240101");
        assert_eq!(dim.color_no, "001");
        assert_eq!(dim.dye_lot_no, Some("20240101001".to_string()));
        assert_eq!(dim.grade, "一等品");
    }
}
