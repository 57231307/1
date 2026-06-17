//! P9-2 库存查询子模块（占位）
//!
//! 拆分自原 `services/inventory_stock_service.rs`。
//!
//! ## 模块职责
//! - 库存余额查询（多维度）
//! - 库存汇总报表
//! - 库存导出（Excel）

/// P9-2 标记：库存查询子模块路径
pub const P92_QRY_MODULE: &str = "stock_query";

/// 库存查询过滤条件
#[derive(Debug, Clone, Default)]
pub struct StockFilter {
    /// 仓库 ID
    pub warehouse_id: Option<i32>,
    /// 产品 ID
    pub product_id: Option<i32>,
    /// 批次号
    pub batch_no: Option<String>,
    /// 色号
    pub color_no: Option<String>,
    /// 等级
    pub grade: Option<String>,
    /// 关键字
    pub keyword: Option<String>,
}

impl StockFilter {
    /// 是否为空过滤
    pub fn is_empty(&self) -> bool {
        self.warehouse_id.is_none()
            && self.product_id.is_none()
            && self.batch_no.is_none()
            && self.color_no.is_none()
            && self.grade.is_none()
            && self.keyword.is_none()
    }

    /// 中文描述
    pub fn desc(&self) -> String {
        let mut parts = Vec::new();
        if let Some(w) = self.warehouse_id {
            parts.push(format!("仓库={w}"));
        }
        if let Some(p) = self.product_id {
            parts.push(format!("产品={p}"));
        }
        if let Some(b) = &self.batch_no {
            parts.push(format!("批次={b}"));
        }
        if let Some(c) = &self.color_no {
            parts.push(format!("色号={c}"));
        }
        if let Some(g) = &self.grade {
            parts.push(format!("等级={g}"));
        }
        if parts.is_empty() {
            "全部库存".to_string()
        } else {
            parts.join(", ")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_is_empty() {
        let f = StockFilter::default();
        assert!(f.is_empty());
        assert_eq!(f.desc(), "全部库存");
    }

    #[test]
    fn test_filter_with_warehouse() {
        let f = StockFilter {
            warehouse_id: Some(2),
            ..Default::default()
        };
        assert!(!f.is_empty());
        assert!(f.desc().contains("仓库=2"));
    }

    #[test]
    fn test_module_loaded() {
        assert_eq!(P92_QRY_MODULE, "stock_query");
    }
}
