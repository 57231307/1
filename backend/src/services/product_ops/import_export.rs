//! 产品 Service CSV 导入导出子模块（product_ops/import_export）
//!
//! 批次 D10 拆分：从原 `product_service.rs` 迁移。
//! 包含 `ProductService` 的 CSV 导入导出方法 + 字段校验/解析 helper：
//! - `export_products_to_xlsx`：导出 CSV（跨模块调用 `crud::list_products`）
//! - `import_products_from_csv`：从 CSV 导入
//! - `process_csv_import_row`：单行处理（跨模块调用 `crud::create_product`）
//! - 静态/私有 helper：PRODUCT_IMPORT_COLUMNS 权威表与 product_import_columns 只读引用 /
//!  build_product_csv_headers / build_product_csv_row /
//!  generate_product_import_template / parse_optional_csv_{string,decimal,i32} /
//!  parse_csv_status_field /
//!  validate_import_row_fields（行级必填与形态校验唯一实现，数据导入中心产品腿共用）/
//!  build_csv_create_args / product_import_error_column（行级失败的列名归属）
//!
//! 列定义唯一源：`PRODUCT_IMPORT_COLUMNS` 权威表（形态同
//! `handlers::sku_mapping_handler::SKU_MAPPING_IMPORT_COLUMNS`）。模板下载 产品导出
//! 通用数据导入中心的模板面 元数据面 注册表面与两条导入解析腿全部只从本表取列，
//! 任何一侧不再手抄列清单。增删导入列只改本表一处。

use crate::models::product;
use crate::models::status::master_data;
use crate::models::status::product_type;
use crate::services::product_service::{CreateProductArgs, ProductService};
use crate::utils::error::AppError;
use crate::utils::import_export::{CsvImporter, FieldValidator, ImportResult, XlsxImporter};
use rust_decimal::Decimal;
use std::collections::HashMap;
use std::str::FromStr;

// ============================================================================
// 产品文件导入列名常量（权威表的列名字面量唯一存放处）
// ============================================================================
// 表头文本同时是：CsvImporter/XlsxImporter 的行 Map 解析键、模板下载列名、
// 产品导出列名、通用数据导入中心 ImportColumnDef.title 与按位读列列序名。
// 逐字一致的同一个字符串，模板与解析不可能再漂移。

const COL_PRODUCT_CODE: &str = "产品编码";
const COL_PRODUCT_NAME: &str = "产品名称";
const COL_PRODUCT_TYPE: &str = "产品类型";
const COL_CATEGORY_ID: &str = "类别ID";
const COL_SPECIFICATION: &str = "规格型号";
const COL_UNIT: &str = "计量单位";
const COL_STANDARD_PRICE: &str = "标准价格";
pub const COL_COST_PRICE: &str = "成本价格";
const COL_COMPOSITION: &str = "面料成分";
const COL_YARN_COUNT: &str = "纱支";
const COL_DENSITY: &str = "密度";
const COL_WIDTH: &str = "有效幅宽";
const COL_GRAM_WEIGHT: &str = "克重";
const COL_STRUCTURE: &str = "组织结构";
const COL_FINISH: &str = "后整理";
const COL_MIN_ORDER_QUANTITY: &str = "订量";
const COL_LEAD_TIME: &str = "交货期";
const COL_STATUS: &str = "状态";
const COL_DESCRIPTION: &str = "产品描述";
const COL_BARCODE: &str = "条码";
const COL_EXECUTION_STANDARD: &str = "执行标准";
const COL_FACTORY_NAME: &str = "生产厂名";
const COL_FACTORY_ADDRESS: &str = "生产厂址";
const COL_PRODUCT_GRADE: &str = "产品等级";

