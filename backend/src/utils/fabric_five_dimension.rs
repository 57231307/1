#![allow(dead_code, unused_variables, unused_imports, unused_mut)]
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// 面料五维管理结构体
///
/// 五维定义：
/// 1. 成品 ID (product_id) - 产品维度
/// 2. 批次号 (batch_no) - 生产批次维度
/// 3. 色号 (color_no) - 颜色维度
/// 4. 缸号 (dye_lot_no) - 染色缸次维度
/// 5. 等级 (grade) - 质量等级维度
///
/// 全局唯一 ID 格式：`P{id}|B{batch}|C{color}|D{dye_lot}|G{grade}`
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct FabricFiveDimension {
    /// 成品 ID
    pub product_id: i32,
    /// 批次号
    pub batch_no: String,
    /// 色号
    pub color_no: String,
    /// 缸号（可选）
    pub dye_lot_no: Option<String>,
    /// 等级
    pub grade: String,
}

impl FabricFiveDimension {
    /// 创建五维对象
    pub fn new(
        product_id: i32,
        batch_no: String,
        color_no: String,
        dye_lot_no: Option<String>,
        grade: String,
    ) -> Self {
        Self {
            product_id,
            batch_no,
            color_no,
            dye_lot_no,
            grade,
        }
    }

    /// 生成全局唯一 ID
    /// 格式：`P{id}|B{batch}|C{color}|D{dye_lot}|G{grade}`
    pub fn generate_unique_id(&self) -> String {
        let dye_lot = self.dye_lot_no.as_deref().unwrap_or("N");
        format!(
            "P{}|B{}|C{}|D{}|G{}",
            self.product_id, self.batch_no, self.color_no, dye_lot, self.grade
        )
    }

    /// 从全局唯一 ID 解析五维对象
    pub fn from_unique_id(unique_id: &str) -> Result<Self, String> {
        let parts: Vec<&str> = unique_id.split('|').collect();

        if parts.len() != 5 {
            return Err(format!("无效的五维 ID 格式：{}", unique_id));
        }

        // 解析成品 ID
        let product_id = parts[0]
            .strip_prefix('P')
            .and_then(|s| s.parse::<i32>().ok())
            .ok_or_else(|| format!("无效的成品 ID：{}", parts[0]))?;

        // 解析批次号
        let batch_no = parts[1]
            .strip_prefix('B')
            .map(|s| s.to_string())
            .ok_or_else(|| format!("无效的批次号：{}", parts[1]))?;

        // 解析色号
        let color_no = parts[2]
            .strip_prefix('C')
            .map(|s| s.to_string())
            .ok_or_else(|| format!("无效的色号：{}", parts[2]))?;

        // 解析缸号
        let dye_lot_no = if parts[3] == "DN"
            || parts[3] == "N"
            || parts[3].strip_prefix('D').is_some_and(|s| s.is_empty())
        {
            None
        } else {
            Some(
                parts[3]
                    .strip_prefix('D')
                    .map(|s| s.to_string())
                    .ok_or_else(|| format!("无效的缸号：{}", parts[3]))?,
            )
        };

        // 解析等级
        let grade = parts[4]
            .strip_prefix('G')
            .map(|s| s.to_string())
            .ok_or_else(|| format!("无效的等级：{}", parts[4]))?;

        Ok(Self {
            product_id,
            batch_no,
            color_no,
            dye_lot_no,
            grade,
        })
    }

    /// 验证五维数据的完整性
    pub fn validate(&self) -> Result<(), String> {
        if self.product_id <= 0 {
            return Err("成品 ID 必须大于 0".to_string());
        }

        if self.batch_no.trim().is_empty() {
            return Err("批次号不能为空".to_string());
        }

        if self.color_no.trim().is_empty() {
            return Err("色号不能为空".to_string());
        }

        if self.grade.trim().is_empty() {
            return Err("等级不能为空".to_string());
        }

        // 验证等级合法性
        let valid_grades = ["一等品", "二等品", "等外品", "优等品", "合格品", "不合格"];
        if !valid_grades.contains(&self.grade.as_str()) {
            return Err(format!(
                "无效的等级：{}，有效值为：{:?}",
                self.grade, valid_grades
            ));
        }

        Ok(())
    }

