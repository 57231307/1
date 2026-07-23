//! 产品 Service CSV 导入导出子模块（product_ops/import_export）
//!
//! 批次 D10 拆分：从原 `product_service.rs` 迁移。
//! 包含 `ProductService` 的 CSV 导入导出方法 + 字段校验/解析 helper：
//! - `export_products_to_csv`：导出 CSV（跨模块调用 `crud::list_products`）
//! - `import_products_from_csv`：从 CSV 导入
//! - `process_csv_import_row`：单行处理（跨模块调用 `crud::create_product`）
//! - 静态/私有 helper：build_product_csv_headers / build_product_csv_row /
//!  generate_product_import_template / get_required_field_value /
//!  parse_optional_csv_{string,f64,i32} / parse_csv_status_field /
//!  validate_csv_{code,name,product_type,unit}_field / validate_csv_import_row /
//!  build_csv_create_args
//!
//! `ValidatedRowFields` struct 从 facade 迁移到本模块（私有，仅 CSV import 内部使用）。

use crate::models::product;
use crate::models::status::master_data;
use crate::services::product_service::{CreateProductArgs, ProductService};
use crate::utils::error::AppError;
use crate::utils::import_export::{CsvImporter, FieldValidator, ImportResult};

/// CSV 导入行校验后的必填字段
///
/// D08-1 第二梯队拆分：用于封装 import_products_from_csv 中校验后的 4 个必填字段
/// （产品编码、产品名称、产品类型、计量单位），避免 validate_import_row 返回 4-tuple。
struct ValidatedRowFields {
    code: String,
    name: String,
    product_type: String,
    unit: String,
}

impl ProductService {
    /// 导出产品数据为 CSV 格式
    pub async fn export_products_to_csv(
        &self,
        category_id: Option<i32>,
        status: Option<String>,
        search: Option<String>,
    ) -> Result<Vec<u8>, AppError> {
        // 查询产品列表（单页拉取全量用于导出）
        let (products, _total) = self
            .list_products(1, 10000, category_id, status, search)
            .await?;

        // 构建 CSV 表头与行数据
        let headers = Self::build_product_csv_headers();
        let rows: Vec<std::collections::HashMap<String, String>> = products
            .iter()
            .map(Self::build_product_csv_row)
            .collect();

        // 生成 CSV 字节流
        crate::utils::import_export::CsvImporter::generate(&headers, &rows)
            .map_err(|e| AppError::business(format!("CSV 生成失败: {}", e)))
    }

    /// 构建 CSV 表头（19 列）
    fn build_product_csv_headers() -> Vec<String> {
        vec![
            "产品编码".to_string(),
            "产品名称".to_string(),
            "产品类型".to_string(),
            "类别ID".to_string(),
            "规格型号".to_string(),
            "计量单位".to_string(),
            "标准价格".to_string(),
            "成本价格".to_string(),
            "面料成分".to_string(),
            "纱支".to_string(),
            "密度".to_string(),
            "幅宽".to_string(),
            "克重".to_string(),
            "组织结构".to_string(),
            "后整理".to_string(),
            "最小起订量".to_string(),
            "交货期".to_string(),
            "状态".to_string(),
            "产品描述".to_string(),
        ]
    }

    /// 将单个产品转为 CSV 行 HashMap
    fn build_product_csv_row(p: &product::Model) -> std::collections::HashMap<String, String> {
        let mut row = std::collections::HashMap::new();
        row.insert("产品编码".to_string(), p.code.clone());
        row.insert("产品名称".to_string(), p.name.clone());
        row.insert("产品类型".to_string(), p.product_type.clone());
        row.insert(
            "类别ID".to_string(),
            p.category_id.map(|id| id.to_string()).unwrap_or_default(),
        );
        row.insert("规格型号".to_string(), p.specification.clone().unwrap_or_default());
        row.insert("计量单位".to_string(), p.unit.clone());
        row.insert(
            "标准价格".to_string(),
            p.standard_price.map(|price| price.to_string()).unwrap_or_default(),
        );
        row.insert(
            "成本价格".to_string(),
            p.cost_price.map(|price| price.to_string()).unwrap_or_default(),
        );
        row.insert(
            "面料成分".to_string(),
            p.fabric_composition.clone().unwrap_or_default(),
        );
        row.insert("纱支".to_string(), p.yarn_count.clone().unwrap_or_default());
        row.insert("密度".to_string(), p.density.clone().unwrap_or_default());
        row.insert(
            "幅宽".to_string(),
            p.width.map(|w| w.to_string()).unwrap_or_default(),
        );
        row.insert(
            "克重".to_string(),
            p.gram_weight.map(|g| g.to_string()).unwrap_or_default(),
        );
        row.insert("组织结构".to_string(), p.structure.clone().unwrap_or_default());
        row.insert("后整理".to_string(), p.finish.clone().unwrap_or_default());
        row.insert(
            "最小起订量".to_string(),
            p.min_order_quantity
                .map(|m| m.to_string())
                .unwrap_or_default(),
        );
        row.insert(
            "交货期".to_string(),
            p.lead_time.map(|l| l.to_string()).unwrap_or_default(),
        );
        row.insert("状态".to_string(), p.status.clone());
        row.insert("产品描述".to_string(), p.description.clone().unwrap_or_default());
        row
    }

