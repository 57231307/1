use axum::{
    Extension, Json,
    extract::{Multipart, Path, Query, State},
};
use rust_decimal::Decimal;
use serde::Deserialize;
use std::str::FromStr;
use validator::Validate;

use crate::container::AppState;
use crate::middleware::auth_context::AuthContext;
use crate::models::product;
use crate::models::product_category;
use crate::models::product_color;
// 批次 213 P2-5 修复（v12 复审）：硬编码 "active" 替换为 master_data 常量
use crate::models::status::master_data;
use crate::services::product_service::{CreateProductArgs, ProductService, UpdateProductArgs};
use crate::utils::admin_checker;
use crate::utils::dual_unit_converter::DualUnitConverter;
use crate::utils::error::AppError;
use crate::utils::response::{ApiResponse, PaginatedResponse};

/// 查询参数 - 产品列表
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ProductListQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub category_id: Option<i32>,
    pub status: Option<String>,
    pub search: Option<String>,
}

/// 创建产品请求（面料行业版）
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct CreateProductRequest {
    #[validate(length(max = 200, message = "产品名称长度不能超过200个字符"))]
    pub name: Option<String>,
    #[validate(length(max = 50, message = "产品编码长度不能超过50个字符"))]
    pub code: Option<String>,
    #[validate(length(max = 100, message = "产品条码长度不能超过100个字符"))]
    pub barcode: Option<String>,
    pub category_id: Option<i32>,
    #[validate(length(max = 500, message = "规格型号长度不能超过500个字符"))]
    pub specification: Option<String>,
    #[validate(length(max = 20, message = "计量单位长度不能超过20个字符"))]
    pub unit: Option<String>,
    // DECIMAL 列入参统一用 Decimal：rust_decimal 的反序列化同时接受 JSON 数字与十进制字符串
    // （出参即字符串形态，编辑回显原样回传必须可用），价格两键与米数键同口径，不走 f64 往返。
    pub standard_price: Option<Decimal>,
    pub cost_price: Option<Decimal>,
    #[validate(length(max = 1000, message = "产品描述长度不能超过1000个字符"))]
    pub description: Option<String>,
    #[validate(length(max = 20, message = "状态长度不能超过20个字符"))]
    pub status: Option<String>,
    // 面料行业字段
    #[validate(length(max = 50, message = "产品类型长度不能超过50个字符"))]
    pub product_type: Option<String>,
    #[validate(length(max = 200, message = "面料成分长度不能超过200个字符"))]
    pub fabric_composition: Option<String>,
    #[validate(length(max = 50, message = "纱支长度不能超过50个字符"))]
    pub yarn_count: Option<String>,
    #[validate(length(max = 50, message = "密度长度不能超过50个字符"))]
    pub density: Option<String>,
    pub width: Option<Decimal>,
    pub gram_weight: Option<Decimal>,
    #[validate(length(max = 50, message = "组织结构长度不能超过50个字符"))]
    pub structure: Option<String>,
    #[validate(length(max = 100, message = "后整理长度不能超过100个字符"))]
    pub finish: Option<String>,
    // 带针幅宽与克重公差与氨纶含量：m0106 扩的规格列，DECIMAL 列统一收 Decimal（JSON 数字与十进制字符串两形态）。
    // 创建侧两态即可（缺席与 null 都落 NULL）。
    pub width_needle_included_cm: Option<Decimal>,
    pub weight_tolerance: Option<Decimal>,
    pub spandex_percent: Option<Decimal>,
    #[validate(length(max = 200, message = "用途长度不能超过200个字符"))]
    pub usage: Option<String>,
    #[validate(length(max = 1, message = "安全类别长度不能超过1个字符"))]
    pub safety_category: Option<String>,
    pub min_order_quantity: Option<Decimal>,
    pub lead_time: Option<i32>,
    // V15 P1 合规字段（《产品质量法》第 27 条）
    #[validate(length(max = 50, message = "执行标准号长度不能超过50个字符"))]
    pub execution_standard: Option<String>,
    #[validate(length(max = 200, message = "生产厂名长度不能超过200个字符"))]
    pub factory_name: Option<String>,
    #[validate(length(max = 500, message = "生产厂址长度不能超过500个字符"))]
    pub factory_address: Option<String>,
    #[validate(length(max = 10, message = "产品等级长度不能超过10个字符"))]
    pub product_grade: Option<String>,
    // 每匹米数（匹↔米换算元数据，一卷即一匹的现网口径下也服务按卷换算），
    // 产品表单不再采集，直连 API 与批量与导入仍可录入。
    pub meters_per_piece: Option<Decimal>,
}

