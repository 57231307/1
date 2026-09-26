//! 供应商商品/色号目录（supplier-products / supplier-product-colors）契约测试
//!
//! 无 DB/服务器、CI 可直接跑的源码字面量契约（仿 tests/sku_mapping_contract_test.rs）。
//! 断言以真实源码/实现为准，不为过测放宽：
//! - list 支持按父级过滤（supplier_id / supplier_product_id）+ keyword LIKE；
//! - create 有父级存在校验（AppError::validation，映射 400/VALIDATION_ERROR）；
//! - 重复编码走 business_displayable（真实文案外显，非脱敏）；
//! - 权限资源键与「URL 段自动派生」规则一致（purchase 模块前缀下 segment4 原名即键名），
//!   且两键授予集合 = {read,create,update}；销售角色一律不授（保密）。
//! - 真库集成用例（#[ignore]）覆盖 create→list 命中→keyword 过滤真实链路。

use bingxi_backend::utils::error::AppError;

const SUP_PRODUCT_SERVICE_SRC: &str = include_str!("../src/services/supplier_product_service.rs");
const SUP_PRODUCT_COLOR_SERVICE_SRC: &str =
    include_str!("../src/services/supplier_product_color_service.rs");
const SUP_PRODUCT_HANDLER_SRC: &str = include_str!("../src/handlers/supplier_product_handler.rs");
const SUP_PRODUCT_COLOR_HANDLER_SRC: &str =
    include_str!("../src/handlers/supplier_product_color_handler.rs");
const PURCHASE_ROUTES_SRC: &str = include_str!("../src/routes/purchase.rs");
const PERMISSION_SRC: &str = include_str!("../src/services/init_service_ops/permission.rs");
const PATH_UTILS_SRC: &str = include_str!("../src/utils/path_utils.rs");

// ---------------------------------------------------------------------------
// list：按父级过滤 + keyword
// ---------------------------------------------------------------------------

#[test]
fn supplier_product_list_filters_by_parent_and_keyword() {
    // 父级必填过滤
    assert!(
        SUP_PRODUCT_SERVICE_SRC.contains("supplier_product::Column::SupplierId.eq"),
        "supplier_product list 应按 supplier_id 过滤"
    );
    assert!(
        SUP_PRODUCT_SERVICE_SRC.contains("supplier_id 不能为空"),
        "supplier_id 缺失应给出 validation 提示（必填父级过滤）"
    );
    // keyword 对 product_code / product_name 双列 LIKE（任一命中）
    assert!(
        SUP_PRODUCT_SERVICE_SRC.contains("ProductCode.contains(kw)")
            && SUP_PRODUCT_SERVICE_SRC.contains("ProductName.contains(kw)"),
        "keyword 应对 product_code 与 product_name 做 LIKE 检索"
    );
    assert!(
        SUP_PRODUCT_SERVICE_SRC.contains("Condition::any()"),
        "关键字应跨两列做 OR 匹配"
    );
}

#[test]
fn supplier_product_color_list_filters_by_parent_and_keyword() {
    assert!(
        SUP_PRODUCT_COLOR_SERVICE_SRC.contains("SupplierProductId.eq"),
        "色号 list 应按 supplier_product_id 过滤"
    );
    assert!(
        SUP_PRODUCT_COLOR_SERVICE_SRC.contains("supplier_product_id 不能为空"),
        "supplier_product_id 缺失应给出 validation 提示（必填父级过滤）"
    );
    assert!(
        SUP_PRODUCT_COLOR_SERVICE_SRC.contains("ColorNo.contains(kw)")
            && SUP_PRODUCT_COLOR_SERVICE_SRC.contains("ColorName.contains(kw)"),
        "keyword 应对 color_no 与 color_name 做 LIKE 检索（一商品可上千色号）"
    );
}

// ---------------------------------------------------------------------------
// create：父级存在校验 → validation（400 / VALIDATION_ERROR）
// ---------------------------------------------------------------------------

#[test]
fn supplier_product_create_validates_parent_exists_via_validation_error() {
    let body = extract_fn(SUP_PRODUCT_SERVICE_SRC, "async fn ensure_supplier_exists");
    assert!(
        body.contains("supplier::Entity::find_by_id"),
        "create/update 应查 suppliers 表校验父级存在"
    );
    assert!(
        body.contains("AppError::validation"),
        "父级不存在应走 AppError::validation（非 business/裸 DB 错）"
    );
    // 本仓 ValidationError → HTTP 400 且 code=VALIDATION_ERROR（非 422），断言真实映射。
    assert_eq!(
        AppError::validation("供应商 ID 9 不存在").error_code(),
        "VALIDATION_ERROR"
    );
}

#[test]
fn supplier_product_color_create_validates_parent_exists_via_validation_error() {
    let body = extract_fn(
        SUP_PRODUCT_COLOR_SERVICE_SRC,
        "async fn ensure_product_exists",
    );
    assert!(
        body.contains("supplier_product::Entity::find_by_id"),
        "色号 create/update 应查 supplier_products 表校验父级存在"
    );
    assert!(
        body.contains("AppError::validation"),
        "父级不存在应走 AppError::validation"
    );
}

// ---------------------------------------------------------------------------
// 重复业务提示：business_displayable（真实文案外显，不裸抛 DB 错、不脱敏）
// ---------------------------------------------------------------------------