    /// 生成产品导入模板
    pub fn generate_product_import_template() -> Result<Vec<u8>, AppError> {
        let headers = vec![
            "产品编码".to_string(),
            "产品名称".to_string(),
            "产品类型".to_string(),
            "类别ID".to_string(),
            "规格型号".to_string(),
            "计量单位".to_string(),
            "标准价格".to_string(),
            "成本价格".to_string(),
            "面料成分".to_string(),
            "纱支".to_string(),
            "密度".to_string(),
            "幅宽".to_string(),
            "克重".to_string(),
            "组织结构".to_string(),
            "后整理".to_string(),
            "最小起订量".to_string(),
            "交货期".to_string(),
            "状态".to_string(),
            "产品描述".to_string(),
        ];

        let mut example = std::collections::HashMap::new();
        example.insert("产品编码".to_string(), "FAB-001".to_string());
        example.insert("产品名称".to_string(), "纯棉坯布".to_string());
        example.insert("产品类型".to_string(), "坯布".to_string());
        example.insert("类别ID".to_string(), "1".to_string());
        example.insert("规格型号".to_string(), "40S*40S".to_string());
        example.insert("计量单位".to_string(), "米".to_string());
        example.insert("标准价格".to_string(), "15.50".to_string());
        example.insert("成本价格".to_string(), "12.00".to_string());
        example.insert("面料成分".to_string(), "100%棉".to_string());
        example.insert("纱支".to_string(), "40S".to_string());
        example.insert("密度".to_string(), "133*72".to_string());
        example.insert("幅宽".to_string(), "150.00".to_string());
        example.insert("克重".to_string(), "120.00".to_string());
        example.insert("组织结构".to_string(), "平纹".to_string());
        example.insert("后整理".to_string(), "防水".to_string());
        example.insert("最小起订量".to_string(), "1000".to_string());
        example.insert("交货期".to_string(), "15".to_string());
        example.insert("状态".to_string(), master_data::ACTIVE.to_string());
        example.insert("产品描述".to_string(), "高品质纯棉坯布".to_string());

        crate::utils::import_export::CsvImporter::generate_template(&headers, Some(&[example]))
            .map_err(|e| AppError::business(format!("模板生成失败: {}", e)))
    }