/// 更新产品请求（面料行业版）
/// 每匹米数与五个 DECIMAL 数值列（价格两列、幅宽、克重、最小起订量）为三态部分更新，
/// 键缺席保持原值、显式 null 清空为 NULL、有值覆盖。
/// DECIMAL 列走三态的原因与米数同由：这些列的出参是十进制字符串，编辑清空若只发 null，
/// 单层 Option 下与键缺席不可区分，清空会被静默丢弃、落不回库里。
/// 其余字段维持单层 Option 的条件 Set 语义，键缺席即不更新该列。
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize, Validate)]
pub struct UpdateProductRequest {
    #[validate(length(max = 200, message = "产品名称长度不能超过200个字符"))]
    pub name: Option<String>,
    #[validate(length(max = 100, message = "产品条码长度不能超过100个字符"))]
    pub barcode: Option<String>,
    #[validate(length(max = 500, message = "规格型号长度不能超过500个字符"))]
    pub specification: Option<String>,
    #[validate(length(max = 20, message = "计量单位长度不能超过20个字符"))]
    pub unit: Option<String>,
    // DECIMAL 列入参统一用 Decimal：rust_decimal 的反序列化同时接受 JSON 数字与十进制字符串，
    // 编辑回显原样回传不再被「invalid type: string, expected f64」拒（与创建 DTO 同口径）。
    // 三态部分更新：键缺席保持原值、显式 null 清空为 NULL、有值覆盖。
    #[serde(default, deserialize_with = "double_option")]
    pub standard_price: Option<Option<Decimal>>,
    #[serde(default, deserialize_with = "double_option")]
    pub cost_price: Option<Option<Decimal>>,
    #[validate(length(max = 1000, message = "产品描述长度不能超过1000个字符"))]
    pub description: Option<String>,
    #[validate(length(max = 20, message = "状态长度不能超过20个字符"))]
    pub status: Option<String>,
    // 面料行业字段
    #[validate(length(max = 50, message = "产品类型长度不能超过50个字符"))]
    pub product_type: Option<String>,
    #[validate(length(max = 200, message = "面料成分长度不能超过200个字符"))]
    pub fabric_composition: Option<String>,
    #[validate(length(max = 50, message = "纱支长度不能超过50个字符"))]
    pub yarn_count: Option<String>,
    #[validate(length(max = 50, message = "密度长度不能超过50个字符"))]
    pub density: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub width: Option<Option<Decimal>>,
    #[serde(default, deserialize_with = "double_option")]
    pub gram_weight: Option<Option<Decimal>>,
    #[validate(length(max = 50, message = "组织结构长度不能超过50个字符"))]
    pub structure: Option<String>,
    #[validate(length(max = 100, message = "后整理长度不能超过100个字符"))]
    pub finish: Option<String>,
    // 带针幅宽与克重公差与氨纶含量：DECIMAL 列，更新侧走三态（键缺席保持原值、显式 null 清空、有值覆盖），
    // 与幅宽克重同由，出参为十进制字符串、编辑清空只发 null 时单层 Option 与缺席不可区分会静默丢弃。
    #[serde(default, deserialize_with = "double_option")]
    pub width_needle_included_cm: Option<Option<Decimal>>,
    #[serde(default, deserialize_with = "double_option")]
    pub weight_tolerance: Option<Option<Decimal>>,
    #[serde(default, deserialize_with = "double_option")]
    pub spandex_percent: Option<Option<Decimal>>,
    // 用途与安全类别为文本列，单层条件更新（键缺席即不更新该列），语义同组织与后整理。
    #[validate(length(max = 200, message = "用途长度不能超过200个字符"))]
    pub usage: Option<String>,
    #[validate(length(max = 1, message = "安全类别长度不能超过1个字符"))]
    pub safety_category: Option<String>,
    #[serde(default, deserialize_with = "double_option")]
    pub min_order_quantity: Option<Option<Decimal>>,
    // 交货期为数值列，清空诉求与 DECIMAL 各列同型，一并走三态：键缺席保持原值、显式 null 清空、有值覆盖。
    #[serde(default, deserialize_with = "double_option")]
    pub lead_time: Option<Option<i32>>,
    // V15 P1 合规字段（《产品质量法》第 27 条）
    #[validate(length(max = 50, message = "执行标准号长度不能超过50个字符"))]
    pub execution_standard: Option<String>,
    #[validate(length(max = 200, message = "生产厂名长度不能超过200个字符"))]
    pub factory_name: Option<String>,
    #[validate(length(max = 500, message = "生产厂址长度不能超过500个字符"))]
    pub factory_address: Option<String>,
    #[validate(length(max = 10, message = "产品等级长度不能超过10个字符"))]
    pub product_grade: Option<String>,
    // 每匹米数换算元数据（一卷即一匹的现网口径下也服务按卷换算，供 dual_unit_converter 读取）。
    // 三态部分更新：键缺席保持原值、显式 null 清空为 NULL、有值覆盖（清空通道即本形态存在的理由）。
    #[serde(default, deserialize_with = "double_option")]
    pub meters_per_piece: Option<Option<Decimal>>,
}

