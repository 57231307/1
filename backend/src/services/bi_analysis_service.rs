//! BI 多维分析 service（facade）
//!
//! 功能：
//! 1. 维度聚合（按时间/客户/产品/区域/品类）
//! 2. 钻取（年→月、月→日、客户→订单、产品→订单）
//! 3. 切片/切块/上卷/透视
//!
//! 实现策略（v9 批次 130 修复）：
//! - 原 P3-4 关键路径 demo 全部返回硬编码 mock 数据，违反规则 0（真实实现强制）
//! - 现使用 SeaORM raw SQL（Statement::from_sql_and_values + FromQueryResult）真实查询
//!   sales_orders / sales_order_items / customers / products / product_categories 表
//! - 16 个 HTTP 端点对外暴露，前端调用后获得真实聚合数据
//!
//! 批次 490 D10-3a 拆分：本文件作为 facade，保留 helper 函数 + Service struct + new 构造函数 + 测试。
//! BiAnalysisService 的 impl 块迁移至 `bi_analysis_ops` 子模块（sales / profit / drilldown / olap）。
//! 数据结构迁移至 `bi_analysis_ops::types`，本 facade 通过 `pub use` 二次 re-export 保持外部引用路径不变。

use rust_decimal::Decimal;
use sea_orm::DatabaseConnection;
use std::sync::Arc;

use crate::utils::cache::{AppCache, Cache};
use crate::utils::data_scope::{DataScope, DataScopeContext, build_data_scope_sql};
use crate::utils::error::AppError;

// re-export ops 子模块的对外 response struct，保持外部 `use crate::services::bi_analysis_service::{...}` 路径不变
pub use crate::services::bi_analysis_ops::{
    BiResponse, CategoryStat, CustomerRank, KpiSummary, ProductRank, ProfitAnalysis, RegionStat,
    TimeSeriesPoint,
};

/// 缺陷 3.1 修复：BI 查询缓存 TTL（5 分钟，与 dashboard 对齐）
const BI_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(300);

/// 缺陷 3.1 修复：构建 BI 查询缓存键
pub(crate) fn build_bi_cache_key(scope: &DataScopeContext, key_parts: &[&str]) -> String {
    let mut key = format!(
        "bi:{}:{}:{}",
        scope.scope.as_str(),
        scope.user_id,
        scope.department_id.unwrap_or(0)
    );
    for part in key_parts {
        key.push(':');
        key.push_str(part);
    }
    key
}

// ==================== 模块级私有 helper（pub(crate) 供 ops 子模块使用） ====================

/// Decimal → f64 安全转换（避免精度损失，使用 to_string().parse()）
pub(crate) fn dec_to_f64(d: Option<Decimal>) -> f64 {
    d.map(|v| v.to_string().parse::<f64>().unwrap_or(0.0))
        .unwrap_or(0.0)
}

/// 维度到 SQL 表达式映射（v11 批次 144 P1-3：透视矩阵维度映射）
/// 返回 (key_expr, label_expr)：key_expr: 用于 GROUP BY 的唯一标识（text 类型）；label_expr: 用于展示的可读标签；支持的维度：customer: 客户 ID + 客户名称；product: 产品 ID + 产品名称；region: 客户所在省份；category: 产品品类名称；time: 订单月份（YYYY-MM 格式）；批次 252 修复：原 `_ => unreachable!()` 在非法维度时 panic 崩溃，；改为返回 AppError::validation 错误，防御性处理非法输入。
pub fn dim_to_expr(dim: &str) -> Result<(&'static str, &'static str), AppError> {
    match dim {
        "customer" => Ok(("c.id::text", "COALESCE(c.customer_name, '未知客户')")),
        "product" => Ok(("p.id::text", "COALESCE(p.name, '未知产品')")),
        "region" => Ok((
            "COALESCE(c.province, '未知')",
            "COALESCE(c.province, '未知')",
        )),
        "category" => Ok(("COALESCE(pc.name, '未分类')", "COALESCE(pc.name, '未分类')")),
        "time" => Ok((
            "to_char(s.order_date, 'YYYY-MM')",
            "to_char(s.order_date, 'YYYY-MM')",
        )),
        _ => Err(AppError::validation(format!("不支持的维度: {}", dim))),
    }
}

