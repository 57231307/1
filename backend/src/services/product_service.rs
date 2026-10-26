//! 产品服务（面料行业版）— facade
//!
//! 本文件为 facade：保留 DTO struct 定义、`ProductService` struct 定义、
//! `new` 构造函数。impl 块已按职责拆分到 [`crate::services::product_ops`] 子模块：
//! - `sync`：ES 同步辅助（build_product_doc / sync_product_to_es）
//! - `crud`：产品 CRUD（list/get/create/update/delete + generate_product_code）
//! - `color`：色号管理（list/create/batch_create/update/delete product_color）
//! - `import_export`：CSV 导入导出（export/template/import + 字段校验/解析 helper）
//!
//! 批次 125 v8 复审 P1 修复：注入 search_syncer 实现 PG→ES 写入同步。
//! - create/update/delete 事务提交后调用 sync_product / delete_product 同步到 ES
//! - ES 同步失败仅记录 tracing::warn!（最终一致性），不回滚 PG 事务
//!
//! 批次 D10 拆分：`db` / `search_syncer` 字段改为 `pub(crate)` 供 ops 子模块访问；
//! 跨 ops 子模块调用的 `sync_product_to_es` 改为 `pub(crate)` 供 `crud` 子模块调用。
//! `ValidatedRowFields` struct 迁移到 `product_ops::import_export`（私有，仅 CSV 流程内部用）。

use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::models::status::color_fastness;
use crate::models::status::product_safety_category;
use crate::models::status::product_type;
use crate::search::{SearchClient, SearchSyncer};
use crate::utils::error::AppError;

/// 创建产品色号输入结构体
#[derive(Debug, Clone)]
pub struct CreateProductColorInput {
    pub color_no: String,
    pub color_name: String,
    pub pantone_code: Option<String>,
    pub color_type: String,
    pub dye_formula: Option<String>,
    pub extra_cost: f64,
    /// 色牢度等级（GB 18401 九档，可空即未考核或未录入）。取值域由 color_fastness 词表判定。
    pub color_fastness_grade: Option<String>,
}

/// 更新产品色号参数对象（批次 330 v10 复审 P3 修复：引入参数对象消除 too_many_arguments 警告）
#[derive(Debug)]
pub struct UpdateProductColorParams {
    /// 色号 ID
    pub id: i32,
    /// 色号名称
    pub color_name: Option<String>,
    /// 潘通色号
    pub pantone_code: Option<String>,
    /// 色号类型
    pub color_type: Option<String>,
    /// 染色配方
    pub dye_formula: Option<String>,
    /// 额外成本
    pub extra_cost: Option<f64>,
    /// 是否启用
    pub is_active: Option<bool>,
    /// 色牢度等级（GB 18401 九档，单层条件更新，键缺席即不改，空串按清空语义处理）
    pub color_fastness_grade: Option<String>,
    /// 操作人 ID（审计日志）
    pub user_id: i32,
}

/// 产品服务（面料行业版）
/// 批次 125 v8 复审 P1 修复：注入 search_syncer 实现 PG→ES 写入同步。；create/update/delete 事务提交后调用 sync_product / delete_product 同步到 ES；ES 同步失败仅记录 tracing::warn!（最终一致性），不回滚 PG 事务；批次 D10 拆分：impl 块已拆到 `product_ops` 子模块（sync/crud/color/import_export）。；`db` / `search_syncer` 字段为 `pub(crate)` 供 ops 子模块访问。
pub struct ProductService {
    pub(crate) db: Arc<DatabaseConnection>,
    /// ES 同步器（PG→ES 写入同步），批次 125 接入
    pub(crate) search_syncer: Arc<SearchSyncer>,
}

impl ProductService {
    pub fn new(db: Arc<DatabaseConnection>, search_client: Arc<dyn SearchClient>) -> Self {
        Self {
            db,
            search_syncer: Arc::new(SearchSyncer::new(search_client)),
        }
    }
}