// ========== 色号管理相关结构体 ==========

/// 创建产品色号请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct CreateProductColorRequest {
    pub color_no: String,
    pub color_name: String,
    pub pantone_code: Option<String>,
    pub color_type: String,
    pub dye_formula: Option<String>,
    pub extra_cost: f64,
    /// 色牢度等级（GB 18401 九档，可空即未录入）。取值域由服务层词表判定。
    pub color_fastness_grade: Option<String>,
}

/// 更新产品色号请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct UpdateProductColorRequest {
    pub color_name: Option<String>,
    pub pantone_code: Option<String>,
    pub color_type: Option<String>,
    pub dye_formula: Option<String>,
    pub extra_cost: Option<f64>,
    pub is_active: Option<bool>,
    /// 色牢度等级（单层条件更新，键缺席即不改；显式空串按清空该列处理）
    pub color_fastness_grade: Option<String>,
}

/// 批量创建色号请求
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct BatchCreateColorsRequest {
    pub colors: Vec<CreateProductColorRequest>,
}

/// 导出产品查询参数
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ExportProductsQuery {
    pub category_id: Option<i32>,
    pub status: Option<String>,
    pub search: Option<String>,
    /// 敏感导出 fail-closed：导出审批令牌
    pub download_token: Option<String>,
}

/// 获取产品列表
use crate::utils::field_mask::mask_sensitive_fields;

/// 前端 `Product` 契约（frontend/src/api/product.ts:6）按 product_name / product_code /
/// is_active / price 读取，而 products 实体字段是 name / code / status / standard_price，
/// 导致产品列表与编辑弹窗的名称、编码、状态在 UI 上恒为空（18+ 个视图消费该契约，
/// 含销售/采购/BOM 的产品选择器）。这里在字段级权限过滤**之后**补别名，且别名只从
/// 过滤后的对象取值，避免用新键位绕过 hidden_fields 把已隐藏字段重新暴露出去。
/// TODO(doto)：长期应收敛为单一契约（后端出 VO 或前端改读实体字段），避免一名两值。
fn append_frontend_aliases(obj: &mut serde_json::Value) {
    let Some(map) = obj.as_object_mut() else {
        return;
    };
    for (alias, column) in [
        ("product_name", "name"),
        ("product_code", "code"),
        ("price", "standard_price"),
    ] {
        let value = map.get(column).cloned();
        if let Some(value) = value {
            map.insert(alias.to_string(), value);
        }
    }
    let is_active = map
        .get("status")
        .and_then(|status| status.as_str())
        .map(|status| status == master_data::ACTIVE);
    if let Some(is_active) = is_active {
        map.insert("is_active".to_string(), serde_json::json!(is_active));
    }
}