    /// 生成五维描述文本
    #[allow(dead_code)]
    pub fn to_description(&self) -> String {
        let dye_lot_desc = self
            .dye_lot_no
            .as_ref()
            .map(|dl| format!("缸号：{}", dl))
            .unwrap_or_else(|| "缸号：无".to_string());

        format!(
            "成品 {} | 批次 {} | 色号 {} | {} | 等级 {}",
            self.product_id, self.batch_no, self.color_no, dye_lot_desc, self.grade
        )
    }

    /// 比较两个五维对象是否相同（忽略缸号）
    #[allow(dead_code)]
    pub fn equals_ignore_dye_lot(&self, other: &Self) -> bool {
        self.product_id == other.product_id
            && self.batch_no == other.batch_no
            && self.color_no == other.color_no
            && self.grade == other.grade
    }

    /// 生成五维键（用于缓存或哈希表）
    #[allow(dead_code)]
    pub fn generate_key(&self) -> String {
        let dye_lot = self.dye_lot_no.as_deref().unwrap_or("*");
        format!(
            "{}:{}:{}:{}:{}",
            self.product_id, self.batch_no, self.color_no, dye_lot, self.grade
        )
    }
}

/// 五维查询条件构建器
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct FiveDimensionQueryBuilder {
    product_id: Option<i32>,
    batch_no: Option<String>,
    color_no: Option<String>,
    dye_lot_no: Option<String>,
    grade: Option<String>,
}

#[allow(dead_code)]
impl FiveDimensionQueryBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn product_id(mut self, product_id: i32) -> Self {
        self.product_id = Some(product_id);
        self
    }

    pub fn batch_no(mut self, batch_no: String) -> Self {
        self.batch_no = Some(batch_no);
        self
    }

    pub fn color_no(mut self, color_no: String) -> Self {
        self.color_no = Some(color_no);
        self
    }

    pub fn dye_lot_no(mut self, dye_lot_no: String) -> Self {
        self.dye_lot_no = Some(dye_lot_no);
        self
    }

    pub fn grade(mut self, grade: String) -> Self {
        self.grade = Some(grade);
        self
    }

    /// 生成 WHERE 子句
    pub fn build_where_clause(&self) -> (String, Vec<String>) {
        let mut conditions = Vec::new();
        let mut params = Vec::new();

        if let Some(pid) = self.product_id {
            conditions.push("product_id = ?");
            params.push(pid.to_string());
        }

        if let Some(ref batch) = self.batch_no {
            conditions.push("batch_no = ?");
            params.push(batch.clone());
        }

        if let Some(ref color) = self.color_no {
            conditions.push("color_no = ?");
            params.push(color.clone());
        }

        if let Some(ref dye_lot) = self.dye_lot_no {
            conditions.push("dye_lot_no = ?");
            params.push(dye_lot.clone());
        }

        if let Some(ref grade) = self.grade {
            conditions.push("grade = ?");
            params.push(grade.clone());
        }

        let where_clause = if conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", conditions.join(" AND "))
        };

        (where_clause, params)
    }

    /// 生成五维 ID 前缀（用于模糊查询）
    pub fn build_id_prefix(&self) -> String {
        let mut prefix = String::new();

        if let Some(pid) = self.product_id {
            prefix.push_str(&format!("P{}|", pid));
        }

        if let Some(batch) = &self.batch_no {
            prefix.push_str(&format!("B{}|", batch));
        }

        if let Some(color) = &self.color_no {
            prefix.push_str(&format!("C{}|", color));
        }

        if let Some(dye_lot) = &self.dye_lot_no {
            prefix.push_str(&format!("D{}|", dye_lot));
        }

        if let Some(grade) = &self.grade {
            prefix.push_str(&format!("G{}", grade));
        }

        prefix
    }
}