/// 创建产品参数对象
/// 批次 339 v10 复审 P3 修复：引入参数对象消除 create_product 的 too_many_arguments 警告。；聚合创建产品所需的全部字段（含面料行业字段），避免函数签名携带 19 个参数。
#[derive(Debug, Clone)]
pub struct CreateProductArgs {
    /// 产品名称
    pub name: String,
    /// 产品编码
    pub code: String,
    /// 产品条码（EAN/UPC 等，未编码为空）
    pub barcode: Option<String>,
    /// 分类 ID
    pub category_id: Option<i32>,
    /// 规格
    pub specification: Option<String>,
    /// 单位
    pub unit: String,
    /// 标准价（DECIMAL 列，全链 Decimal，不做 f64 往返）
    pub standard_price: Option<Decimal>,
    /// 成本价（DECIMAL 列，同上）
    pub cost_price: Option<Decimal>,
    /// 描述
    pub description: Option<String>,
    /// 状态
    pub status: String,
    /// 产品类型（面料行业字段）。新建必须显式提交，缺省即被 create_product 入口拒绝，
    /// 不再在 handler 侧缺省成成品布，避免「按品类分级必填」被缺省绕过。
    pub product_type: Option<String>,
    /// 面料成分
    pub fabric_composition: Option<String>,
    /// 纱支
    pub yarn_count: Option<String>,
    /// 密度
    pub density: Option<String>,
    /// 幅宽（cm，DECIMAL(10,2) 列，入参到落库全程 Decimal）
    pub width: Option<Decimal>,
    /// 克重（g/m²，DECIMAL(10,2) 列，入参到落库全程 Decimal）
    pub gram_weight: Option<Decimal>,
    /// 组织
    pub structure: Option<String>,
    /// 后整理
    pub finish: Option<String>,
    /// 最小起订量（DECIMAL 列，同上）
    pub min_order_quantity: Option<Decimal>,
    /// 交期（天）
    pub lead_time: Option<i32>,
    /// V15 P1 合规字段：面料执行标准号（GB/T 406-2018 等）
    pub execution_standard: Option<String>,
    /// V15 P1 合规字段：生产厂名（《产品质量法》第 27 条）
    pub factory_name: Option<String>,
    /// V15 P1 合规字段：生产厂址
    pub factory_address: Option<String>,
    /// V15 P1 合规字段：产品等级（优等品/一等品/合格品）
    pub product_grade: Option<String>,
    /// 带针幅宽（cm，含针边，DECIMAL(10,2) 列）。与 width（有效幅宽，净幅）两列独立可空，
    /// 本列不参与克重与米数换算。
    pub width_needle_included_cm: Option<Decimal>,
    /// 克重公差（±g/m²，DECIMAL(10,2) 列，非负）。目标值在 gram_weight，本列记允差幅度。
    pub weight_tolerance: Option<Decimal>,
    /// 氨纶含量（百分比，DECIMAL(5,2) 列，0 到 100）。弹性要求的行业口径。
    pub spandex_percent: Option<Decimal>,
    /// 用途（自由文本，如 服装面料/家纺/产业用）
    pub usage: Option<String>,
    /// 安全类别（GB 18401-2010 的 A/B/C，可空即未定级）。取值以 product_safety_category 词表为唯一源，
    /// A 类另须满足 GB 31701-2015，该条件判据落应用层不由本列表达。
    pub safety_category: Option<String>,
    /// 每匹米数（匹↔米换算元数据，可选数值，None 表示未录入）
    pub meters_per_piece: Option<Decimal>,
    /// 每卷米数（卷↔米换算元数据，可选数值，None 表示未录入）
    pub meters_per_roll: Option<Decimal>,
}