/// 产品导入权威表单列定义。
pub(crate) struct ProductImportColumn {
    /// 表头中文列名，同时是解析键（用户文件的表头必须逐字等于它）。
    pub(crate) title: &'static str,
    /// 英文字段名，与建品入参对象 `CreateProductArgs` 的字段同名，
    /// 通用数据导入中心 `ImportColumnDef.field` 与元数据 JSON 消费该值。
    pub(crate) field: &'static str,
    /// 整行必填位（对一切产品类型都必填）。面料类三列（幅宽 克重 面料成分）属
    /// 按品类分级必填的条件必填，判据源是 services::product_service::validate_product_create_fields，
    /// 不由本位表达，也绝不在本表缺省填 0 或默认值绕过。
    pub(crate) required: bool,
    /// 数据类型词 string/decimal/integer，与通用模板 ImportColumnDef.data_type 与
    /// validate_import_data 的数值形态检查同词表（decimal 列解析收 Decimal，不做 f64 往返）。
    pub(crate) data_type: &'static str,
    /// 模板示例行取值。示例行自身必须能通过建品判据（坯布为面料类，
    /// 幅宽 克重 面料成分三列必须同全且为正），否则模板自己的示例行就导不进去。
    pub(crate) example: &'static str,
}

/// 产品文件导入的权威列清单（唯一清单，五面同源）。
///
/// 消费面：
/// 1. 模板面 `generate_product_import_template_xlsx`（/products/import-template 下载）。
/// 2. 导出面 `build_product_csv_headers` 与 `build_product_csv_row`（/products/export）。
/// 3. 通用模板面 `services::import_export_service::build_products_template`
///    （/import/templates/download/products 与 /import/templates/meta/products 与
///    数据导入中心注册表 IMPORT_TEMPLATE_STORE 的 products 行，三面都从它派生）。
/// 4. 通用导入腿 `services::import_export_ops::import::import_product_row`
///    （/import/excel 按位读列，列序即本表顺序）。
/// 5. 解析面 `validate_import_row_fields` 与 `build_csv_create_args`（/products/import）。
///
/// 列序契约：3 与 4 按位对齐（同 crm 线索腿「列序即导入器按位读列的次序」锁型），
/// 表头文本必须等于解析键，顺序变了就会把克重读进幅宽列，故模板与解析不许单边改动。
const PRODUCT_IMPORT_COLUMNS: &[ProductImportColumn] = &[
    ProductImportColumn {
        title: COL_PRODUCT_CODE,
        field: "code",
        required: true,
        data_type: "string",
        example: "FAB-001",
    },
    ProductImportColumn {
        title: COL_PRODUCT_NAME,
        field: "name",
        required: true,
        data_type: "string",
        example: "纯棉坯布",
    },
    ProductImportColumn {
        title: COL_PRODUCT_TYPE,
        field: "product_type",
        required: true,
        data_type: "string",
        example: "坯布",
    },
    ProductImportColumn {
        title: COL_CATEGORY_ID,
        field: "category_id",
        required: false,
        data_type: "integer",
        example: "1",
    },
    ProductImportColumn {
        title: COL_SPECIFICATION,
        field: "specification",
        required: false,
        data_type: "string",
        example: "40S*40S",
    },
    ProductImportColumn {
        title: COL_UNIT,
        field: "unit",
        required: true,
        data_type: "string",
        example: "米",
    },
    ProductImportColumn {
        title: COL_STANDARD_PRICE,
        field: "standard_price",
        required: false,
        data_type: "decimal",
        example: "15.50",
    },
    ProductImportColumn {
        title: COL_COST_PRICE,
        field: "cost_price",
        required: false,
        data_type: "decimal",
        example: "12.00",
    },
    ProductImportColumn {
        title: COL_COMPOSITION,
        field: "fabric_composition",
        required: false,
        data_type: "string",
        example: "100%棉",
    },
    ProductImportColumn {
        title: COL_YARN_COUNT,
        field: "yarn_count",
        required: false,
        data_type: "string",
        example: "40S",
    },
    ProductImportColumn {
        title: COL_DENSITY,
        field: "density",
        required: false,
        data_type: "string",
        example: "133*72",
    },
    ProductImportColumn {
        title: COL_WIDTH,
        field: "width",
        required: false,
        data_type: "decimal",
        example: "150.00",
    },
    ProductImportColumn {
        title: COL_GRAM_WEIGHT,
        field: "gram_weight",
        required: false,
        data_type: "decimal",
        example: "120.00",
    },
    ProductImportColumn {
        title: COL_STRUCTURE,
        field: "structure",
        required: false,
        data_type: "string",
        example: "平纹",
    },
    ProductImportColumn {
        title: COL_FINISH,
        field: "finish",
        required: false,
        data_type: "string",
        example: "防水",
    },
    ProductImportColumn {
        title: COL_MIN_ORDER_QUANTITY,
        field: "min_order_quantity",
        required: false,
        data_type: "decimal",
        example: "1000",
    },
    ProductImportColumn {
        title: COL_LEAD_TIME,
        field: "lead_time",
        required: false,
        data_type: "integer",
        example: "15",
    },
    ProductImportColumn {
        title: COL_STATUS,
        field: "status",
        required: false,
        data_type: "string",
        example: master_data::ACTIVE,
    },
    ProductImportColumn {
        title: COL_DESCRIPTION,
        field: "description",
        required: false,
        data_type: "string",
        example: "高品质纯棉坯布",
    },
    ProductImportColumn {
        title: COL_BARCODE,
        field: "barcode",
        required: false,
        data_type: "string",
        example: "6901234567890",
    },
    ProductImportColumn {
        title: COL_EXECUTION_STANDARD,
        field: "execution_standard",
        required: false,
        data_type: "string",
        example: "GB/T 406-2018",
    },
    ProductImportColumn {
        title: COL_FACTORY_NAME,
        field: "factory_name",
        required: false,
        data_type: "string",
        example: "示例纺织有限公司",
    },
    ProductImportColumn {
        title: COL_FACTORY_ADDRESS,
        field: "factory_address",
        required: false,
        data_type: "string",
        example: "浙江省杭州市示例路1号",
    },
    ProductImportColumn {
        title: COL_PRODUCT_GRADE,
        field: "product_grade",
        required: false,
        data_type: "string",
        example: "合格品",
    },
];