    /// 获取必填字段值，缺失列时添加错误并返回 None
    ///
    /// D08-1 第二梯队拆分：提取 import_products_from_csv 中重复的“取列或报错”模式。
    fn get_required_field_value<'a>(
        row: &'a std::collections::HashMap<String, String>,
        field: &str,
        row_num: usize,
        result: &mut ImportResult,
    ) -> Option<&'a str> {
        match row.get(field) {
            Some(v) => Some(v.as_str()),
            None => {
                result.add_error(
                    row_num,
                    field.to_string(),
                    format!("缺少{}列", field),
                    "".to_string(),
                );
                None
            }
        }
    }

    /// 解析可选字符串字段（缺失或空字符串视为 None）
    ///
    /// D08-1 第二梯队拆分：提取 import_products_from_csv 中重复的字符串解析模式。
    fn parse_optional_csv_string(
        row: &std::collections::HashMap<String, String>,
        field: &str,
    ) -> Option<String> {
        row.get(field).filter(|v| !v.is_empty()).cloned()
    }

    /// 解析可选 f64 字段（缺失、空字符串或解析失败均为 None）
    ///
    /// D08-1 第二梯队拆分：提取 import_products_from_csv 中重复的 f64 解析模式。
    fn parse_optional_csv_f64(
        row: &std::collections::HashMap<String, String>,
        field: &str,
    ) -> Option<f64> {
        row.get(field).and_then(|v| {
            if v.is_empty() {
                None
            } else {
                v.parse::<f64>().ok()
            }
        })
    }

    /// 解析可选 i32 字段（缺失、空字符串或解析失败均为 None）
    ///
    /// D08-1 第二梯队拆分：提取 import_products_from_csv 中重复的 i32 解析模式。
    fn parse_optional_csv_i32(
        row: &std::collections::HashMap<String, String>,
        field: &str,
    ) -> Option<i32> {
        row.get(field).and_then(|v| {
            if v.is_empty() {
                None
            } else {
                v.parse::<i32>().ok()
            }
        })
    }

    /// 解析状态字段，缺省为 master_data::ACTIVE
    ///
    /// D08-1 第二梯队拆分：提取 import_products_from_csv 中状态字段的默认值逻辑。
    fn parse_csv_status_field(row: &std::collections::HashMap<String, String>) -> String {
        row.get("状态")
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string())
            .unwrap_or_else(|| master_data::ACTIVE.to_string())
    }

    /// 校验产品编码（必填 + 长度 ≤50）
    ///
    /// D08-1 第二梯队拆分：从 validate_csv_import_row 拆出的单字段校验。
    fn validate_csv_code_field(
        row: &std::collections::HashMap<String, String>,
        row_num: usize,
        result: &mut ImportResult,
    ) -> Option<String> {
        let code = Self::get_required_field_value(row, "产品编码", row_num, result)?;
        if let Err(e) = FieldValidator::required(code, "产品编码") {
            result.add_error(row_num, "产品编码".to_string(), e, code.to_string());
            return None;
        }
        // v11 批次 156 P2-D：接入 FieldValidator::max_length 校验编码长度
        if let Err(e) = FieldValidator::max_length(code, "产品编码", 50) {
            result.add_error(row_num, "产品编码".to_string(), e, code.to_string());
            return None;
        }
        Some(code.to_string())
    }

    /// 校验产品名称（必填 + 长度 ≤200）
    ///
    /// D08-1 第二梯队拆分：从 validate_csv_import_row 拆出的单字段校验。
    fn validate_csv_name_field(
        row: &std::collections::HashMap<String, String>,
        row_num: usize,
        result: &mut ImportResult,
    ) -> Option<String> {
        let name = Self::get_required_field_value(row, "产品名称", row_num, result)?;
        if let Err(e) = FieldValidator::required(name, "产品名称") {
            result.add_error(row_num, "产品名称".to_string(), e, name.to_string());
            return None;
        }
        // v11 批次 156 P2-D：接入 FieldValidator::max_length 校验名称长度
        if let Err(e) = FieldValidator::max_length(name, "产品名称", 200) {
            result.add_error(row_num, "产品名称".to_string(), e, name.to_string());
            return None;
        }
        Some(name.to_string())
    }

    /// 校验产品类型（枚举值：坯布/成品布/辅料）
    ///
    /// D08-1 第二梯队拆分：从 validate_csv_import_row 拆出的单字段校验。
    fn validate_csv_product_type_field(
        row: &std::collections::HashMap<String, String>,
        row_num: usize,
        result: &mut ImportResult,
    ) -> Option<String> {
        let product_type = Self::get_required_field_value(row, "产品类型", row_num, result)?;
        if let Err(e) =
            FieldValidator::enum_value(product_type, "产品类型", &["坯布", "成品布", "辅料"])
        {
            result.add_error(
                row_num,
                "产品类型".to_string(),
                e,
                product_type.to_string(),
            );
            return None;
        }
        Some(product_type.to_string())
    }

    /// 校验计量单位（必填）
    ///
    /// D08-1 第二梯队拆分：从 validate_csv_import_row 拆出的单字段校验。
    fn validate_csv_unit_field(
        row: &std::collections::HashMap<String, String>,
        row_num: usize,
        result: &mut ImportResult,
    ) -> Option<String> {
        let unit = Self::get_required_field_value(row, "计量单位", row_num, result)?;
        if let Err(e) = FieldValidator::required(unit, "计量单位") {
            result.add_error(row_num, "计量单位".to_string(), e, unit.to_string());
            return None;
        }
        Some(unit.to_string())
    }

    /// 校验行的必填字段（产品编码、产品名称、产品类型、计量单位）
    ///
    /// D08-1 第二梯队拆分：从 import_products_from_csv 提取的必填字段校验协调逻辑，
    /// 校验失败时向 result 添加错误并返回 None，全部通过则返回 ValidatedRowFields。
    fn validate_csv_import_row(
        row: &std::collections::HashMap<String, String>,
        row_num: usize,
        result: &mut ImportResult,
    ) -> Option<ValidatedRowFields> {
        let code = Self::validate_csv_code_field(row, row_num, result)?;
        let name = Self::validate_csv_name_field(row, row_num, result)?;
        let product_type = Self::validate_csv_product_type_field(row, row_num, result)?;
        let unit = Self::validate_csv_unit_field(row, row_num, result)?;
        Some(ValidatedRowFields {
            code,
            name,
            product_type,
            unit,
        })
    }

    /// 基于校验后的必填字段和行数据构建 CreateProductArgs
    ///
    /// D08-1 第二梯队拆分：从 import_products_from_csv 提取的字段解析 + 参数对象构建逻辑。
    fn build_csv_create_args(
        row: &std::collections::HashMap<String, String>,
        validated: &ValidatedRowFields,
    ) -> CreateProductArgs {
        let category_id = Self::parse_optional_csv_i32(row, "类别ID");
        let standard_price = Self::parse_optional_csv_f64(row, "标准价格");
        let cost_price = Self::parse_optional_csv_f64(row, "成本价格");
        let specification = Self::parse_optional_csv_string(row, "规格型号");
        let fabric_composition = Self::parse_optional_csv_string(row, "面料成分");
        let yarn_count = Self::parse_optional_csv_string(row, "纱支");
        let density = Self::parse_optional_csv_string(row, "密度");
        let width = Self::parse_optional_csv_f64(row, "幅宽");
        let gram_weight = Self::parse_optional_csv_f64(row, "克重");
        let structure = Self::parse_optional_csv_string(row, "组织结构");
        let finish = Self::parse_optional_csv_string(row, "后整理");
        let min_order_quantity = Self::parse_optional_csv_f64(row, "最小起订量");
        let lead_time = Self::parse_optional_csv_i32(row, "交货期");
        let description = Self::parse_optional_csv_string(row, "产品描述");
        let status = Self::parse_csv_status_field(row);

        CreateProductArgs {
            name: validated.name.clone(),
            code: validated.code.clone(),
            category_id,
            specification,
            unit: validated.unit.clone(),
            standard_price,
            cost_price,
            description,
            status,
            product_type: validated.product_type.clone(),
            fabric_composition,
            yarn_count,
            density,
            width,
            gram_weight,
            structure,
            finish,
            min_order_quantity,
            lead_time,
        }
    }

    /// 处理 CSV 单行：校验 → 构建 args → 创建产品 → 写结果
    ///
    /// D08-1 第二梯队拆分：从 import_products_from_csv 提取的单行处理流程，
    /// 校验失败时直接返回（外层循环已 add_total，无需补偿）。
    async fn process_csv_import_row(
        &self,
        row: &std::collections::HashMap<String, String>,
        row_num: usize,
        result: &mut ImportResult,
    ) {
        let validated = match Self::validate_csv_import_row(row, row_num, result) {
            Some(v) => v,
            None => return,
        };
        let args = Self::build_csv_create_args(row, &validated);
        let code = validated.code.clone();
        match self.create_product(args).await {
            Ok(_) => result.add_success(),
            Err(e) => {
                result.add_error(
                    row_num,
                    "数据库".to_string(),
                    format!("创建产品失败: {}", e),
                    code,
                );
            }
        }
    }

    /// 从产品 CSV 数据导入
    pub async fn import_products_from_csv(
        &self,
        data: &[u8],
    ) -> Result<ImportResult, AppError> {
        let records = CsvImporter::parse(data)?;
        let mut result = ImportResult::new();

        for (row_idx, row) in records.iter().enumerate() {
            result.add_total();
            let row_num = row_idx + 2;
            self.process_csv_import_row(row, row_num, &mut result).await;
        }

        Ok(result)
    }
}
