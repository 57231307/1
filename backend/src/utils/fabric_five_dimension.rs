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
        let dim = FabricFiveDimension::from_unique_id(unique_id).expect("valid id");

        assert_eq!(dim.product_id, 100);
        assert_eq!(dim.batch_no, "20240101");
        assert_eq!(dim.color_no, "001");
        assert_eq!(dim.dye_lot_no, Some("20240101001".to_string()));
        assert_eq!(dim.grade, "一等品");
    }

    #[test]
    fn test_from_unique_id_without_dye_lot() {
        let unique_id = "P100|B20240101|C001|DN|G一等品";
        let dim = FabricFiveDimension::from_unique_id(unique_id).expect("valid id");

        assert_eq!(dim.product_id, 100);
        assert_eq!(dim.dye_lot_no, None);
    }
}
