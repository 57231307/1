//! 对照表批量导入权限键 `sku-mappings:import` 的三通道一致锁。
//!
//! 锁定的事实链（缺一即本文件判红）：
//! 1. 运行时判权键由 URL 派生：`POST /api/v1/erp/purchase/sku-mappings/import` 的资源段是
//!    `sku-mappings`（`purchase` 是模块前缀，需消歧），动作段是 `import`（`import` 在
//!    `PATH_ACTION_KEYWORDS` 在册，末段动作词优先于 HTTP 方法 ⇒ 键不是 `sku-mappings:create`）。
//! 2. 注册表含 `sku-mappings`：资源未登记时门层在管理员旁路之前就先判「未知的资源路径」，
//!    于是该端点对任何角色都永不可达。
//! 3. init 角色矩阵与回收迁移 `m0096_grant_sku_mapping_import_purchase_roles` 的受授集合
//!    按「矩阵码 + 在册别名码」归一后必须**集合相等**：只补一侧就会造成新装库与存量库
//!    权限形态分叉。
//! 4. 前端以命名常量 `SKU_MAPPING_IMPORT` 登记同一枚键，且对照表页的导入按钮用它门控，
//!    不得再借用 `SKU_MAPPING_CREATE`（前端借 create、后端要 import 是已发生过的 403 根因）。
//!
//! 本文件同时自证检测力：把末段换成未登记的伪动作词必须派生不出动作，
//! 把资源段换成未登记组合必须消歧不回 `sku-mappings` —— 否则上面的相等断言恒真。

use bingxi_backend::middleware::permission::{extract_action_from_path, extract_resource_info};
use bingxi_backend::services::init_service::PERMISSION_RESOURCES;
use bingxi_backend::utils::path_utils::resolve_module_prefixed_resource;

const RESOURCE: &str = "sku-mappings";
const ACTION: &str = "import";
const IMPORT_PATH: &str = "/api/v1/erp/purchase/sku-mappings/import";

/// init 角色矩阵源（含每个角色的 (资源, 动作) 元组清单）
fn matrix_src() -> String {
    let raw = include_str!("../src/services/init_service_ops/permission.rs");
    raw.replace("\r\n", "\n")
}

/// 去掉注释行，避免注释里出现的元组被当成授予
fn code_only(src: &str) -> String {
    src.lines()
        .filter(|l| !l.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 矩阵里持有 (RESOURCE, action) 的角色码。角色块的识别按
/// 「`"role_code",` 紧随 `&[`」这一唯一书写形态扫描，元组行归属最近的角色块。
fn matrix_roles_with_action(matrix_code: &str, action: &str) -> Vec<String> {
    let needle = format!("(\"{RESOURCE}\", \"{action}\")");
    let mut holders: Vec<String> = Vec::new();
    let mut current: Option<String> = None;
    let mut pending_role: Option<String> = None;
    for line in matrix_code.lines() {
        let t = line.trim();
        if let Some(code) = role_block_opener(t) {
            pending_role = Some(code);
            continue;
        }
        if t.starts_with("&[") {
            if let Some(code) = pending_role.take() {
                current = Some(code);
            }
            continue;
        }
        if t == "]," || t == "]" {
            current = None;
            continue;
        }
        if t.contains(&needle) {
            if let Some(code) = &current {
                if !holders.iter().any(|h| h == code) {
                    holders.push(code.clone());
                }
            }
        }
    }
    holders
}

/// 识别 `"role_code",` 形式的角色块起始行（角色码只由小写字母与下划线组成）
fn role_block_opener(trimmed: &str) -> Option<String> {
    let inner = trimmed.strip_suffix(',')?;
    let (head, tail) = inner.split_once('"')?;
    if !head.trim().is_empty() {
        return None;
    }
    let code = tail.strip_suffix('"')?;
    if code.is_empty() || !code.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
        return None;
    }
    Some(code.to_string())
}

/// 迁移文件里的受授角色码常量（`const NAME: &[&str] = [...];`）
fn migration_role_codes(mig_code: &str, const_name: &str) -> Vec<String> {
    let anchor = format!("const {const_name}");
    let start = mig_code
        .find(&anchor)
        .unwrap_or_else(|| panic!("迁移缺少常量 {const_name} —— 受授集合必须写在迁移里才可回滚"));
    let tail = &mig_code[start..];
    // 数组字面量的 `[` 必须从赋值号 `=` 之后取：类型标注 `&[&str]` 里也有一对 []，
    // 直接 find('[') 会命中类型段而把 `[&str]` 误当成角色码。
    let eq = tail.find('=').expect("受授常量声明缺少赋值号");
    let open = tail[eq..]
        .find('[')
        .map(|i| eq + i)
        .expect("受授常量应为数组字面量");
    let close = tail[open..]
        .find(']')
        .map(|i| open + i)
        .expect("受授常量数组未闭合");
    tail[open..=close]
        .split('"')
        .step_by(2)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty() && s.chars().any(|c| c.is_ascii_alphabetic()))
        .map(|s| s.to_string())
        .collect()
}

/// 矩阵码 ↔ 别名码归一：采购经理与采购员各有两枚在册码（矩阵名与部署/e2e 名）
fn fold_purchase_aliases(code: &str) -> String {
    match code {
        "purchasing_manager" => "purchase_manager".to_string(),
        "purchaser" => "purchase_clerk".to_string(),
        other => other.to_string(),
    }
}

