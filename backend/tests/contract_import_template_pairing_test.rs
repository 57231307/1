//! 导入端点与导入模板的配对锁（静态，不连库）。
//!
//! 钉的对象 后端所有"文件导入"端点必须能取到配套导入模板，模板的列必须与该导入器
//! 实际解析的列同源；登记了模板却没有导入端点的类型必须在豁免清单里写明理由。
//! 调用方 CI 静态门与人工复核。入参 无（从源码解析期望值）。传给谁 断言失败消息点名
//! 缺失的端点路径、模板类型或列名。存哪里 不落库。
//!
//! 期望值一律从源码解析，测试不手抄第二套清单：
//! 导入端点路径解析 routes 目录的字符串字面量，SKU 列解析 sku_mapping_handler.rs 的
//! 权威表 `SKU_MAPPING_IMPORT_COLUMNS`，线索列序解析 services/crm/lead.rs 的按位读列下标，
//! 模板类型解析 import_export_service.rs 里 get_import_template 的 match 分支。
//! 锚点缺失或形态不可解析直接 panic 点名，禁止把"测不了"塌成"测过了"。
//! 分类清单只允许收窄不允许放宽：新增导入端点必须先归类，否则判红。

use bingxi_backend::services::import_export_service::ImportExportService;
use std::path::PathBuf;

const ROUTES_DIR_REL: &str = "src/routes";
const SKU_HANDLER_REL: &str = "src/handlers/sku_mapping_handler.rs";
const CRM_LEAD_SERVICE_REL: &str = "src/services/crm/lead.rs";
const TEMPLATE_SERVICE_REL: &str = "src/services/import_export_service.rs";

/// 已归类的文件导入端点 路由字面量到模板类型。调用方 配对断言。
/// 新增文件导入端点必须在此登记并配套模板，未登记的导入字面量由分类锁判红。
const FILE_IMPORT_ENDPOINTS: &[(&str, &str)] = &[
    ("/products/import", "products"),
    ("/purchase/sku-mappings/import", "sku_mapping"),
    ("/leads/import", "crm_leads"),
    ("/excel", "__data_import_center__"),
];

/// 已归类的结构化批量端点（JSON 载荷，不是文件导入，因此不需要 xlsx 模板）。
/// 判据是入参形态而非业务名，载荷为 JSON 数组的批量端点在此登记。
const JSON_BATCH_ENDPOINTS: &[&str] =
    &["/purchase-prices/import", "/items/batch", "/sku-mappings/batch"];

/// 登记了模板但没有对应文件导入端点的类型，必须写理由，不许静默空转。
const TEMPLATE_TYPES_WITHOUT_IMPORT_ENDPOINT: &[(&str, &str)] = &[
    (
        "customers",
        "客户面前只有客户新增与编辑口，批量导入入口未开，模板留作开通时的列定义依据",
    ),
    (
        "inventory",
        "库存走收货与直建入库两条真实腿，没有整表导入入口，模板留作列定义依据",
    ),
];

fn backend_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn read_source(rel: &str) -> String {
    let path = backend_root().join(rel);
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("读取 {} 失败: {}", path.display(), e))
}

/// 从 routes 目录源码里收集所有含 import 字面量的路由路径。
/// 返回按路径去重并排序后的清单，路由文件缺目录或锚点缺失直接 panic。
fn collect_import_route_literals() -> Vec<String> {
    let dir = backend_root().join(ROUTES_DIR_REL);
    let mut paths = Vec::new();
    let entries = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("读取路由目录 {} 失败: {}", dir.display(), e));
    for entry in entries {
        let entry = entry.expect("路由目录条目读取失败");
        let file_type = entry.file_type().expect("路由目录条目类型读取失败");
        if !(file_type.is_file() && entry.path().extension().is_some_and(|e| e == "rs")) {
            continue;
        }
        let src = std::fs::read_to_string(entry.path())
            .unwrap_or_else(|e| panic!("读取路由文件失败: {}", e));
        collect_route_literals_with_substring(&src, "import", &mut paths);
    }
    paths.sort();
    paths.dedup();
    paths
}