/// 度量聚合表达式生成（批次 252 修复：提取为独立函数，消除 unreachable! panic）
/// 根据 item_level 选择项级或订单级聚合的 SQL 表达式：item_level=true：关联 sales_order_items 表进行项级聚合；item_level=false：订单级聚合，避免 total_amount 重复计算
pub fn measure_to_expr(measure: &str, item_level: bool) -> Result<&'static str, AppError> {
    match (measure, item_level) {
        ("total_amount", true) => Ok("COALESCE(SUM(si.total_amount), 0)"),
        ("order_count", true) => Ok("COUNT(DISTINCT s.id)::numeric"),
        ("quantity", true) => Ok("COALESCE(SUM(si.quantity), 0)"),
        ("profit_amount", true) => Ok(
            "COALESCE(SUM(si.total_amount), 0) - COALESCE(SUM(si.quantity * COALESCE(p.cost_price, 0)), 0)",
        ),
        ("total_amount", false) => Ok("COALESCE(SUM(s.total_amount), 0)"),
        ("order_count", false) => Ok("COUNT(*)::numeric"),
        ("quantity", false) => Ok(
            "COALESCE(SUM((SELECT SUM(si.quantity) FROM sales_order_items si WHERE si.order_id = s.id)), 0)",
        ),
        ("profit_amount", false) => Ok(
            "COALESCE(SUM(s.total_amount), 0) - COALESCE(SUM((SELECT SUM(si.quantity * COALESCE(p.cost_price, 0)) FROM sales_order_items si LEFT JOIN products p ON p.id = si.product_id WHERE si.order_id = s.id)), 0)",
        ),
        _ => Err(AppError::validation(format!("不支持的度量: {}", measure))),
    }
}

// ==================== Service struct 定义（impl 块在 bi_analysis_ops 子模块） ====================

/// BI 多维分析 service
/// v9 批次 130 修复：原全部方法返回硬编码 mock 数据，现真实查询数据库。；查询 sales_orders / sales_order_items / customers / products / product_categories 表，；排除 CANCELLED 和 DRAFT 状态的订单。；V15 P0-B10（Batch 483）：新增 data_scope 字段，所有 raw SQL 查询注入行级数据权限过滤。；All：不过滤（管理员/总经理）；Dept：按 users.department_id 过滤（部门经理）；Self_：按 sales_orders.created_by 过滤（普通员工）
pub struct BiAnalysisService {
    /// 数据库连接（pub(crate) 供 bi_analysis_ops 子模块访问）
    pub(crate) db: Arc<DatabaseConnection>,
    /// V15 P0-B10：行级数据权限上下文，所有查询自动注入（pub(crate) 供 bi_analysis_ops 子模块访问）
    pub(crate) data_scope: DataScopeContext,
    /// 缺陷 3.1 修复：聚合查询结果缓存（5 分钟 TTL），None 时禁用缓存
    pub(crate) cache: Option<Arc<AppCache>>,
}

impl BiAnalysisService {
    /// 创建 BI 服务（默认 All 数据范围，仅用于测试/内部调用）（生产环境应使用 `new_with_data_scope` 注入真实数据范围。）
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self {
            db,
            data_scope: DataScopeContext {
                scope: DataScope::All,
                user_id: 0,
                department_id: None,
            },
            cache: None,
        }
    }

    /// V15 P0-B10：创建带数据范围上下文的 BI 服务（由 handler 调用，从 AuthContext.to_data_scope_context() 注入。）
    pub fn new_with_data_scope(db: Arc<DatabaseConnection>, ctx: DataScopeContext) -> Self {
        Self {
            db,
            data_scope: ctx,
            cache: None,
        }
    }

    /// 缺陷 3.1 修复：创建带缓存的 BI 服务（推荐生产环境使用）
    pub fn new_with_cache(
        db: Arc<DatabaseConnection>,
        ctx: DataScopeContext,
        cache: Arc<AppCache>,
    ) -> Self {
        Self {
            db,
            data_scope: ctx,
            cache: Some(cache),
        }
    }

    /// V15 P0-B10：构建数据范围 SQL 片段（带别名和起始索引）（内部辅助方法，封装 build_data_scope_sql 调用。；pub(crate) 供 bi_analysis_ops 子模块使用。）
    pub(crate) fn scope_sql(
        &self,
        table_alias: &str,
        next_index: usize,
    ) -> (String, Vec<sea_orm::Value>) {
        build_data_scope_sql(&self.data_scope, table_alias, next_index)
    }

    /// 缺陷 3.1 修复：尝试从缓存读取聚合结果，未命中返回 None
    pub(crate) fn try_get_cache<T: serde::de::DeserializeOwned>(&self, key: &str) -> Option<T> {
        let cache = self.cache.as_ref()?;
        let cached = cache.get_bi_cache().get(&key.to_string())?;
        serde_json::from_value(cached).ok()
    }

    /// 缺陷 3.1 修复：写入聚合结果到缓存（5 分钟 TTL）
    pub(crate) fn set_cache<T: serde::Serialize>(&self, key: &str, value: &T) {
        if let Some(cache) = self.cache.as_ref()  && let Ok(v) = serde_json::to_value(value)  {
                cache
                    .get_bi_cache()
                    .set(key.to_string(), v, Some(BI_CACHE_TTL));
            }
        }
    }
}