/// 派生只读键「每公斤出米数」（meters_per_kg）：由已下发的 gram_weight 与 width 两值算出，
/// 不可手输、不入库，随产品列表与详情出参下发。
/// 单一事实来源在服务层 DualUnitConverter::meters_per_kg，handler 只读字段过滤后的对象取值再调用，
/// 不在此重抄公式，避免换算面与出参面两处漂移。
/// 读的是本对象里经字段级权限过滤后的 gram_weight、width 两值；写给同一对象新增的只读键
/// meters_per_kg（serde_json 把 Decimal 序列化为字符串、None 序列化为 null）。
/// 为什么放在字段级权限过滤之后：与 append_frontend_aliases 同段，取的是过滤后的值，
/// 禁止用派生键位绕过 hidden_fields 把已隐藏的克重或幅宽反向重新暴露出去。
/// 缺任一列或值 ≤0 时该换算器返回 None，出参即为 null（不算成 0，避免把未录入当 0 米）。
/// pub：出参形状契约锁需以真实 JSON 对象直测本注入面（键存在、取值关系、缺值形态），
/// 形状照 map_product_fk_error 以生产入口直测的先例。
pub fn append_meters_per_kg(obj: &mut serde_json::Value) {
    // Decimal 序列化为字符串（rust_decimal 仅启 serde、未启 serde-float），这里反解回 Decimal 供换算器消费
    let (gram_weight, width) = match obj.as_object() {
        Some(map) => {
            let gram_weight = map
                .get("gram_weight")
                .and_then(|v| v.as_str())
                .and_then(|s| Decimal::from_str(s).ok());
            let width = map
                .get("width")
                .and_then(|v| v.as_str())
                .and_then(|s| Decimal::from_str(s).ok());
            (gram_weight, width)
        }
        None => return,
    };
    let meters = DualUnitConverter::meters_per_kg(gram_weight, width);
    // 派生键以对象补丁形态并入出参：键名在此显式声明，前端契约与出参真相同一处可查
    let patch = serde_json::json!({ "meters_per_kg": meters });
    if let (Some(map), Some(patch_map)) = (obj.as_object_mut(), patch.as_object()) {
        map.extend(patch_map.clone());
    }
}

/// 分类名称：products 表只存 category_id，而产品列表与详情的「分类名称」列需要主数据名称
async fn attach_category_names(db: &sea_orm::DatabaseConnection, rows: &mut [serde_json::Value]) {
    use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
    let cat_ids: Vec<i32> = rows
        .iter()
        .filter_map(|r| r.get("category_id").and_then(|v| v.as_i64()))
        .map(|id| id as i32)
        .collect();
    if cat_ids.is_empty() {
        return;
    }
    let categories = match product_category::Entity::find()
        .filter(product_category::Column::Id.is_in(cat_ids))
        .all(db)
        .await
    {
        Ok(list) => list,
        Err(e) => {
            tracing::error!(error = %e, "查询产品分类主数据失败，产品分类名称将为空");
            return;
        }
    };
    let name_map: std::collections::HashMap<i32, String> =
        categories.into_iter().map(|c| (c.id, c.name)).collect();
    for row in rows.iter_mut() {
        let Some(map) = row.as_object_mut() else {
            continue;
        };
        let Some(cid) = map.get("category_id").and_then(|v| v.as_i64()) else {
            continue;
        };
        match name_map.get(&(cid as i32)) {
            Some(name) => {
                map.insert(
                    "category_name".to_string(),
                    serde_json::Value::String(name.clone()),
                );
            }
            None => tracing::error!(
                category_id = cid,
                "产品行指向的产品分类主数据不存在，分类名称将为空"
            ),
        }
    }
}