/// 更新产品参数对象
/// 批次 339 v10 复审 P3 修复：引入参数对象消除 update_product 的 too_many_arguments 警告。；聚合更新产品所需的全部字段（含面料行业字段），避免函数签名携带 19 个参数。
#[derive(Debug, Clone)]
pub struct UpdateProductArgs {
    /// 产品 ID
    pub id: i32,
    /// 产品名称
    pub name: Option<String>,
    /// 产品条码（未编码为空）
    pub barcode: Option<String>,
    /// 规格
    pub specification: Option<String>,
    /// 单位
    pub unit: Option<String>,
    /// 标准价（DECIMAL 列，三态部分更新）。外层 None 即键缺席、保持原值，
    /// Some(None) 即显式 null、清空为 NULL，Some(Some(v)) 即覆盖为该值。
    pub standard_price: Option<Option<Decimal>>,
    /// 成本价（DECIMAL 列，三态部分更新），形态与语义同上。
    pub cost_price: Option<Option<Decimal>>,
    /// 描述
    pub description: Option<String>,
    /// 状态
    pub status: Option<String>,
    /// 产品类型
    pub product_type: Option<String>,
    /// 面料成分
    pub fabric_composition: Option<String>,
    /// 纱支
    pub yarn_count: Option<String>,
    /// 密度
    pub density: Option<String>,
    /// 幅宽（cm，DECIMAL(10,2) 列，三态部分更新），形态与语义同标准价。
    pub width: Option<Option<Decimal>>,
    /// 克重（g/m²，DECIMAL(10,2) 列，三态部分更新），形态与语义同上。
    pub gram_weight: Option<Option<Decimal>>,
    /// 组织
    pub structure: Option<String>,
    /// 后整理
    pub finish: Option<String>,
    /// 最小起订量（DECIMAL 列，三态部分更新），形态与语义同上。
    pub min_order_quantity: Option<Option<Decimal>>,
    /// 交期（天，三态部分更新），形态与语义同最小起订量。
    pub lead_time: Option<Option<i32>>,
    /// V15 P1 合规字段：面料执行标准号（GB/T 406-2018 等）
    pub execution_standard: Option<String>,
    /// V15 P1 合规字段：生产厂名
    pub factory_name: Option<String>,
    /// V15 P1 合规字段：生产厂址
    pub factory_address: Option<String>,
    /// V15 P1 合规字段：产品等级（优等品/一等品/合格品）
    pub product_grade: Option<String>,
    /// 带针幅宽（cm，含针边，DECIMAL(10,2) 列，三态部分更新），形态与语义同幅宽。
    pub width_needle_included_cm: Option<Option<Decimal>>,
    /// 克重公差（±g/m²，DECIMAL(10,2) 列，三态部分更新），形态与语义同克重。
    pub weight_tolerance: Option<Option<Decimal>>,
    /// 氨纶含量（百分比，DECIMAL(5,2) 列，三态部分更新），形态与语义同克重。
    pub spandex_percent: Option<Option<Decimal>>,
    /// 用途（自由文本，单层条件更新，键缺席即不更新该列），语义同组织结构与后整理。
    pub usage: Option<String>,
    /// 安全类别（GB 18401-2010 的 A/B/C，单层条件更新，键缺席即不更新该列），语义同用途。
    pub safety_category: Option<String>,
    /// 每匹米数（匹↔米换算元数据，三态部分更新）。外层 None 即键缺席、本列保持原值，
    /// Some(None) 即显式 null、清空为 NULL，Some(Some(v)) 即覆盖为该值。
    pub meters_per_piece: Option<Option<Decimal>>,
    /// 每卷米数（卷↔米换算元数据，三态部分更新），形态与语义同上。
    pub meters_per_roll: Option<Option<Decimal>>,
    /// 操作人 ID
    pub user_id: i32,
}

/// 匹与卷的每单位米数同源补齐。
///
/// 读的是请求携带的每匹米数与每卷米数两值的存在性。只带其中一个时把另一列补成同值，
/// 两个都带时各自原样返回（含不等也不取其一、不互相覆写），两个都不带时不补齐、
/// 不引入默认值掩盖缺键。
/// 写给产品单条创建与单条更新两条链路（handler 层），以及批量创建与批量更新
/// （batch_service 层），结果交给 CreateProductArgs 与 UpdateProductArgs 的同名两字段，
/// 再由 product_ops 的 crud 与 batch_service 的 ActiveModel 落库。
/// 存的是 products 表的每匹米数与每卷米数两列，DECIMAL 可空，NULL 表示未录入。
/// 为什么泛型。创建侧 T 为 Decimal、单层 Option 即两态（带值或缺值）。更新侧 T 为
/// Option<Decimal>、双层 Option 即三态（覆盖、清空、不动）。补齐只判断外层存在性，
/// 单层 Some(x) 与双层 Some(显式声明值) 都被视为本次带了这个键，复制到另一键。
/// 为什么放服务层而不是只留在 handler。批量链路在 batch_service 也需要同一份补齐语义，
/// 服务与处理器共用一个真源，避免两处各写一套导致语义漂移。
/// 边界。更新侧只声明清空每匹米数时，每卷米数按同源补成同样的清空声明。
pub fn pair_piece_roll_meters<T: Clone>(
    meters_per_piece: Option<T>,
    meters_per_roll: Option<T>,
) -> (Option<T>, Option<T>) {
    match (meters_per_piece, meters_per_roll) {
        (Some(piece), None) => (Some(piece.clone()), Some(piece)),
        (None, Some(roll)) => (Some(roll.clone()), Some(roll)),
        (piece, roll) => (piece, roll),
    }
}

