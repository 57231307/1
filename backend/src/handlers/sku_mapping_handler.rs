//! 供应商商品/色号对照表 Handler（sku-mappings）
//!
//! 提供 CRUD、批量导入和 SKU 解析端点。

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::services::sku_mapping_service::{
    ImportMappingRow, SkuMappingQueryParams, SkuMappingService, UpsertSkuMappingInput,
};
use crate::utils::error::AppError;
use crate::utils::import_export::{CsvImporter, FieldValidator, XlsxImporter};
use crate::utils::response::{ApiResponse, PaginatedResponse};
use axum::{
    Json,
    extract::{Multipart, Path, Query, State},
};
use serde::Deserialize;
use std::collections::{BTreeSet, HashMap};
use tracing::info;

// =====================================================
// 请求 DTO
// =====================================================

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ListMappingsQuery {
    pub product_id: Option<i32>,
    pub supplier_id: Option<i32>,
    pub is_enabled: Option<bool>,
    pub keyword: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ResolveQuery {
    pub product_id: i32,
    pub color_id: Option<i32>,
    pub supplier_id: i32,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct CreateMappingRequest {
    pub product_id: i32,
    pub product_color_id: Option<i32>,
    pub supplier_id: i32,
    pub supplier_product_id: i32,
    pub supplier_product_color_id: Option<i32>,
    pub supplier_price: Option<String>,
    pub min_order_quantity: Option<String>,
    pub lead_time: Option<i32>,
    pub is_primary: Option<bool>,
    pub priority: Option<i32>,
    pub is_enabled: Option<bool>,
    pub remarks: Option<String>,
}

#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct UpdateMappingRequest {
    pub product_id: i32,
    pub product_color_id: Option<i32>,
    pub supplier_id: i32,
    pub supplier_product_id: i32,
    pub supplier_product_color_id: Option<i32>,
    pub supplier_price: Option<String>,
    pub min_order_quantity: Option<String>,
    pub lead_time: Option<i32>,
    pub is_primary: Option<bool>,
    pub priority: Option<i32>,
    pub is_enabled: Option<bool>,
    pub remarks: Option<String>,
}

// =====================================================
// 导入文件：列名 → ImportMappingRow 字段权威映射表
// =====================================================

/// SKU 对照导入文件表头列名常量（唯一真源）。
///
/// 列名与 `ImportMappingRow` 的 serde 字段名逐字符一致：导入文件表头 → 解析产物键 →
/// DTO 字段全程同一组名字，不做中英文转译层，杜绝「模板列名 ≠ DTO 字段名」的漂移。
/// 表头校验（缺列/未知列）、行解析、错误提示一律以下方映射表为唯一来源，
/// 其余位置不得再散落列名字面量。
const COL_PRODUCT_CODE: &str = "product_code";
const COL_COLOR_NO: &str = "color_no";
const COL_SUPPLIER_CODE: &str = "supplier_code";
const COL_SUPPLIER_PRODUCT_CODE: &str = "supplier_product_code";
const COL_SUPPLIER_COLOR_NO: &str = "supplier_color_no";
const COL_SUPPLIER_PRICE: &str = "supplier_price";
const COL_MIN_ORDER_QUANTITY: &str = "min_order_quantity";
const COL_LEAD_TIME: &str = "lead_time";
const COL_IS_PRIMARY: &str = "is_primary";
const COL_PRIORITY: &str = "priority";
const COL_IS_ENABLED: &str = "is_enabled";
const COL_REMARKS: &str = "remarks";

/// 权威「表头列名 → ImportMappingRow 字段」映射表：(列名, 是否必需)。
///
/// 必需列 = `ImportMappingRow` 中非 Option 的文本字段（product_code/supplier_code/
/// supplier_product_code）；其余列可选（整列缺失或单元格为空 → None，落库缺省语义由
/// `import_batch` 统一决定，本层不兜底）。增删导入列只改本表 + `build_sku_import_row`
/// 的对应赋值；集成测试
/// （tests/contract_wave11_sku_mapping_import_file_test.rs）从源码解析本表生成文件表头，
/// 防止测试手抄第二套列清单造成假绿。
const SKU_MAPPING_IMPORT_COLUMNS: &[(&str, bool)] = &[
    (COL_PRODUCT_CODE, true),
    (COL_COLOR_NO, false),
    (COL_SUPPLIER_CODE, true),
    (COL_SUPPLIER_PRODUCT_CODE, true),
    (COL_SUPPLIER_COLOR_NO, false),
    (COL_SUPPLIER_PRICE, false),
    (COL_MIN_ORDER_QUANTITY, false),
    (COL_LEAD_TIME, false),
    (COL_IS_PRIMARY, false),
    (COL_PRIORITY, false),
    (COL_IS_ENABLED, false),
    (COL_REMARKS, false),
];

// =====================================================
// Handlers
// =====================================================

/// GET /sku-mappings
pub async fn list_mappings(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<ListMappingsQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<serde_json::Value>>>, AppError> {
    let service = SkuMappingService::new(state.db.clone());
    let query = SkuMappingQueryParams {
        product_id: params.product_id,
        supplier_id: params.supplier_id,
        is_enabled: params.is_enabled,
        keyword: params.keyword,
        page: params.page,
        page_size: params.page_size,
    };

    let page = query.page.unwrap_or(1);
    let page_size = query.page_size.unwrap_or(20);

    let (items, total) = service.list(query).await?;

    let items_json: Vec<serde_json::Value> = items
        .into_iter()
        .map(|d| serde_json::to_value(d).map_err(AppError::from))
        .collect::<Result<Vec<_>, _>>()?;

    info!("SKU 对照表列表查询成功，共 {} 条", items_json.len());
    Ok(Json(ApiResponse::success_paginated(
        items_json, total, page, page_size,
    )))
}

/// GET /sku-mappings/{id}
pub async fn get_mapping(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SkuMappingService::new(state.db.clone());
    let dto = service.get(id).await?;
    info!("SKU 对照表查询成功，ID: {}", id);
    Ok(Json(ApiResponse::success(serde_json::to_value(dto)?)))
}

/// POST /sku-mappings
pub async fn create_mapping(
    State(state): State<AppState>,
    auth: AuthContext,
    Json(req): Json<CreateMappingRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 正在创建 SKU 对照", auth.user_id);

    let service = SkuMappingService::new(state.db.clone());
    let input = to_upsert_input(
        req.product_id,
        req.product_color_id,
        req.supplier_id,
        req.supplier_product_id,
        req.supplier_product_color_id,
        &req.supplier_price,
        &req.min_order_quantity,
        req.lead_time,
        req.is_primary,
        req.priority,
        req.is_enabled,
        req.remarks,
    )?;

    let dto = service.create(input, auth.user_id).await?;
    info!("SKU 对照创建成功，ID: {}", dto.id);
    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(dto)?,
        "对照记录创建成功",
    )))
}