pub async fn list_products(
    Extension(auth): Extension<AuthContext>,
    State(state): State<AppState>,
    Query(query): Query<ProductListQuery>,
) -> Result<Json<ApiResponse<PaginatedResponse<serde_json::Value>>>, AppError> {
    let product_service = ProductService::new(state.db.clone(), state.search_client.clone());

    let page = query.page.unwrap_or(1).clamp(1, 1000); // 批次 95 P3-3~8：分页 clamp 防 DoS
    let page_size = query.page_size.unwrap_or(10).clamp(1, 100);

    let (products, total) = product_service
        .list_products(
            page,
            page_size,
            query.category_id,
            query.status,
            query.search,
        )
        .await?;

    // Serialize each product model to Value and mask sensitive fields.
    // admin 判定走本仓唯一权威源 admin_checker::is_admin_role（roles.code='admin'，
    // role_id 缺失/查询失败 fail-closed=false，与本改造前"无角色必脱敏"的 fail-closed
    // 方向一致），每请求在循环外算一次并复用，
    // 禁止下放进逐行 map 闭包（admin_checker 带 5 分钟缓存 + DashMap 自死锁修复史）。
    let is_admin = match auth.role_id {
        Some(role_id) => admin_checker::is_admin_role(&state.db, role_id).await,
        None => false,
    };
    let mut masked_products: Vec<serde_json::Value> = products
        .into_iter()
        .map(|p| {
            serde_json::to_value(p)
                .map(|val| mask_sensitive_fields(val, is_admin))
                .unwrap_or_else(|e| {
                    tracing::error!("Product serialization failed: {:?}", e);
                    serde_json::Value::Null
                })
        })
        .collect();

    // B12-P2-2：字段级权限过滤
    if let Some(role_id) = auth.role_id {
        if let Ok(Some(permission)) = state
            .data_permission_service
            .get_role_data_permission(role_id, "product")
            .await
        {
            state.data_permission_service.filter_fields_batch(
                &mut masked_products,
                &permission.allowed_fields,
                &permission.hidden_fields,
            );
        }
    }

    // 权限过滤完成后再补前端契约别名，避免别名键绕过 hidden_fields 暴露被隐藏字段
    for item in masked_products.iter_mut() {
        append_frontend_aliases(item);
        append_meters_per_kg(item);
    }
    attach_category_names(&state.db, &mut masked_products).await;

    Ok(Json(ApiResponse::success(PaginatedResponse::new(
        masked_products,
        total,
        page,
        page_size,
    ))))
}
/// 获取产品详情
pub async fn get_product(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let product_service = ProductService::new(state.db.clone(), state.search_client.clone());
    let product = product_service.get_product(id).await?;
    let mut product_json = serde_json::to_value(product)?;

    // B12-P2-2：字段级权限过滤
    if let Some(role_id) = auth.role_id {
        if let Ok(Some(permission)) = state
            .data_permission_service
            .get_role_data_permission(role_id, "product")
            .await
        {
            state.data_permission_service.filter_fields(
                &mut product_json,
                &permission.allowed_fields,
                &permission.hidden_fields,
            );
        }
    }

    append_frontend_aliases(&mut product_json);
    append_meters_per_kg(&mut product_json);
    attach_category_names(&state.db, std::slice::from_mut(&mut product_json)).await;

    // 成本价掩码补到详情腿：列表腿早已对非 admin 置空 cost_price，详情腿此前原样下发，
    // 任何持 products:read 的角色都能用详情端点绕开列表掩码读到成本价。
    // admin 判定与列表腿同源（admin_checker::is_admin_role，role_id 缺失 fail-closed=false），
    // 放在别名与派生键装配之后，确保别名位不会把已置空的成本价再带出来。
    let is_admin = match auth.role_id {
        Some(role_id) => admin_checker::is_admin_role(&state.db, role_id).await,
        None => false,
    };
    let product_json = mask_sensitive_fields(product_json, is_admin);

    Ok(Json(ApiResponse::success(product_json)))
}

/// JSON 三态反序列化适配器（RFC 7386 JSON Merge Patch 的键缺席不等于显式 null 语义所需）。
///
/// 为何需要。serde_json 对双层 Option 的默认反序列化在遇到 JSON null 时将外层塌成 None，
/// 与键缺席不可区分，用户清空每匹米数就无法送达后端。本适配器把字段先按内层 Option
/// 反序列化再包一层。键缺席（配合 serde default）为 None 即保持原值，显式 null 为
/// Some(None) 即清空该列，有值为 Some(Some(v)) 即覆盖。
/// 与 ar_payment_handler 及 department_handler 等处的同型私有适配器一致。
fn double_option<'de, T, D>(de: D) -> Result<Option<Option<T>>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Deserialize::deserialize(de).map(Some)
}

/// 创建产品
pub async fn create_product(
    State(state): State<AppState>,
    _auth: AuthContext,
    Json(req): Json<CreateProductRequest>,
) -> Result<Json<ApiResponse<product::Model>>, AppError> {
    // 输入验证
    if let Err(e) = req.validate() {
        return Err(AppError::from(e));
    }

    let product_service = ProductService::new(state.db.clone(), state.search_client.clone());

    // 自动生成产品编码
    let code = match req.code {
        Some(c) if !c.is_empty() => c,
        _ => product_service.generate_product_code().await?,
    };

    let product = product_service
        .create_product(CreateProductArgs {
            name: req
                .name
                .unwrap_or_else(|| format!("产品_{}", chrono::Utc::now().timestamp())),
            code,
            barcode: req.barcode,
            category_id: req.category_id,
            specification: req.specification,
            unit: req.unit.unwrap_or_else(|| "个".to_string()),
            standard_price: req.standard_price,
            cost_price: req.cost_price,
            description: req.description,
            status: req
                .status
                .unwrap_or_else(|| master_data::ACTIVE.to_string()),
            // 产品类型不再缺省成成品布：新建必须显式提交，缺省会被 create_product 入口拒绝。
            // 停掉静默缺省是为了让「面料类才要求幅宽/克重/成分」的分级必填不被缺省绕过——
            // 没选类型的建单若当成面料会撞上必填、若当成非面料又漏掉真正的面料。
            // 必录判据（类型显式提交 + 面料类三列）统一落服务层 create_product，此处只透传请求值。
            product_type: req.product_type,
            fabric_composition: req.fabric_composition,
            yarn_count: req.yarn_count,
            density: req.density,
            width: req.width,
            gram_weight: req.gram_weight,
            structure: req.structure,
            finish: req.finish,
            width_needle_included_cm: req.width_needle_included_cm,
            weight_tolerance: req.weight_tolerance,
            spandex_percent: req.spandex_percent,
            usage: req.usage,
            safety_category: req.safety_category,
            min_order_quantity: req.min_order_quantity,
            lead_time: req.lead_time,
            execution_standard: req.execution_standard,
            factory_name: req.factory_name,
            factory_address: req.factory_address,
            product_grade: req.product_grade,
            meters_per_piece: req.meters_per_piece,
        })
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        product,
        "产品创建成功",
    )))
}