fn distinct_canonical(codes: &[String]) -> Vec<String> {
    let mut out: Vec<String> = codes.iter().map(|c| fold_purchase_aliases(c)).collect();
    out.sort();
    out.dedup();
    out
}

// —— 一、运行时判权键必须逐字派生成 sku-mappings:import ——
#[test]
fn import_key_derives_from_real_path_parsing() {
    let (resource, id) = extract_resource_info(IMPORT_PATH);
    assert_eq!(
        resource, RESOURCE,
        "{IMPORT_PATH} 的资源段须消歧为 {RESOURCE}，实得 {resource}"
    );
    assert_eq!(
        id, None,
        "import 段不是资源 ID；若被当成 ID 说明路径解析把动作词吃进了 ID 位"
    );
    assert_eq!(
        extract_action_from_path(IMPORT_PATH).as_deref(),
        Some(ACTION),
        "末段 import 必须派生成动作键 {ACTION}，否则门层按 HTTP 方法派生成 create，本键永不生效"
    );
}

// —— 二、资源必须已登记，否则该端点对任何角色永不可达 ——
#[test]
fn resource_is_registered_in_permission_registry() {
    assert!(
        PERMISSION_RESOURCES.contains(&RESOURCE),
        "{RESOURCE} 未登记进 PERMISSION_RESOURCES，门层会先判「未知的资源路径」拦掉一切角色"
    );
}

// —— 三、通道①（init 矩阵）与通道②（回收迁移）受授集合归一后必须相等 ——
#[test]
fn grantee_set_matches_across_channels() {
    let matrix = matrix_roles_with_action(&code_only(&matrix_src()), ACTION);
    assert!(
        !matrix.is_empty(),
        "通道①解析出空集说明角色块/元组书写形态变了，本锁的相等断言将恒真 —— 检测力自证见本文件末条"
    );
    let mig_code = code_only(include_str!(
        "../migration/src/domain/business/m0096_grant_sku_mapping_import_purchase_roles.rs"
    ));
    let migration = migration_role_codes(&mig_code, "ROLE_CODES");

    let from_matrix = distinct_canonical(&matrix);
    let from_migration = distinct_canonical(&migration);
    assert_eq!(
        from_matrix, from_migration,
        "三通道分叉：init 矩阵授予 {from_matrix:?}，迁移授予 {from_migration:?}（按别名归一后）——\
         新装库与存量库会得到不同的权限形态"
    );
    assert_eq!(
        from_matrix,
        vec!["purchase_clerk".to_string(), "purchase_manager".to_string()],
        "导入键的受授集合应当恰是采购经理与采购员两岗（对照表属采购保密域，销售侧一律不授）"
    );
}

// —— 四、前端必须用命名常量按同一枚键门控导入按钮，不得借用 create ——
#[test]
fn frontend_gates_import_button_with_the_same_key() {
    let consts = include_str!("../../frontend/src/constants/permissions.ts").replace('\r', "");
    assert!(
        consts.contains(&format!("'{RESOURCE}:{ACTION}'")),
        "前端常量表必须登记 {RESOURCE}:{ACTION} 这枚键"
    );
    let view = include_str!("../../frontend/src/views/sku-mapping/index.vue").replace('\r', "");
    let opener_at = view
        .find("openImportDialog()")
        .expect("对照表页应保留导入入口，删除入口须与后端键同批处理");
    let gating = &view[..opener_at];
    let gate_start = gating
        .rfind("v-permission")
        .expect("导入入口必须由权限指令门控");
    let gate_window = &gating[gate_start..];
    assert!(
        gate_window.contains("SKU_MAPPING_IMPORT"),
        "导入按钮必须以命名常量 SKU_MAPPING_IMPORT 门控；若仍借用 SKU_MAPPING_CREATE，\
         后端派生键是 {RESOURCE}:{ACTION}，按钮可见性与实际判权就会两套事实"
    );
    assert!(
        !gate_window.contains("SKU_MAPPING_CREATE"),
        "导入按钮的门控里不应再出现 create 键"
    );
}

// —— 五、检测力自证：本锁不是恒真 ——
#[test]
fn lock_can_distinguish_absent_keys_and_words() {
    assert_eq!(
        extract_action_from_path("/api/v1/erp/purchase/sku-mappings/ghost-action").as_deref(),
        None,
        "未登记的动作词不得被当成动作键"
    );
    assert_ne!(
        resolve_module_prefixed_resource("purchase", "sku-mappings-ghost"),
        RESOURCE,
        "未登记的资源段不得被消歧成 {RESOURCE}"
    );
    let synthetic = format!("\"some_role\",\n&[\n(\"{RESOURCE}\", \"read\"),\n],");
    assert!(
        matrix_roles_with_action(&synthetic, ACTION).is_empty(),
        "解析器须对不含 import 键的输入返回空集 —— 这是通道①②相等断言能检出漂移的前提"
    );
    assert_eq!(
        matrix_roles_with_action(&code_only(&matrix_src()), ACTION).len(),
        2,
        "真实矩阵里 {RESOURCE}:{ACTION} 应恰被两岗持有；解析器对真源应给出非空且计数为二"
    );
}