/// PUT /sku-mappings/{id}
pub async fn update_mapping(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateMappingRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    info!("用户 {} 正在更新 SKU 对照 ID: {}", auth.user_id, id);

    let service = SkuMappingService::new(state.db.clone());
    let input = to_upsert_input(
        req.product_id,
        req.product_color_id,
        req.supplier_id,
        req.supplier_product_id,
        req.supplier_product_color_id,
        &req.supplier_price,
        &req.min_order_quantity,
        req.lead_time,
        req.is_primary,
        req.priority,
        req.is_enabled,
        req.remarks,
    )?;

    let dto = service.update(id, input, auth.user_id).await?;
    info!("SKU 对照更新成功，ID: {}", id);
    Ok(Json(ApiResponse::success_with_message(
        serde_json::to_value(dto)?,
        "对照记录更新成功",
    )))
}

/// DELETE /sku-mappings/{id}
pub async fn delete_mapping(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    info!("用户 {} 正在删除 SKU 对照 ID: {}", auth.user_id, id);

    let service = SkuMappingService::new(state.db.clone());
    service.delete(id).await?;
    info!("SKU 对照删除成功，ID: {}", id);
    Ok(Json(ApiResponse::success_with_message(
        (),
        "对照记录删除成功",
    )))
}

/// POST /sku-mappings/import（multipart/form-data 文件上传）
///
/// 前端 SkuMappingImportDialog 用 FormData append `file` 并显式声明
/// `Content-Type: multipart/form-data`（frontend/src/api/sku-mapping.ts 的
/// importSkuMappings），因此入参必须取 `Multipart`：声明 `Json<T>` 提取器会在提取器层
/// 就把该请求拒掉，导入功能不可达。范式与 product_handler::import_products 一致：
/// `Multipart` 必须保持最后一个提取器；
/// 服务端解析 CSV/xlsx 为「表头→值」行，经权威映射表构造类型行，
/// 逐行引用校验与 UPSERT 落库仍原样复用 `SkuMappingService::import_batch`，不建并行路径。
///
/// fail-visible（一律 400 类拒绝并点名行列，禁止静默成「0 条成功」）：
/// 未收到文件/空文件、体积超限、行数超限、表头缺必需列、表头含未知列、无数据行、
/// 必需单元格为空、单元格类型解析失败。出参 data 形状仍为 `ImportMappingResult`
/// （total_count/success_count/error_count/errors[{row,column,message,value}]）。
pub async fn import_mappings(
    State(state): State<AppState>,
    auth: AuthContext,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    // 与产品/CRM 导入同口径的文件体积上限（10MB；全局 12MB 请求体上限兜底，
    // 数值唯一出处见 constants::MAX_HTTP_BODY_BYTES 的注释约定）
    const MAX_IMPORT_SIZE: usize = 10 * 1024 * 1024;
    // 行数上限：SKU 对照行是窄行（每行约 200B），10MB 体积闸单独拦不住「窄行海量文件」
    //（约 5 万行），而 import_batch 每行要做多次引用 SELECT，行数必须有显式闸门防逐行查库
    // 放大；5000 = 单业务对照表的常规容量，超限要求拆分导入。
    const MAX_IMPORT_ROWS: usize = 5000;

    let mut file_bytes = Vec::new();
    let mut file_name = String::new();
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::bad_request(format!("文件上传解析失败: {}", e)))?
    {
        if field.name() == Some("file") {
            file_name = field.file_name().unwrap_or("").to_string();
            file_bytes = field
                .bytes()
                .await
                .map_err(|e| AppError::bad_request(format!("读取文件内容失败: {}", e)))?
                .to_vec();
        }
    }
    if file_bytes.is_empty() {
        return Err(AppError::bad_request("未收到文件或文件为空"));
    }
    if file_bytes.len() > MAX_IMPORT_SIZE {
        return Err(AppError::bad_request(format!(
            "文件大小超过限制 ({}MB)",
            MAX_IMPORT_SIZE / 1024 / 1024
        )));
    }

    info!(
        "用户 {} 正在批量导入 SKU 对照，上传文件 {}（{} 字节）",
        auth.user_id,
        file_name,
        file_bytes.len()
    );

    let records = parse_sku_import_file(&file_bytes, &file_name)?;
    if records.is_empty() {
        return Err(AppError::validation_displayable(
            "导入文件没有数据行（仅有表头或全部为空行）",
        ));
    }
    if records.len() > MAX_IMPORT_ROWS {
        return Err(AppError::validation_displayable(format!(
            "导入行数超过上限 {} 行（实际 {} 行），请拆分文件后分批导入",
            MAX_IMPORT_ROWS,
            records.len()
        )));
    }
    validate_sku_import_headers(&records)?;
    let rows = build_sku_import_rows(&records)?;

    let service = SkuMappingService::new(state.db.clone());
    let result = service.import_batch(rows, auth.user_id).await?;

    info!(
        "SKU 对照批量导入完成: 成功 {}，失败 {}",
        result.success_count, result.error_count
    );
    Ok(Json(ApiResponse::success(serde_json::to_value(result)?)))
}