/// 更新产品
pub async fn update_product(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
    Json(req): Json<UpdateProductRequest>,
) -> Result<Json<ApiResponse<product::Model>>, AppError> {
    // 输入验证
    if let Err(e) = req.validate() {
        return Err(AppError::from(e));
    }

    let product_service = ProductService::new(state.db.clone(), state.search_client.clone());

    let product = product_service
        .update_product(UpdateProductArgs {
            id,
            name: req.name,
            barcode: req.barcode,
            specification: req.specification,
            unit: req.unit,
            standard_price: req.standard_price,
            cost_price: req.cost_price,
            description: req.description,
            status: req.status,
            product_type: req.product_type,
            fabric_composition: req.fabric_composition,
            yarn_count: req.yarn_count,
            density: req.density,
            width: req.width,
            gram_weight: req.gram_weight,
            structure: req.structure,
            finish: req.finish,
            width_needle_included_cm: req.width_needle_included_cm,
            weight_tolerance: req.weight_tolerance,
            spandex_percent: req.spandex_percent,
            usage: req.usage,
            safety_category: req.safety_category,
            min_order_quantity: req.min_order_quantity,
            lead_time: req.lead_time,
            execution_standard: req.execution_standard,
            factory_name: req.factory_name,
            factory_address: req.factory_address,
            product_grade: req.product_grade,
            meters_per_piece: req.meters_per_piece,
            // 入参：当前登录用户 id，透传给服务层 update_product 作为审计日志操作人
            user_id: auth.user_id,
        })
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        product,
        "产品更新成功",
    )))
}

/// 删除产品
pub async fn delete_product(
    State(state): State<AppState>,
    auth: AuthContext,
    Path(id): Path<i32>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let product_service = ProductService::new(state.db.clone(), state.search_client.clone());
    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    product_service.delete_product(id, auth.user_id).await?;
    Ok(Json(ApiResponse::success_with_message((), "产品删除成功")))
}

// ========== 色号管理接口 ==========

/// 色号列表查询参数（keyword 用于前端 el-select-v2 远程搜索高基数场景）
#[allow(dead_code, reason = "反序列化输入字段")]
#[derive(Debug, Deserialize)]
pub struct ColorListQuery {
    pub keyword: Option<String>,
    pub page: Option<u64>,
    pub page_size: Option<u64>,
}

/// 获取产品色号列表（支持关键词搜索与分页）
pub async fn list_product_colors(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(product_id): Path<i32>,
    Query(params): Query<ColorListQuery>,
) -> Result<Json<ApiResponse<Vec<product_color::Model>>>, AppError> {
    let product_service = ProductService::new(state.db.clone(), state.search_client.clone());
    let colors = product_service
        .list_product_colors_with_filter(product_id, params.keyword, params.page, params.page_size)
        .await?;
    Ok(Json(ApiResponse::success(colors)))
}

/// 创建产品色号
pub async fn create_product_color(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(product_id): Path<i32>,
    Json(req): Json<CreateProductColorRequest>,
) -> Result<Json<ApiResponse<product_color::Model>>, AppError> {
    let product_service = ProductService::new(state.db.clone(), state.search_client.clone());

    let color = product_service
        .create_product_color(
            product_id,
            crate::services::product_service::CreateProductColorInput {
                color_no: req.color_no,
                color_name: req.color_name,
                pantone_code: req.pantone_code,
                color_type: req.color_type,
                dye_formula: req.dye_formula,
                extra_cost: req.extra_cost,
                color_fastness_grade: req.color_fastness_grade,
            },
        )
        .await?;

    Ok(Json(ApiResponse::success_with_message(
        color,
        "色号创建成功",
    )))
}