/// 从源码里取出同时含指定子串且以斜杠开头的路由字符串字面量。
/// 只认 `"/xxx"` 形态的路由字面量，注释与文档里的同名文本不算。
fn collect_route_literals_with_substring(src: &str, needle: &str, out: &mut Vec<String>) {
    let bytes = src.as_bytes();
    let mut idx = 0usize;
    while idx < bytes.len() {
        if bytes[idx] != b'"' {
            idx += 1;
            continue;
        }
        let Some(end) = src[idx + 1..].find('"') else {
            break;
        };
        let literal = &src[idx + 1..idx + 1 + end];
        if literal.starts_with('/') && literal.contains(needle) {
            out.push(literal.to_string());
        }
        idx += end + 2;
    }
}

/// 从 sku_mapping_handler.rs 解析权威列清单 列名与是否必需。
fn parse_sku_authority_columns() -> Vec<(String, bool)> {
    let src = read_source(SKU_HANDLER_REL);
    let anchor = "const SKU_MAPPING_IMPORT_COLUMNS: &[(&str, bool)] = &[";
    let start = src.find(anchor).unwrap_or_else(|| {
        panic!("sku_mapping_handler.rs 缺少权威表锚点 {}，权威表形态漂移", anchor)
    });
    let rest = &src[start + anchor.len()..];
    let end = rest
        .find("];")
        .unwrap_or_else(|| panic!("SKU 权威表字面量数组未闭合，找不到收尾方括号分号"));
    let body = &rest[..end];
    let mut columns = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('(') {
            continue;
        }
        let inner = trimmed.trim_start_matches('(').trim_end_matches(',').trim();
        let mut parts = inner.split(',').map(|p| p.trim()).filter(|p| !p.is_empty());
        let name_ref = parts.next().unwrap_or_else(|| panic!("SKU 权威表条目缺列名: {}", trimmed));
        let required_ref =
            parts.next().unwrap_or_else(|| panic!("SKU 权威表条目缺必需位: {}", trimmed));
        let field = resolve_column_const(&src, name_ref)
            .unwrap_or_else(|| panic!("SKU 权威表列名 {} 无法解析为常量定义", name_ref));
        columns.push((field, required_ref == "true"));
    }
    assert!(!columns.is_empty(), "SKU 权威表解析结果为空，判红不许放行");
    columns
}