/// 产品文件导入权威表的只读引用。
/// 调用方 本模块 模板与导出与解析三面 、通用模板面 build_products_template 与
/// 通用导入腿按位 zip 读列。入参 无，返回常量表本身，不落库也不改写。
pub(crate) fn product_import_columns() -> &'static [ProductImportColumn] {
    PRODUCT_IMPORT_COLUMNS
}

/// CSV 导入行校验后的必填字段
/// D08-1 第二梯队拆分：用于封装 import_products_from_csv 中校验后的 4 个必填字段；（产品编码、产品名称、产品类型、计量单位），避免 validate_import_row 返回 4-tuple。
pub(crate) struct ValidatedRowFields {
    code: String,
    name: String,
    product_type: String,
    unit: String,
}

/// 产品导入行级校验失败项（两条导入腿共用的错误载体）。
pub(crate) struct RowFieldIssue {
    /// 权威表列名，用户文件里的真实列头文本（错误归属的列，不是「数据库」这类无信息名）。
    pub(crate) column: &'static str,
    /// 用户可见文案，只描述该列的录入规则。
    pub(crate) message: String,
    /// 单元格原文，供导入结果表回显定位。
    pub(crate) value: String,
}

impl ProductService {
    /// 通用数据导入中心 /import/excel 产品腿的列序契约：把按位提交的一维字符串行
    /// 按权威表列序 zip 成「表头到值」的 Map，供与 /products/import 同一套行解析与判据消费。
    /// 模板面由同一张表生成，本 zip 与模板列序同源：增删列或调换顺序只改权威表一处，
    /// 两腿自动跟随，不存在模板改了而按位读列错位的可能。
    /// 行越界的位视作该列整列未提交（必填校验按「缺少X列」语义拒绝或可选列落 None）；
    /// 空串单元格照常入 Map（必填校验按「不能为空」语义拒绝），不当成缺列吞掉。
    pub(crate) fn product_import_map_from_positional(row: &[String]) -> HashMap<String, String> {
        let mut map = HashMap::new();
        for (idx, col) in Self::product_import_column_refs().iter().enumerate() {
            if let Some(value) = row.get(idx) {
                map.insert(col.title.to_string(), value.clone());
            }
        }
        map
    }