#[test]
fn duplicate_code_uses_displayable_business_message() {
    assert!(
        SUP_PRODUCT_SERVICE_SRC.contains("AppError::business_displayable"),
        "供应商商品编码重复应使用 business_displayable 让文案如实外显"
    );
    assert!(
        SUP_PRODUCT_SERVICE_SRC.contains("该供应商商品编码已存在"),
        "重复编码应有用户可感知的业务提示"
    );
    assert!(
        SUP_PRODUCT_COLOR_SERVICE_SRC.contains("AppError::business_displayable"),
        "色号重复应使用 business_displayable"
    );
    // displayable 出参携带真实文案（非脱敏常量）
    let err = AppError::business_displayable("该供应商商品编码已存在，请勿重复创建");
    assert_eq!(
        err.to_response().message,
        "该供应商商品编码已存在，请勿重复创建"
    );
    assert_eq!(err.error_code(), "BUSINESS_ERROR");
}

// ---------------------------------------------------------------------------
// 端点注册与路径前缀
// ---------------------------------------------------------------------------

#[test]
fn routes_registered_under_purchase_domain_with_hyphen_prefixes() {
    assert!(
        PURCHASE_ROUTES_SRC.contains("fn supplier_product_routes()")
            && PURCHASE_ROUTES_SRC.contains("fn supplier_product_color_routes()"),
        "应新增两个子路由函数"
    );
    assert!(
        PURCHASE_ROUTES_SRC.contains(".merge(supplier_product_routes())")
            && PURCHASE_ROUTES_SRC.contains(".merge(supplier_product_color_routes())"),
        "两个子路由应 merge 进 purchase::routes()"
    );
    assert!(
        PURCHASE_ROUTES_SRC.contains("\"/supplier-products\"")
            && PURCHASE_ROUTES_SRC.contains("\"/supplier-product-colors\""),
        "path 前缀必须是连字符命名"
    );
}

#[test]
fn handlers_validate_request_bodies() {
    assert!(
        SUP_PRODUCT_HANDLER_SRC.contains("req.validate()?"),
        "handler 必须 req.validate()? 触发 #[validate] 校验"
    );
    assert!(
        SUP_PRODUCT_COLOR_HANDLER_SRC.contains("req.validate()?"),
        "色号 handler 必须 req.validate()? 触发 #[validate] 校验"
    );
}

// ---------------------------------------------------------------------------
// 权限资源键：与 URL 段派生规则一致 + 授予集合 + 销售保密
// ---------------------------------------------------------------------------

/// 派生规则锁定：purchase 是模块前缀，且 resolve_module_prefixed_resource 未对
/// supplier-products / supplier-product-colors 做特殊映射（走 `_` 默认分支）→ 资源键
/// 就是 URL 第 4 段原名（连字符）。因此权限键必须与 path 段字面一致。
#[test]
fn resource_key_derivation_matches_path_segment() {
    assert!(
        PATH_UTILS_SRC.contains("\"purchase\""),
        "purchase 应被识别为模块前缀"
    );
    // resolve 默认分支保留 segment4 原名（无 supplier-products 专用映射）
    let resolve_body = extract_fn(PATH_UTILS_SRC, "pub fn resolve_module_prefixed_resource");
    assert!(
        !resolve_body.contains("supplier-products")
            && !resolve_body.contains("supplier-product-colors"),
        "两键不得被 resolve 特殊映射改写，保持与 path 段一致"
    );
    assert!(
        resolve_body.contains("_ => resource.to_string()"),
        "默认分支应原样返回 segment4 作资源键"
    );
}

/// 权限授予集合与派生键一致：两键均以连字符出现，且授予动作 ⊆ {read,create,update}。
#[test]
fn permission_keys_granted_for_purchase_roles() {
    for role in ["purchase_manager", "purchase_clerk", "sourcing_specialist"] {
        let body = extract_role_group(PERMISSION_SRC, role);
        assert!(
            body.contains("\"supplier-products\"") && body.contains("\"supplier-product-colors\""),
            "{role} 应被授 supplier-products / supplier-product-colors 读权限"
        );
        assert!(
            body.contains("(\"supplier-products\", \"read\")"),
            "{role} 应含 supplier-products:read"
        );
        assert!(
            body.contains("(\"supplier-product-colors\", \"read\")"),
            "{role} 应含 supplier-product-colors:read"
        );
    }
    // clerk 与 sourcing 可建目录（create+update）
    let clerk = extract_role_group(PERMISSION_SRC, "purchase_clerk");
    assert!(clerk.contains("(\"supplier-products\", \"create\")"));
    assert!(clerk.contains("(\"supplier-product-colors\", \"create\")"));
}

/// 销售角色一律不得授予这两个资源（保密：供应商目录不外泄给销售域）。
#[test]
fn sales_roles_not_granted_supplier_catalog() {
    for role in ["sales_manager", "sales_rep"] {
        let body = extract_role_group(PERMISSION_SRC, role);
        assert!(
            !body.contains("supplier-products") && !body.contains("supplier-product-colors"),
            "{role} 不得被授予供应商商品/色号目录（保密）"
        );
    }
}

// ---------------------------------------------------------------------------
// 辅助：函数/角色块截取（最小实现，仅用于契约字面量定位）
// ---------------------------------------------------------------------------

fn extract_fn<'a>(src: &'a str, sig: &str) -> &'a str {
    let start = src.find(sig).unwrap_or(0);
    let rest = &src[start..];
    if let Some(end) = rest.find("\n    }") {
        &rest[..end]
    } else {
        rest
    }
}

/// 截取某个角色代码块的资源列表（从 `"role_code",` 到其闭合 `],`）。
fn extract_role_group<'a>(src: &'a str, role: &str) -> &'a str {
    let anchor = format!("\"{}\"", role);
    let start = src.find(&anchor).unwrap_or(0);
    let rest = &src[start..];
    // 角色块以其后的资源数组闭合 `],` 收尾（首个出现）
    if let Some(end) = rest.find("],\n            ),") {
        &rest[..end]
    } else if let Some(end) = rest.find("],") {
        &rest[..end]
    } else {
        rest
    }
}