/// GET /sku-mappings/resolve?product_id=&color_id=&supplier_id=
pub async fn resolve_sku(
    State(state): State<AppState>,
    _auth: AuthContext,
    Query(params): Query<ResolveQuery>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let service = SkuMappingService::new(state.db.clone());
    let resolved = service
        .resolve_supplier_sku(params.product_id, params.color_id, params.supplier_id)
        .await?
        .ok_or_else(|| AppError::business("该产品色号暂无供应商对照，请先维护对照表"))?;

    Ok(Json(ApiResponse::success(serde_json::to_value(resolved)?)))
}

// =====================================================
// Helpers
// =====================================================

/// 按扩展名 / magic 把上传文件解析为「表头→值」行（分派口径与
/// product_ops/import_export.rs 的 is_xlsx_file 一致：扩展名优先，未知扩展名回退 ZIP magic）。
///
/// `.xls`（OLE2 容器）显式点名拒绝：既有解析工具链 `XlsxImporter` 只接受 OOXML
/// （ZIP magic `50 4B 03 04`），把 .xls 塞进 CSV 文本解析只会以「无效的 UTF-8 数据」冒错；
/// 这里直接告知用户另存为 .xlsx / CSV，错误归因诚实且 fail-visible。
fn parse_sku_import_file(
    data: &[u8],
    file_name: &str,
) -> Result<Vec<HashMap<String, String>>, AppError> {
    let lower = file_name.to_ascii_lowercase();
    if lower.ends_with(".csv") {
        CsvImporter::parse(strip_utf8_bom(data))
    } else if lower.ends_with(".xlsx") || lower.ends_with(".xlsm") {
        XlsxImporter::parse(data)
    } else if lower.ends_with(".xls") {
        Err(AppError::validation_displayable(
            "暂不支持 .xls 旧格式，请将文件另存为 .xlsx 或使用 CSV 后导入",
        ))
    } else if XlsxImporter::verify_magic(data) {
        XlsxImporter::parse(data)
    } else {
        CsvImporter::parse(strip_utf8_bom(data))
    }
}