    /// 权威表引用的 impl 面入口（模板 导出 解析与通用腿共用同一只读清单）。
    fn product_import_column_refs() -> &'static [ProductImportColumn] {
        product_import_columns()
    }

    /// T1: 导出产品数据为结构化格式（去除 CSV 中转）
    /// show_cost_price 由调用方按是否 admin 传入：成本价在 JSON 列表腿本就对非 admin 置空，
    /// 导出面若整表带出即绕过同一策略，故此处与列表腿同源收口（置空该列，不删列）。
    pub async fn export_products_to_xlsx(
        &self,
        category_id: Option<i32>,
        status: Option<String>,
        search: Option<String>,
        show_cost_price: bool,
    ) -> Result<(Vec<String>, Vec<Vec<String>>), AppError> {
        // 查询产品列表（单页拉取全量用于导出）
        let (products, _total) = self
            .list_products(1, 10000, category_id, status, search)
            .await?;

        // 构建表头
        let headers = Self::build_product_csv_headers();

        // 构建行数据（直接转为 Vec<Vec<String>>，无需 CSV 中转）
        let mut rows: Vec<Vec<String>> = products
            .iter()
            .map(|p| {
                let row = Self::build_product_csv_row(p);
                headers
                    .iter()
                    .map(|h| row.get(h).cloned().unwrap_or_default())
                    .collect()
            })
            .collect();

        if !show_cost_price {
            crate::utils::field_mask::blank_column_by_title(&headers, &mut rows, COL_COST_PRICE);
        }

        Ok((headers, rows))
    }

    /// 构建产品 CSV 表头：逐列取权威表列名，零手抄（与模板与解析同一张表）。
    fn build_product_csv_headers() -> Vec<String> {
        Self::product_import_column_refs()
            .iter()
            .map(|c| c.title.to_string())
            .collect()
    }

    /// 将单个产品转为 CSV 行 HashMap（键为权威表列名）。
    /// 调用方 本模块导出腿 与 通用数据导入中心导出面 import_export_ops::export::export_products。
    /// 每列都必须落一个键：漏键的行会在两条按表头取值的导出腿上静默成空列，新增入参列时这里必须同步补值。
    pub(crate) fn build_product_csv_row(p: &product::Model) -> HashMap<String, String> {
        let mut row = HashMap::new();
        row.insert(COL_PRODUCT_CODE.to_string(), p.code.clone());
        row.insert(COL_PRODUCT_NAME.to_string(), p.name.clone());
        row.insert(COL_PRODUCT_TYPE.to_string(), p.product_type.clone());
        row.insert(
            COL_CATEGORY_ID.to_string(),
            p.category_id.map(|id| id.to_string()).unwrap_or_default(),
        );
        row.insert(
            COL_SPECIFICATION.to_string(),
            p.specification.clone().unwrap_or_default(),
        );
        row.insert(COL_UNIT.to_string(), p.unit.clone());
        row.insert(
            COL_STANDARD_PRICE.to_string(),
            p.standard_price
                .map(|price| price.to_string())
                .unwrap_or_default(),
        );
        row.insert(
            COL_COST_PRICE.to_string(),
            p.cost_price
                .map(|price| price.to_string())
                .unwrap_or_default(),
        );
        row.insert(
            COL_COMPOSITION.to_string(),
            p.fabric_composition.clone().unwrap_or_default(),
        );
        row.insert(
            COL_YARN_COUNT.to_string(),
            p.yarn_count.clone().unwrap_or_default(),
        );
        row.insert(
            COL_DENSITY.to_string(),
            p.density.clone().unwrap_or_default(),
        );
        row.insert(
            COL_WIDTH.to_string(),
            p.width.map(|w| w.to_string()).unwrap_or_default(),
        );
        row.insert(
            COL_GRAM_WEIGHT.to_string(),
            p.gram_weight.map(|g| g.to_string()).unwrap_or_default(),
        );
        row.insert(
            COL_STRUCTURE.to_string(),
            p.structure.clone().unwrap_or_default(),
        );
        row.insert(COL_FINISH.to_string(), p.finish.clone().unwrap_or_default());
        row.insert(
            COL_MIN_ORDER_QUANTITY.to_string(),
            p.min_order_quantity
                .map(|m| m.to_string())
                .unwrap_or_default(),
        );
        row.insert(
            COL_LEAD_TIME.to_string(),
            p.lead_time.map(|l| l.to_string()).unwrap_or_default(),
        );
        row.insert(COL_STATUS.to_string(), p.status.clone());
        row.insert(
            COL_DESCRIPTION.to_string(),
            p.description.clone().unwrap_or_default(),
        );
        row.insert(
            COL_BARCODE.to_string(),
            p.barcode.clone().unwrap_or_default(),
        );
        row.insert(
            COL_EXECUTION_STANDARD.to_string(),
            p.execution_standard.clone().unwrap_or_default(),
        );
        row.insert(
            COL_FACTORY_NAME.to_string(),
            p.factory_name.clone().unwrap_or_default(),
        );
        row.insert(
            COL_FACTORY_ADDRESS.to_string(),
            p.factory_address.clone().unwrap_or_default(),
        );
        row.insert(
            COL_PRODUCT_GRADE.to_string(),
            p.product_grade.clone().unwrap_or_default(),
        );
        row
    }

    /// T1: 生成产品导入模板（结构化格式，去除 CSV 中转）。
    /// 表头与示例行逐列取权威表（表头文本等于解析键，示例行自身可过建品判据），
    /// 与解析面与导出面同源，不存在模板与解析各抄一份再漂移的形态。
    pub fn generate_product_import_template_xlsx()
    -> Result<(Vec<String>, Vec<Vec<String>>), AppError> {
        let columns = Self::product_import_column_refs();
        let headers: Vec<String> = columns.iter().map(|c| c.title.to_string()).collect();
        let example_row: Vec<String> = columns.iter().map(|c| c.example.to_string()).collect();
        Ok((headers, vec![example_row]))
    }

    /// 解析可选字符串字段（缺失或空字符串视为 None）（D08-1 第二梯队拆分：提取 import_products_from_csv 中重复的字符串解析模式。）
    fn parse_optional_csv_string(row: &HashMap<String, String>, field: &str) -> Option<String> {
        row.get(field).filter(|v| !v.is_empty()).cloned()
    }

    /// 解析可选十进制字段（缺失、空字符串或解析失败均为 None）。DECIMAL 列按十进制直接解析，不做 f64 往返。
    fn parse_optional_csv_decimal(row: &HashMap<String, String>, field: &str) -> Option<Decimal> {
        row.get(field).and_then(|v| {
            if v.is_empty() {
                None
            } else {
                Decimal::from_str(v).ok()
            }
        })
    }

    /// 解析可选 i32 字段（缺失、空字符串或解析失败均为 None）（D08-1 第二梯队拆分：提取 import_products_from_csv 中重复的 i32 解析模式。）
    fn parse_optional_csv_i32(row: &HashMap<String, String>, field: &str) -> Option<i32> {
        row.get(field).and_then(|v| {
            if v.is_empty() {
                None
            } else {
                v.parse::<i32>().ok()
            }
        })
    }

    /// 解析状态字段，缺省为 master_data::ACTIVE（D08-1 第二梯队拆分：提取 import_products_from_csv 中状态字段的默认值逻辑。）
    fn parse_csv_status_field(row: &HashMap<String, String>) -> String {
        row.get(COL_STATUS)
            .filter(|v| !v.is_empty())
            .map(|v| v.to_string())
            .unwrap_or_else(|| master_data::ACTIVE.to_string())
    }

    /// 行级必填与形态校验的唯一实现（覆盖旧的四列独立校验器形态）。
    /// 调用方 CSV 导入腿 `validate_csv_import_row` 包装 与 通用数据导入中心产品腿
    /// （import_export_ops::import::import_product_row）。
    /// 规则：必填集合逐列读权威表 required 位（不在本函数另列清单）；整列缺失报「缺少X列」，
    /// 空值文案由 FieldValidator::required 产出；编码与名称再补长度上限，
    /// 产品类型再补词表枚举（product_type::ALL_TYPES），文案同出 FieldValidator 家族。
    /// 首错即返（单行一条错误），与两条腿既有「一行一错」的计数语义一致，
    /// 不放宽任何一列的判据。
    pub(crate) fn validate_import_row_fields(
        row: &HashMap<String, String>,
    ) -> Result<ValidatedRowFields, RowFieldIssue> {
        let mut checked: HashMap<&str, String> = HashMap::new();
        for col in Self::product_import_column_refs()
            .iter()
            .filter(|c| c.required)
        {
            let Some(raw) = row.get(col.title) else {
                return Err(RowFieldIssue {
                    column: col.title,
                    message: format!("缺少{}列", col.title),
                    value: String::new(),
                });
            };
            if let Err(e) = FieldValidator::required(raw, col.title) {
                return Err(RowFieldIssue {
                    column: col.title,
                    message: e,
                    value: raw.clone(),
                });
            }
            checked.insert(col.title, raw.trim().to_string());
        }
        // 四列取用：上方循环对 required 列已逐一放行，此处只可能因权威表 required 位被
        // 单边改动而不可达；按缺值拒绝保持 fail-closed，不放行也不 panic。
        let take = |title: &'static str| -> Result<String, RowFieldIssue> {
            checked.get(title).cloned().ok_or_else(|| RowFieldIssue {
                column: title,
                message: format!("{} 不能为空", title),
                value: row.get(title).cloned().unwrap_or_default(),
            })
        };
        let code = take(COL_PRODUCT_CODE)?;
        let name = take(COL_PRODUCT_NAME)?;
        let product_type = take(COL_PRODUCT_TYPE)?;
        let unit = take(COL_UNIT)?;

        if let Err(e) = FieldValidator::max_length(&code, COL_PRODUCT_CODE, 50) {
            return Err(RowFieldIssue {
                column: COL_PRODUCT_CODE,
                message: e,
                value: code,
            });
        }
        if let Err(e) = FieldValidator::max_length(&name, COL_PRODUCT_NAME, 200) {
            return Err(RowFieldIssue {
                column: COL_PRODUCT_NAME,
                message: e,
                value: name,
            });
        }
        let product_type = match FieldValidator::enum_value(
            &product_type,
            COL_PRODUCT_TYPE,
            product_type::ALL_TYPES,
        ) {
            Ok(v) => v,
            Err(e) => {
                return Err(RowFieldIssue {
                    column: COL_PRODUCT_TYPE,
                    message: e,
                    value: product_type,
                });
            }
        };
        Ok(ValidatedRowFields {
            code,
            name,
            product_type,
            unit,
        })
    }

    /// 校验行的必填字段（产品编码、产品名称、产品类型、计量单位）
    /// D08-1 第二梯队拆分：从 import_products_from_csv 提取的必填字段校验协调逻辑，；校验失败时向 result 添加错误并返回 None，全部通过则返回 ValidatedRowFields。
    fn validate_csv_import_row(
        row: &HashMap<String, String>,
        row_num: usize,
        result: &mut ImportResult,
    ) -> Option<ValidatedRowFields> {
        match Self::validate_import_row_fields(row) {
            Ok(v) => Some(v),
            Err(issue) => {
                result.add_error(
                    row_num,
                    issue.column.to_string(),
                    issue.message,
                    issue.value,
                );
                None
            }
        }
    }

    /// 基于校验后的必填字段和行数据构建 CreateProductArgs（D08-1 第二梯队拆分：从 import_products_from_csv 提取的字段解析 + 参数对象构建逻辑。）
    /// 调用方 CSV 导入腿与通用数据导入中心产品腿（两腿共用同一构造，列名解析键全部来自权威表常量）。
    pub(crate) fn build_csv_create_args(
        row: &HashMap<String, String>,
        validated: &ValidatedRowFields,
    ) -> CreateProductArgs {
        let category_id = Self::parse_optional_csv_i32(row, COL_CATEGORY_ID);
        let barcode = Self::parse_optional_csv_string(row, COL_BARCODE);
        let standard_price = Self::parse_optional_csv_decimal(row, COL_STANDARD_PRICE);
        let cost_price = Self::parse_optional_csv_decimal(row, COL_COST_PRICE);
        let specification = Self::parse_optional_csv_string(row, COL_SPECIFICATION);
        let fabric_composition = Self::parse_optional_csv_string(row, COL_COMPOSITION);
        let yarn_count = Self::parse_optional_csv_string(row, COL_YARN_COUNT);
        let density = Self::parse_optional_csv_string(row, COL_DENSITY);
        let width = Self::parse_optional_csv_decimal(row, COL_WIDTH);
        let gram_weight = Self::parse_optional_csv_decimal(row, COL_GRAM_WEIGHT);
        let structure = Self::parse_optional_csv_string(row, COL_STRUCTURE);
        let finish = Self::parse_optional_csv_string(row, COL_FINISH);
        let min_order_quantity = Self::parse_optional_csv_decimal(row, COL_MIN_ORDER_QUANTITY);
        let lead_time = Self::parse_optional_csv_i32(row, COL_LEAD_TIME);
        let description = Self::parse_optional_csv_string(row, COL_DESCRIPTION);
        let status = Self::parse_csv_status_field(row);
        let execution_standard = Self::parse_optional_csv_string(row, COL_EXECUTION_STANDARD);
        let factory_name = Self::parse_optional_csv_string(row, COL_FACTORY_NAME);
        let factory_address = Self::parse_optional_csv_string(row, COL_FACTORY_ADDRESS);
        let product_grade = Self::parse_optional_csv_string(row, COL_PRODUCT_GRADE);

        CreateProductArgs {
            name: validated.name.clone(),
            code: validated.code.clone(),
            barcode,
            category_id,
            specification,
            unit: validated.unit.clone(),
            standard_price,
            cost_price,
            description,
            status,
            // CSV 行的产品类型列已由 validate_import_row_fields 校验为必填且在枚举词表内，
            // 这里包成 Option 交给 create_product 的统一必录校验（面料类再要求幅宽/克重/成分）。
            product_type: Some(validated.product_type.clone()),
            fabric_composition,
            yarn_count,
            density,
            width,
            gram_weight,
            structure,
            finish,
            min_order_quantity,
            lead_time,
            execution_standard,
            factory_name,
            factory_address,
            product_grade,
            // CSV 导入权威表不含带针幅宽/克重公差/氨纶含量/用途/安全类别列，保持 NULL（不臆造换算值与定级）。
            width_needle_included_cm: None,
            weight_tolerance: None,
            spandex_percent: None,
            usage: None,
            safety_category: None,
            // CSV 导入权威表不含匹卷换算元数据列，保持 NULL（不臆造换算值）
            meters_per_piece: None,
        }
    }

    /// 建品失败的列名归属：把行级错误的真实原因归到权威表的实际列名。
    /// 判据族（services::product_service::validate_product_create_fields 与
    /// validate_product_numeric_domains）的每条文案都含对应列名（有效幅宽 克重 面料成分 产品类型
    /// 订量 交货期 标准价格 成本价格），按表列名子串命中即归属该列；
    /// 外键预校验文案「产品分类 {id} 不存在」归到 类别ID；
    /// 其余无列指向的整行级失败（唯一约束 数据库故障等）归到行身份列 产品编码
    /// （错误 value 即携带该行编码，可直接定位行）。不再落「数据库」这种无信息列名。
    /// 判据文案若改动措辞，必须同步复核本映射的关键词覆盖（面料三列与产品类型的
    /// 逐条文案可命中性由解析级自检工装钉住，不靠运行时兜底）。
    fn product_import_error_column(message: &str) -> &'static str {
        for title in [
            COL_COMPOSITION,
            COL_WIDTH,
            COL_GRAM_WEIGHT,
            COL_MIN_ORDER_QUANTITY,
            COL_LEAD_TIME,
            COL_STANDARD_PRICE,
            COL_COST_PRICE,
            COL_PRODUCT_TYPE,
        ] {
            if message.contains(title) {
                return title;
            }
        }
        if message.contains("产品分类") {
            return COL_CATEGORY_ID;
        }
        COL_PRODUCT_CODE
    }

    /// 处理 CSV 单行：校验 → 构建 args → 创建产品 → 写结果
    /// D08-1 第二梯队拆分：从 import_products_from_csv 提取的单行处理流程，；校验失败时直接返回（外层循环已 add_total，无需补偿）。
    async fn process_csv_import_row(
        &self,
        row: &HashMap<String, String>,
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
                let text = e.to_string();
                let column = Self::product_import_error_column(&text);
                result.add_error(
                    row_num,
                    column.to_string(),
                    format!("创建产品失败: {}", e),
                    code,
                );
            }
        }
    }

    /// 从产品 CSV 数据导入
    pub async fn import_products_from_csv(&self, data: &[u8]) -> Result<ImportResult, AppError> {
        let records = CsvImporter::parse(data)?;
        self.import_records(records).await
    }

    /// 从产品 xlsx 数据导入
    pub async fn import_products_from_xlsx(&self, data: &[u8]) -> Result<ImportResult, AppError> {
        let records = XlsxImporter::parse(data)?;
        self.import_records(records).await
    }

    /// 按文件格式自动分派导入（xlsx / csv）
    ///
    /// 修复背景：此前无论上传 xlsx 还是 csv，一律走 CSV 文本解析，
    /// 导致上传 xlsx 必然失败于「无效的 UTF-8 数据」。
    pub async fn import_products_from_file(
        &self,
        data: &[u8],
        file_name: &str,
    ) -> Result<ImportResult, AppError> {
        if is_xlsx_file(data, file_name) {
            self.import_products_from_xlsx(data).await
        } else {
            self.import_products_from_csv(data).await
        }
    }

    /// 共用：把解析后的记录逐行落库（CSV 与 xlsx 共用同一套行处理逻辑）
    async fn import_records(
        &self,
        records: Vec<HashMap<String, String>>,
    ) -> Result<ImportResult, AppError> {
        let mut result = ImportResult::new();

        for (row_idx, row) in records.iter().enumerate() {
            result.add_total();
            let row_num = row_idx + 2;
            self.process_csv_import_row(row, row_num, &mut result).await;
        }

        Ok(result)
    }
}

/// 判断上传文件是否应按 xlsx 解析
///
/// 优先按扩展名判定；扩展名缺失或未知时回退到 magic bytes
/// （xlsx 为 ZIP 容器，首 4 字节 `50 4B 03 04`，仅靠后缀判断可被绕过）。
fn is_xlsx_file(data: &[u8], file_name: &str) -> bool {
    let lower = file_name.to_ascii_lowercase();
    if lower.ends_with(".xlsx") || lower.ends_with(".xlsm") {
        return true;
    }
    if lower.ends_with(".csv") {
        return false;
    }
    XlsxImporter::verify_magic(data)
}