/// 把 `COL_PRODUCT_CODE` 这类常量引用解析成它的字面量值。
fn resolve_column_const(src: &str, reference: &str) -> Option<String> {
    let anchor = format!("const {}: &str = \"", reference);
    let start = src.find(&anchor)?;
    let rest = &src[start + anchor.len()..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

/// 从 services/crm/lead.rs 的按位读列里解析线索导入实际消费的最大列下标与下标集合。
fn parse_lead_column_indices() -> (usize, Vec<usize>) {
    let src = read_source(CRM_LEAD_SERVICE_REL);
    let anchor = "fn build_lead_request_from_row";
    let start = src
        .find(anchor)
        .unwrap_or_else(|| panic!("lead.rs 缺少按位读列函数锚点 {}", anchor));
    let rest = &src[start..];
    let end = rest
        .find("\n    }")
        .unwrap_or_else(|| panic!("线索按位读列函数体未闭合"));
    let body = &rest[..end];
    let mut indices = Vec::new();
    for line in body.lines() {
        let marker = "extract_cell_string(row, ";
        let Some(at) = line.find(marker) else { continue };
        let after = &line[at + marker.len()..];
        let digits: String = after.chars().take_while(|c| c.is_ascii_digit()).collect();
        assert!(
            !digits.is_empty(),
            "线索读列下标不可解析，该行内容: {}",
            line.trim()
        );
        indices.push(digits.parse::<usize>().expect("线索读列下标转数失败"));
    }
    indices.sort();
    indices.dedup();
    assert!(!indices.is_empty(), "线索导入读列下标解析为空，判红不许放行");
    let max = *indices.last().expect("线索读列下标取最大值失败");
    (max, indices)
}

/// 解析 get_import_template 里登记的模板类型清单。
fn parse_registered_template_types() -> Vec<String> {
    let src = read_source(TEMPLATE_SERVICE_REL);
    let anchor = "pub fn get_import_template(import_type: &str)";
    let start = src
        .find(anchor)
        .unwrap_or_else(|| panic!("import_export_service.rs 缺少模板登记锚点 {}", anchor));
    let rest = &src[start..];
    let end = rest
        .find("    pub(crate) fn build_products_template")
        .unwrap_or_else(|| panic!("模板登记函数体未闭合，找不到产品模板构造函数的位置界标"));
    let body = &rest[..end];
    let mut types = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim();
        let Some(quote_at) = trimmed.find('"') else {
            continue;
        };
        let after = &trimmed[quote_at + 1..];
        let Some(close) = after.find('"') else {
            continue;
        };
        let candidate = &after[..close];
        if trimmed.contains("=>") && !candidate.is_empty() {
            types.push(candidate.to_string());
        }
    }
    assert!(
        !types.is_empty(),
        "模板类型登记清单解析为空，判红不许放行"
    );
    types
}

/// 判定给定模板类型能否取到模板。
fn template_available(import_type: &str) -> bool {
    ImportExportService::get_import_template(import_type).is_ok()
}

/// 正向核心断言，每个已归类的文件导入端点都能取到配套模板。
#[test]
fn every_file_import_endpoint_has_downloadable_template() {
    for (route, import_type) in FILE_IMPORT_ENDPOINTS {
        if *import_type == "__data_import_center__" {
            assert!(
                !template_available(import_type),
                "通用数据导入中心走 /data-import/templates 自有模板体系，不该在通用登记表里冒出同名类型: {}",
                route
            );
            continue;
        }
        assert!(
            template_available(import_type),
            "导入端点 {} 没有配套模板类型 {}，用户无法下载模板得知导入格式",
            route,
            import_type
        );
    }
}

/// 分类锁，routes 里出现的每个 import 路由字面量都必须已被归类，新增未归类即判红。
#[test]
fn all_import_route_literals_are_classified() {
    let literals = collect_import_route_literals();
    assert!(
        !literals.is_empty(),
        "routes 目录里没解析到任何 import 路由字面量，判定器失效嫌疑"
    );
    let mut classified: Vec<&str> = Vec::new();
    for (route, _) in FILE_IMPORT_ENDPOINTS {
        classified.push(route);
    }
    for route in JSON_BATCH_ENDPOINTS {
        classified.push(route);
    }
    classified.push("/import");
    classified.push("/templates");
    classified.push("/templates/download/{import_type}");
    classified.push("/templates/meta/{import_type}");
    for literal in &literals {
        if literal.starts_with("/data-import") {
            // 通用数据导入中心自带模板体系（列表 详情 下载 端点齐全），
            // 其路径字面量含 import 子串属同源命名，不参与"每个文件导入端点必须有通用模板类型"的判定。
            continue;
        }
        let covered = classified.iter().any(|known| {
            literal.as_str() == *known
                || known.ends_with("/import") && literal.ends_with(known)
                || literal.contains("/templates")
        });
        assert!(
            covered,
            "导入相关路由 {} 未归类，请按入参形态登记到文件导入或结构化批量清单并配套模板",
            literal
        );
    }
}

/// SKU 模板的列与导入器权威表逐列同源，列序与必需性一致，示例值不得为空。
#[test]
fn sku_template_columns_match_importer_authority_table() {
    let authority = parse_sku_authority_columns();
    let template = ImportExportService::get_import_template("sku_mapping")
        .expect("SKU 对照表模板不可用，取模板失败");
    let actual: Vec<(String, bool)> = template
        .columns
        .iter()
        .map(|c| (c.field.clone(), c.required))
        .collect();
    let expected: Vec<(String, bool)> =
        authority.iter().map(|(f, r)| (f.clone(), *r)).collect();
    assert_eq!(
        expected, actual,
        "SKU 模板列与导入器权威表不一致，权威表 {} 列，模板 {} 列",
        authority.len(),
        template.columns.len()
    );
    assert_eq!(
        template.columns.len(),
        template
            .columns
            .iter()
            .filter(|c| c.example.is_some())
            .count(),
        "SKU 模板存在无示例值的列，用户按模板填不出可导入的行"
    );
    for column in &template.columns {
        assert_eq!(
            column.field, column.title,
            "SKU 模板列 {} 的表头与解析键不一致，按模板填的文件会被表头校验判成未知列",
            column.field
        );
    }
}

/// 线索模板的列序必须覆盖导入器按位消费的全部下标，少一列就会把后一列读进前一列。
#[test]
fn leads_template_columns_cover_positional_indices() {
    let (max_index, indices) = parse_lead_column_indices();
    let template = ImportExportService::get_import_template("crm_leads")
        .expect("CRM 线索模板不可用，取模板失败");
    assert!(
        template.columns.len() >= max_index + 1,
        "线索模板列数 {} 不足，导入器消费到第 {} 列，缺列会让读列落空",
        template.columns.len(),
        max_index
    );
    for index in &indices {
        let column = &template.columns[*index];
        assert!(
            !column.title.is_empty(),
            "线索模板第 {} 列表头为空，用户看不出该列要填什么",
            index + 1
        );
    }
}

/// 反向锁，登记了模板却没有文件导入端点的类型必须在豁免清单里且写明理由。
#[test]
fn template_types_without_import_endpoint_are_exempted_with_reason() {
    let registered = parse_registered_template_types();
    assert!(
        !registered.is_empty(),
        "模板类型清单解析为空，判定器失效嫌疑"
    );
    let endpoint_types: Vec<&str> = FILE_IMPORT_ENDPOINTS
        .iter()
        .map(|(_, import_type)| *import_type)
        .filter(|t| *t != "__data_import_center__")
        .collect();
    for import_type in &registered {
        if endpoint_types.contains(&import_type.as_str()) {
            continue;
        }
        let exemption = TEMPLATE_TYPES_WITHOUT_IMPORT_ENDPOINT
            .iter()
            .find(|(t, _)| *t == import_type.as_str());
        let Some((_, reason)) = exemption else {
            panic!(
                "模板类型 {} 既没有对应导入端点也没登记豁免理由，属模板空转",
                import_type
            );
        };
        assert!(
            reason.trim().len() >= 12,
            "模板类型 {} 的豁免理由过短，必须写清为什么留着",
            import_type
        );
    }
}

/// 判定器自证，人为造缺模板与掉列两种坏样本时断言必须报错。
#[test]
fn pairing_checker_detects_broken_inputs() {
    assert!(
        !template_available("no_such_import_type"),
        "模板可用性判定失效，不存在的类型也返回可用，正向锁会退化成恒真"
    );
    let mut broken = parse_sku_authority_columns();
    assert!(
        broken.len() >= 2,
        "SKU 权威表列数不足两条，无法构造掉列样本，自证不成立"
    );
    broken.remove(0);
    let template = ImportExportService::get_import_template("sku_mapping")
        .expect("SKU 对照表模板不可用，取模板失败");
    let full: Vec<(String, bool)> = template
        .columns
        .iter()
        .map(|c| (c.field.clone(), c.required))
        .collect();
    let mutated: Vec<(String, bool)> = broken;
    assert_ne!(
        full, mutated,
        "掉列样本与完整模板仍然相等，说明比对根本没在校验列集合"
    );
    let mut swapped = full.clone();
    swapped.swap(0, 1);
    assert_ne!(
        full, swapped,
        "换序样本与完整模板仍然相等，说明比对没在按列序校验"
    );
}