/// 面料类主数据必录校验（新建建品唯一判据源）。
///
/// 读的是本次建品提交的产品类型、幅宽(cm)、克重(g/m²)、面料成分四值。写给谁调用：
/// 产品单条 HTTP 建品（product_handler::create_product）与 CSV 导入建品
/// （product_ops::import_export::process_csv_import_row）共用的 product_ops::crud::create_product 入口，
/// 以及批量建品 batch_service::batch_create_products 的逐行前置检查；三处共用这一个函数，判据只有一份。
/// 存什么：不落库，只做拒绝，命中即返回校验错误。
///
/// 规则。产品类型必须显式提交（新建不再缺省成成品布，否则「按品类分级必填」会被缺省绕过）；
/// 类型落在面料集合（坯布、成品布，models/status/general.rs::product_type::FABRIC_TYPES）内时，
/// 幅宽与克重都必须有值且大于 0、面料成分必须非空（空串与纯空白算缺失）；
/// 缺哪一列就点名哪一列，三项各出一条消息，不合并成一句模糊提示。辅料等非面料类不要求这三列，直接通过。
///
/// 为什么放服务层唯一入口而不是只写在前端 rules 或某个 handler。
/// 前端 rules 被 CSV 导入、批量建品、直连 API 全部绕过；只在单条 handler 写又漏掉批量与导入两路。
/// 收敛到 create_product 这一入口，两条路都在落库前撞到同一判据；批量链路不经 create_product，
/// 故在 batch_service 另调本函数，判据仍是同一份，不复制第二套。
/// 数值取 Decimal 是为让单条 HTTP、CSV 导入与批量入参三条路径在同一个形态上比较
/// （幅宽与克重列为 DECIMAL(10,2)，入参到落库全程 Decimal，无 f64 往返），
/// 不做取整、不做哨兵兜底，缺值与非正一律拒绝。
///
/// 为什么用 validation_displayable 而不是 validation。取值域同为本仓输入校验族
/// （HTTP 400 / code=VALIDATION_ERROR），但 validation 出参 message 恒为脱敏固定文案，
/// 会把「缺哪一列」吞掉、不满足 fail-visible；本函数文案只描述用户自己提交字段的必填规则
/// （幅宽/克重/面料成分/产品类型），不涉表名、SQL、他人数据，符合 validation_displayable 的安全边界。
pub fn validate_product_create_fields(
    product_type: Option<&str>,
    width: Option<Decimal>,
    gram_weight: Option<Decimal>,
    fabric_composition: Option<&str>,
) -> Result<(), AppError> {
    let trimmed_type = product_type.map(str::trim).filter(|s| !s.is_empty());
    let resolved_type = trimmed_type.ok_or_else(|| {
        AppError::validation_displayable("新建产品必须显式提交产品类型，不允许缺省")
    })?;

    if !product_type::FABRIC_TYPES.contains(&resolved_type) {
        return Ok(());
    }

    let positive = |v: Option<Decimal>| v.is_some_and(|x| x > Decimal::ZERO);
    if !positive(width) {
        return Err(AppError::validation_displayable(
            "面料类产品必须填写幅宽（单位 cm），且幅宽必须大于 0",
        ));
    }
    if !positive(gram_weight) {
        return Err(AppError::validation_displayable(
            "面料类产品必须填写克重（单位 g/m²），且克重必须大于 0",
        ));
    }
    let composition_present = fabric_composition
        .map(str::trim)
        .is_some_and(|s| !s.is_empty());
    if !composition_present {
        return Err(AppError::validation_displayable(
            "面料类产品必须填写面料成分，不能为空",
        ));
    }
    Ok(())
}