/// 更新产品色号
pub async fn update_product_color(
    State(state): State<AppState>,
    auth: AuthContext,
    Path((_product_id, color_id)): Path<(i32, i32)>,
    Json(req): Json<UpdateProductColorRequest>,
) -> Result<Json<ApiResponse<product_color::Model>>, AppError> {
    let product_service = ProductService::new(state.db.clone(), state.search_client.clone());

    // 批次 330 v10 复审 P3 修复：使用参数对象替代多参数
    let params = crate::services::product_service::UpdateProductColorParams {
        id: color_id,
        color_name: req.color_name,
        pantone_code: req.pantone_code,
        color_type: req.color_type,
        dye_formula: req.dye_formula,
        extra_cost: req.extra_cost,
        is_active: req.is_active,
        color_fastness_grade: req.color_fastness_grade,
        user_id: auth.user_id,
    };
    let color = product_service.update_product_color(params).await?;

    Ok(Json(ApiResponse::success_with_message(
        color,
        "色号更新成功",
    )))
}

/// 删除产品色号
pub async fn delete_product_color(
    State(state): State<AppState>,
    auth: AuthContext,
    Path((_product_id, color_id)): Path<(i32, i32)>,
) -> Result<Json<ApiResponse<()>>, AppError> {
    let product_service = ProductService::new(state.db.clone(), state.search_client.clone());
    // 批次 94 P2-10：注入真实操作人 user_id 用于审计日志
    product_service
        .delete_product_color(color_id, auth.user_id)
        .await?;
    Ok(Json(ApiResponse::success_with_message((), "色号删除成功")))
}

/// 批量创建色号
pub async fn batch_create_colors(
    State(state): State<AppState>,
    _auth: AuthContext,
    Path(product_id): Path<i32>,
    Json(req): Json<BatchCreateColorsRequest>,
) -> Result<Json<ApiResponse<Vec<product_color::Model>>>, AppError> {
    let product_service = ProductService::new(state.db.clone(), state.search_client.clone());

    let colors_input: Vec<_> = req
        .colors
        .into_iter()
        .map(
            |c| crate::services::product_service::CreateProductColorInput {
                color_no: c.color_no,
                color_name: c.color_name,
                pantone_code: c.pantone_code,
                color_type: c.color_type,
                dye_formula: c.dye_formula,
                extra_cost: c.extra_cost,
                color_fastness_grade: c.color_fastness_grade,
            },
        )
        .collect();

    let colors = product_service
        .batch_create_product_colors(product_id, colors_input)
        .await?;
    let msg = format!("批量创建{}个色号成功", colors.len());
    Ok(Json(ApiResponse::success_with_message(colors, &msg)))
}

// ========== 数据导入导出接口 ==========

use crate::models::audit_log::{OperationType, Severity};
use crate::services::audit_log_service::{AuditEvent, AuditLogService};
use crate::services::export_approval_service::ExportApprovalService;
use crate::utils::xlsx_export::{XlsxTable, build_xlsx_response};
use std::sync::Arc;