/// 剥离 UTF-8 BOM（EF BB BF）。
///
/// 真实形态所需而非臆测：Excel「CSV UTF-8」保存的文件带 BOM，首个表头会被解析成
/// "\u{FEFF}product_code"，与权威列名不匹配导致整文件被误拒。
fn strip_utf8_bom(data: &[u8]) -> &[u8] {
    data.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(data)
}

/// 表头级校验：必需列全局缺失 → 400 点名缺失列；出现权威表之外的列 → 400 点名未知列。
///
/// 两个解析器产物都以表头为键，因此「某必需列在所有数据行都没有键」即表头缺该列；
/// 单行尾部列数不足只是单元格缺失，不伪装成表头问题（由 `build_sku_import_row`
/// 的必需单元格校验处理）。文案只描述用户自己提交文件的字段，按 error.rs 模块文档
/// 属参数校验类、允许点名，故用 `validation_displayable`；不得含表名/SQL/SQLSTATE。
fn validate_sku_import_headers(records: &[HashMap<String, String>]) -> Result<(), AppError> {
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    for record in records {
        for key in record.keys() {
            seen.insert(key.as_str());
        }
    }

    let missing: Vec<&str> = SKU_MAPPING_IMPORT_COLUMNS
        .iter()
        .filter(|(column, required)| *required && !seen.contains(column))
        .map(|(column, _)| *column)
        .collect();
    if !missing.is_empty() {
        return Err(AppError::validation_displayable(format!(
            "导入文件表头缺少必需列: {}",
            missing.join(", ")
        )));
    }

    let unknown: Vec<&str> = seen
        .iter()
        .filter(|key| {
            !SKU_MAPPING_IMPORT_COLUMNS
                .iter()
                .any(|(column, _)| column == *key)
        })
        .copied()
        .collect();
    if !unknown.is_empty() {
        return Err(AppError::validation_displayable(format!(
            "导入文件表头包含未知列: {}",
            unknown.join(", ")
        )));
    }
    Ok(())
}

/// 解析行 → `ImportMappingRow`（整文件一次通过；任一行不合法即拒绝整个文件并点名该行）。
///
/// 不做「坏行剔除、好行继续」的部分导入：`import_batch` 的行级错误 row 序号按它收到的行
/// 序列从 1 计数，先剔除坏行会让落库错误的行号与文件行号错位；文件级问题在类型层就归 400。
fn build_sku_import_rows(
    records: &[HashMap<String, String>],
) -> Result<Vec<ImportMappingRow>, AppError> {
    records
        .iter()
        .enumerate()
        .map(|(idx, record)| build_sku_import_row(record, idx + 2))
        .collect()
}