/// 五维统计数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FiveDimensionStatistics {
    /// 五维对象
    pub dimension: FabricFiveDimension,
    /// 总米数
    pub total_meters: Decimal,
    /// 总公斤数
    pub total_kg: Decimal,
    /// 库存记录数
    pub stock_count: i64,
    /// 最后更新时间
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_unique_id() {
        let dim = FabricFiveDimension {
            product_id: 100,
            batch_no: "20240101".to_string(),
            color_no: "001".to_string(),
            dye_lot_no: Some("20240101001".to_string()),
            grade: "一等品".to_string(),
        };

        let unique_id = dim.generate_unique_id();
        assert_eq!(unique_id, "P100|B20240101|C001|D20240101001|G一等品");
    }

    #[test]
    fn test_from_unique_id() {
        let unique_id = "P100|B20240101|C001|D20240101001|G一等品";
        let dim = FabricFiveDimension::from_unique_id(unique_id).unwrap();

        assert_eq!(dim.product_id, 100);
        assert_eq!(dim.batch_no, "20240101");
        assert_eq!(dim.color_no, "001");
        assert_eq!(dim.dye_lot_no, Some("20240101001".to_string()));
        assert_eq!(dim.grade, "一等品");
    }

    #[test]
    fn test_from_unique_id_without_dye_lot() {
        let unique_id = "P100|B20240101|C001|DN|G一等品";
        let dim = FabricFiveDimension::from_unique_id(unique_id).unwrap();

        assert_eq!(dim.product_id, 100);
        assert_eq!(dim.dye_lot_no, None);
    }

    #[test]
    fn test_validate_success() {
        let dim = FabricFiveDimension::new(
            100,
            "B20240101".to_string(),
            "C001".to_string(),
            Some("D20240101001".to_string()),
            "一等品".to_string(),
        );

        assert!(dim.validate().is_ok());
    }

    #[test]
    fn test_validate_invalid_product_id() {
        let dim = FabricFiveDimension::new(
            0,
            "B20240101".to_string(),
            "C001".to_string(),
            Some("D20240101001".to_string()),
            "一等品".to_string(),
        );

        assert!(dim.validate().is_err());
    }

    #[test]
    fn test_validate_invalid_grade() {
        let dim = FabricFiveDimension::new(
            100,
            "B20240101".to_string(),
            "C001".to_string(),
            Some("D20240101001".to_string()),
            "次品".to_string(),
        );

        assert!(dim.validate().is_err());
    }

    #[test]
    fn test_equals_ignore_dye_lot() {
        let dim1 = FabricFiveDimension::new(
            100,
            "B20240101".to_string(),
            "C001".to_string(),
            Some("D20240101001".to_string()),
            "一等品".to_string(),
        );

        let dim2 = FabricFiveDimension::new(
            100,
            "B20240101".to_string(),
            "C001".to_string(),
            Some("D20240101002".to_string()),
            "一等品".to_string(),
        );

        assert!(dim1.equals_ignore_dye_lot(&dim2));
    }

    #[test]
    fn test_generate_key() {
        let dim = FabricFiveDimension::new(
            100,
            "B20240101".to_string(),
            "C001".to_string(),
            None,
            "一等品".to_string(),
        );

        let key = dim.generate_key();
        assert_eq!(key, "100:B20240101:C001:*:一等品");
    }

    #[test]
    fn test_query_builder() {
        let builder = FiveDimensionQueryBuilder::new()
            .product_id(100)
            .batch_no("B20240101".to_string())
            .grade("一等品".to_string());

        let (where_clause, params) = builder.build_where_clause();
        assert!(where_clause.contains("product_id = ?"));
        assert!(where_clause.contains("batch_no = ?"));
        assert!(where_clause.contains("grade = ?"));
        assert_eq!(params.len(), 3);
    }
}