/// 产品主档取值域校验（提交值形态判据，创建与更新共用）。
///
/// 读的是本次提交中「带了值」的类型与数值主档列。写给谁调用：
/// crud::create_product 与 crud::update_product（单条 HTTP 与 CSV 导入共用这两个入口），
/// 以及批量建品 batch_service::batch_create_products 的逐行前置检查；判据一份，三路同门。
/// 存什么：不落库，只做拒绝。
///
/// 规则。产品类型出现时必须在词表 ALL_TYPES（坯布/成品布/辅料）内，词表外值点名拒绝，
/// 不落库后再靠数据修正；幅宽出现必须大于 0（列语义为有效幅宽、单位 cm，0 与负数是非法录入）；
/// 克重出现必须大于 0（g/m²）；最小起订量与交货期出现不得为负；价格两列出现在不得为负。
/// 键缺席与显式空（None）不在本判据范围——必填性由 validate_product_create_fields 按品类分级判定，
/// 本函数只管「提交了就必须是合法形态」。
/// 文案走 validation_displayable：只描述用户自己提交字段的取值规则，不涉表名与 SQL，满足 fail-visible。
pub fn validate_product_numeric_domains(
    product_type: Option<&str>,
    width: Option<Decimal>,
    gram_weight: Option<Decimal>,
    min_order_quantity: Option<Decimal>,
    lead_time: Option<i32>,
    standard_price: Option<Decimal>,
    cost_price: Option<Decimal>,
) -> Result<(), AppError> {
    if let Some(t) = product_type.map(str::trim).filter(|s| !s.is_empty())
        && !product_type::ALL_TYPES.contains(&t)
    {
        return Err(AppError::validation_displayable(format!(
            "产品类型「{t}」不在词表内，允许值：坯布、成品布、辅料"
        )));
    }
    if width.is_some_and(|w| w <= Decimal::ZERO) {
        return Err(AppError::validation_displayable(
            "幅宽（有效幅宽，单位 cm）必须大于 0",
        ));
    }
    if gram_weight.is_some_and(|g| g <= Decimal::ZERO) {
        return Err(AppError::validation_displayable(
            "克重（单位 g/m²）必须大于 0",
        ));
    }
    if min_order_quantity.is_some_and(|q| q < Decimal::ZERO) {
        return Err(AppError::validation_displayable("最小起订量不能为负数"));
    }
    if lead_time.is_some_and(|l| l < 0) {
        return Err(AppError::validation_displayable("交货期（天）不能为负数"));
    }
    if standard_price.is_some_and(|p| p < Decimal::ZERO) {
        return Err(AppError::validation_displayable("标准价格不能为负数"));
    }
    if cost_price.is_some_and(|p| p < Decimal::ZERO) {
        return Err(AppError::validation_displayable("成本价格不能为负数"));
    }
    Ok(())
}

/// 产品安全类别取值域校验（GB 18401-2010 三类，创建与更新共用）。
///
/// 读的是本次提交的安全类别值（键缺席或未录入即 None，不判）。写给谁调用：
/// crud::create_product 与 crud::update_product，以及批量建品逐行前置检查；判据一份，多路同门。
/// 存什么：不落库，只做拒绝。
///
/// 规则。出现值时必须在 product_safety_category::ALL_CLASSES（A/B/C）内，词表外值点名拒绝；
/// 空白串等同未录入放行（必填性不属本判据）。A 类另须满足 GB 31701-2015 这一条件判据是
/// 跨标准联动的考核口径，落在检验判定侧，不在本入参校验里伪装成拒绝。
/// 文案走 validation_displayable：只描述用户自己提交字段的取值规则，不涉表名与 SQL。
pub fn validate_product_safety_category(value: Option<&str>) -> Result<(), AppError> {
    if let Some(v) = value.map(str::trim).filter(|s| !s.is_empty())
        && !product_safety_category::ALL_CLASSES.contains(&v)
    {
        return Err(AppError::validation_displayable(format!(
            "安全类别「{v}」不在词表内，允许值：A、B、C（GB 18401-2010）"
        )));
    }
    Ok(())
}

/// 色牢度等级取值域校验（GB 18401 九档，色号创建与更新共用）。
///
/// 读的是本次色号提交或编辑的色牢度等级值（键缺席或未录入即 None，不判）。写给谁调用：
/// product_ops::color 的 create_product_color 与 update_product_color。判据一份，色牢度权威词表
/// 与 DB CHECK（product_vocab_check 域 m0108）单源，均在 models::status::general::color_fastness。
/// 存什么：不落库，只做拒绝。
///
/// 规则。出现值时必须在 color_fastness::ALL_GRADES 九档内（含半级 1-2、2-3、3-4、4-5，半级是合法档
/// 不是脏值），词表外值点名拒绝。耐唾液项目只对 A 类考核、本色漂白与传统手工着色不考核这两条是
/// 条件性判据，留在检验判定侧，不在本入参校验里表达。
pub fn validate_product_color_fastness_grade(value: Option<&str>) -> Result<(), AppError> {
    if let Some(v) = value.map(str::trim).filter(|s| !s.is_empty())
        && !color_fastness::ALL_GRADES.contains(&v)
    {
        return Err(AppError::validation_displayable(format!(
            "色牢度等级「{v}」不在词表内，允许值：1、1-2、2、2-3、3、3-4、4、4-5、5"
        )));
    }
    Ok(())
}