/// 构造单行。`file_row` 为文件物理行号：第 1 行是表头，数据行自第 2 行起
/// （与产品导入的文件行号口径一致；import_batch 返回的行号是数据行序号，不含表头）。
fn build_sku_import_row(
    record: &HashMap<String, String>,
    file_row: usize,
) -> Result<ImportMappingRow, AppError> {
    let required_text = |column: &str| -> Result<String, AppError> {
        cell(record, column).map(str::to_string).ok_or_else(|| {
            AppError::validation_displayable(format!(
                "第 {} 行缺少必填列 {} 的值",
                file_row, column
            ))
        })
    };
    let optional_text =
        |column: &str| -> Option<String> { cell(record, column).map(str::to_string) };
    let optional_decimal = |column: &str| -> Result<Option<rust_decimal::Decimal>, AppError> {
        match cell(record, column) {
            None => Ok(None),
            Some(value) => FieldValidator::decimal(value, column)
                .map(Some)
                .map_err(|e| row_level_error(file_row, e)),
        }
    };
    let optional_integer = |column: &str| -> Result<Option<i32>, AppError> {
        match cell(record, column) {
            None => Ok(None),
            Some(value) => FieldValidator::integer(value, column)
                .map(Some)
                .map_err(|e| row_level_error(file_row, e)),
        }
    };
    let optional_boolean = |column: &str| -> Result<Option<bool>, AppError> {
        match cell(record, column) {
            None => Ok(None),
            Some(value) => FieldValidator::boolean(value, column)
                .map(Some)
                .map_err(|e| row_level_error(file_row, e)),
        }
    };

    Ok(ImportMappingRow {
        product_code: required_text(COL_PRODUCT_CODE)?,
        color_no: optional_text(COL_COLOR_NO),
        supplier_code: required_text(COL_SUPPLIER_CODE)?,
        supplier_product_code: required_text(COL_SUPPLIER_PRODUCT_CODE)?,
        supplier_color_no: optional_text(COL_SUPPLIER_COLOR_NO),
        supplier_price: optional_decimal(COL_SUPPLIER_PRICE)?,
        min_order_quantity: optional_decimal(COL_MIN_ORDER_QUANTITY)?,
        lead_time: optional_integer(COL_LEAD_TIME)?,
        is_primary: optional_boolean(COL_IS_PRIMARY)?,
        priority: optional_integer(COL_PRIORITY)?,
        is_enabled: optional_boolean(COL_IS_ENABLED)?,
        remarks: optional_text(COL_REMARKS),
    })
}

/// 读单元格：缺列、空串或仅空白 → None；其余 trim 后返回。
/// 与 FieldValidator 各校验的 trim 语义一致。
fn cell<'r>(record: &'r HashMap<String, String>, column: &str) -> Option<&'r str> {
    record
        .get(column)
        .map(String::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

/// 行级解析失败：冠以文件行号后透传 FieldValidator 的公开规则文案
/// （只描述用户提交字段，属可外显的参数校验类）。
fn row_level_error(file_row: usize, reason: String) -> AppError {
    AppError::validation_displayable(format!("第 {} 行：{}", file_row, reason))
}

#[allow(clippy::too_many_arguments)]
fn to_upsert_input(
    product_id: i32,
    product_color_id: Option<i32>,
    supplier_id: i32,
    supplier_product_id: i32,
    supplier_product_color_id: Option<i32>,
    supplier_price: &Option<String>,
    min_order_quantity: &Option<String>,
    lead_time: Option<i32>,
    is_primary: Option<bool>,
    priority: Option<i32>,
    is_enabled: Option<bool>,
    remarks: Option<String>,
) -> Result<UpsertSkuMappingInput, AppError> {
    use rust_decimal::Decimal;
    use std::str::FromStr;

    let sp = match supplier_price {
        Some(s) if !s.is_empty() => Some(Decimal::from_str(s).map_err(|_| {
            AppError::validation_displayable(format!("supplier_price 格式错误: {}", s))
        })?),
        _ => None,
    };
    let moq = match min_order_quantity {
        Some(s) if !s.is_empty() => Some(Decimal::from_str(s).map_err(|_| {
            AppError::validation_displayable(format!("min_order_quantity 格式错误: {}", s))
        })?),
        _ => None,
    };

    Ok(UpsertSkuMappingInput {
        product_id,
        product_color_id,
        supplier_id,
        supplier_product_id,
        supplier_product_color_id,
        supplier_price: sp,
        min_order_quantity: moq,
        lead_time,
        is_primary,
        priority,
        is_enabled,
        remarks,
    })
}