/// 导出产品数据为 xlsx，端点为 GET /api/v1/erp/products/export
///
/// 功能：按本次请求的筛选取产品全集，落成 xlsx 响应体并写导出审计
/// 调用方：产品路由的导出端点
/// 入参：查询参数（category_id、status、search 与审批令牌）、应用状态、会话上下文
/// 传给谁：受理门交 ExportApprovalService::enforce_export_download_secured，导出腿交
/// run_export_leg，取数交 ProductService::export_products_to_xlsx，审计交本函数内构造的
/// AuditEvent，成功登记交 record_download
/// 存什么：不写业务表；产品行写进导出文件，审计行写审计日志表，令牌消费写导出审批表
/// 存哪里：不适用
///
/// 范围参数只收本次请求实际带的筛选项，与批准范围逐项比对。审计日志为异步写入，
/// 不阻塞响应。
pub async fn export_products(
    State(state): State<AppState>,
    auth: AuthContext,
    Query(query): Query<ExportProductsQuery>,
) -> Result<axum::response::Response, AppError> {
    // 敏感导出 fail-closed：受理门走加固入口（申请人本人 + 批准范围逐项比对）
    let approval_svc = ExportApprovalService::new(state.db.clone());
    let requested_scope = ExportApprovalService::scope_params(&[
        (
            "category_id",
            query.category_id.map(serde_json::Value::from),
        ),
        (
            "status",
            query.status.as_deref().map(serde_json::Value::from),
        ),
        (
            "search",
            query.search.as_deref().map(serde_json::Value::from),
        ),
    ]);
    let approval = approval_svc
        .enforce_export_download_secured(
            query.download_token.as_deref(),
            "product",
            auth.user_id,
            Some(requested_scope),
        )
        .await?;

    let (table, filename) = approval_svc
        .run_export_leg(approval.id, async {
            let product_service =
                ProductService::new(state.db.clone(), state.search_client.clone());

            // 提前 clone 查询条件用于审计日志（避免 service 调用 move 后 borrow of moved value）
            let audit_status = query.status.clone();
            let audit_search = query.search.clone();

            // 直接获取结构化数据，去除 CSV 中转
            // 成本价列与 JSON 列表腿同源收口：非 admin 导出整表时该列置空（保留列位不删列，
            // 否则导出件回填再导会按权威表列序错位）。admin 判定走唯一权威源
            // admin_checker::is_admin_role，在调用前算一次，不下放进逐行闭包。
            let show_cost_price = match auth.role_id {
                Some(role_id) => admin_checker::is_admin_role(&state.db, role_id).await,
                None => false,
            };
            let (headers, rows) = product_service
                .export_products_to_xlsx(
                    query.category_id,
                    query.status.clone(),
                    query.search.clone(),
                    show_cost_price,
                )
                .await?;

            let row_count = rows.len();
            let table = XlsxTable {
                sheet_name: "产品列表".to_string(),
                headers,
                rows,
            };

            let filename = format!(
                "products_export_{}",
                chrono::Utc::now().format("%Y%m%d_%H%M%S")
            );

            // 导出审计日志写入（best-effort，异步不阻塞响应）
            let event = AuditEvent {
                user_id: Some(auth.user_id),
                username: Some(auth.username.clone()),
                operation_type: OperationType::Export,
                severity: Severity::Info,
                resource_type: Some("product".to_string()),
                resource_id: None,
                resource_name: Some(format!("{}.xlsx", filename)),
                description: Some(format!(
                    "用户 {} 导出产品数据（共 {} 条）",
                    auth.username, row_count
                )),
                request_method: Some("GET".to_string()),
                request_path: Some("/api/v1/erp/products/export".to_string()),
                before_snapshot: None,
                after_snapshot: Some(serde_json::json!({
                    "format": "xlsx",
                    "total": row_count,
                    "category_id_filter": query.category_id,
                    "status_filter": audit_status,
                    "search_filter": audit_search,
                })),
            };
            let svc = Arc::new(AuditLogService::new(state.db.clone()));
            svc.record_async(event, None);

            // 敏感导出 fail-closed：记录令牌消费
            approval_svc
                .record_download(
                    approval.id,
                    filename.clone(),
                    row_count as i64,
                    String::new(),
                )
                .await?;

            Ok((table, filename))
        })
        .await?;

    build_xlsx_response(&table, &filename)
}

/// 导入产品数据（multipart/form-data 文件上传，前端用 FormData 发送 xlsx/csv 文件）
pub async fn import_products(
    State(state): State<AppState>,
    _auth: AuthContext,
    mut multipart: Multipart,
) -> Result<Json<ApiResponse<crate::utils::import_export::ImportResult>>, AppError> {
    let product_service = ProductService::new(state.db.clone(), state.search_client.clone());

    // 导入文件大小上限，与 crm import_leads 保持一致（10MB），
    // 避免超大文件一次性读入内存打爆进程。
    const MAX_IMPORT_SIZE: usize = 10 * 1024 * 1024;

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

    let result: crate::utils::import_export::ImportResult = product_service
        .import_products_from_file(&file_bytes, &file_name)
        .await?;

    let msg = if result.is_all_success() {
        format!("成功导入 {} 条产品数据", result.success_count)
    } else {
        format!(
            "导入完成：成功 {} 条，失败 {} 条，共 {} 条",
            result.success_count, result.error_count, result.total_count
        )
    };

    Ok(Json(ApiResponse::success_with_message(result, &msg)))
}

/// 获取产品导入模板
pub async fn get_product_import_template(
    State(_state): State<AppState>,
    _auth: AuthContext,
) -> Result<axum::response::Response, AppError> {
    // T1: 直接获取结构化数据，去除 CSV 中转
    let (headers, rows) = ProductService::generate_product_import_template_xlsx()?;

    let table = XlsxTable {
        sheet_name: "产品导入模板".to_string(),
        headers,
        rows,
    };

    build_xlsx_response(&table, "product_import_template")
}
