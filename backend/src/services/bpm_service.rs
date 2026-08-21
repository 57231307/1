//! BPM 工作流服务（facade）
//!
//! 本文件为 facade 入口，仅保留：
//! - `BpmService` struct 定义 + `new` 构造函数
//! - 纯函数 `evaluate_bpm_condition`（边条件表达式评估，`pub(crate)`）
//! - 纯函数 `resolve_first_task_node`（从流程定义解析首个任务节点，`pub(crate)`）
//! - `BPM_CONDITION_RE` 静态正则（LazyLock 全局编译一次，私有）
//!
//! 业务实现已按职责拆分到 [`crate::services::bpm_ops`] 子模块：
//! - `instance`：流程实例生命周期（start_process / cancel_instance / 查询 / 审批链 / 业务关联）
//! - `task`：任务审批流（approve_task + 推进/拒绝/完成 + 查询/转办/催办）
//! - `monitor`：流程监控统计（实例/任务统计 + 待处理任务列表 + 实例列表）
//!
//! `db` 字段使用 `pub(crate)` 可见性，bpm_ops 子模块的 impl 块可直接访问。
//! 外部调用路径不变：`crate::services::bpm_service::BpmService` 保持稳定。

use sea_orm::DatabaseConnection;
use std::sync::Arc;

// 批次 23（2026-06-29 v5 P0-2）：BPM 条件正则改为 LazyLock 全局编译一次
// 原实现每次调用 evaluate_bpm_condition 都执行 Regex::new，涉及 NFA→DFA 构造开销。
// BPM 审批是中频操作，每次审批可能扫描多条带条件的边，重复编译正则是性能瓶颈。
// 批次 404 修复：正则编译失败时优雅降级（条件匹配返回 false 而非 panic）。
static BPM_CONDITION_RE: std::sync::LazyLock<Option<regex::Regex>> =
    std::sync::LazyLock::new(|| regex::Regex::new(r"\$\{(\w+)\}\s*(==|!=|>|<|>=|<=)\s*(.+)").ok());

/// 评估 BPM 边条件表达式
/// 支持的条件格式:`${amount} > 10000` - 变量数值比较；`${status} == 'APPROVED'` - 变量字符串比较；
/// A.14 修复：支持 `&&`（AND）和 `||`（OR）组合，如 `${amount} > 10000 && ${status} == 'APPROVED'`。
/// `pub(crate)`：bpm_ops::task 子模块的 `try_advance_to_next_node` 调用。
pub(crate) fn evaluate_bpm_condition(
    condition: &str,
    variables: &Option<serde_json::Value>,
) -> bool {
    let vars = match variables {
        Some(v) => v,
        None => return false,
    };

    let condition = condition.trim();
    if condition.is_empty() {
        return true; // 无条件默认通过
    }

    // A.14 修复：支持 || (OR) 和 && (AND) 组合
    // 先按 || 分割（任一为真则整体为真），每段再按 && 分割（全部为真则段为真）
    let or_parts: Vec<&str> = condition.split("||").map(|s| s.trim()).collect();
    if or_parts.len() > 1 {
        return or_parts
            .iter()
            .any(|part| evaluate_single_bpm_condition(part, vars));
    }
    let and_parts: Vec<&str> = condition.split("&&").map(|s| s.trim()).collect();
    if and_parts.len() > 1 {
        return and_parts
            .iter()
            .all(|part| evaluate_single_bpm_condition(part, vars));
    }

    evaluate_single_bpm_condition(condition, vars)
}

/// 评估单个 BPM 条件表达式（不含 && / || 组合）
fn evaluate_single_bpm_condition(condition: &str, vars: &serde_json::Value) -> bool {
    let condition = condition.trim();
    if condition.is_empty() {
        return true;
    }

    // 提取变量名和比较操作: ${var_name} operator value（使用全局编译的正则）
    // 批次 404 修复：正则编译失败时优雅降级（返回 None → 条件不匹配）
    if let Some(caps) = BPM_CONDITION_RE
        .as_ref()
        .and_then(|re| re.captures(condition))
    {
        let var_name = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        let operator = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let expected_value = caps.get(3).map(|m| m.as_str()).unwrap_or("").trim();

        // 获取实际变量值
        let actual_value = vars.get(var_name).and_then(|v| {
            v.as_str()
                .map(|s| s.to_string())
                .or_else(|| v.as_i64().map(|i| i.to_string()))
                .or_else(|| v.as_f64().map(|f| f.to_string()))
        });

        match actual_value {
            Some(actual) => {
                // 尝试数值比较
                if let (Ok(actual_num), Ok(expected_num)) =
                    (actual.parse::<f64>(), expected_value.parse::<f64>())
                {
                    match operator {
                        ">" => actual_num > expected_num,
                        "<" => actual_num < expected_num,
                        ">=" => actual_num >= expected_num,
                        "<=" => actual_num <= expected_num,
                        "==" => (actual_num - expected_num).abs() < f64::EPSILON,
                        "!=" => (actual_num - expected_num).abs() >= f64::EPSILON,
                        _ => false,
                    }
                } else {
                    // 字符串比较
                    let expected = expected_value.trim_matches('\'').trim_matches('"');
                    match operator {
                        "==" => actual == expected,
                        "!=" => actual != expected,
                        _ => false,
                    }
                }
            }
            None => false,
        }
    } else {
        // 安全修复：无法解析的条件时 fail-closed（默认拒绝），防止审批被绕过
        tracing::warn!(
            "无法解析 BPM 条件表达式: {}，默认拒绝（fail-closed）",
            condition
        );
        false
    }
}

/// BPM 工作流服务
/// struct 定义保留在 facade，impl 业务方法块按职责分散到 `bpm_ops/` 子模块；（instance / task / monitor）。`db` 字段为 `pub(crate)` 供 ops 子模块访问。
pub struct BpmService {
    // 批次 67：db 字段改为 pub(crate)，允许 bpm_process_definition_service.rs 等 crate 内其他模块访问
    pub(crate) db: Arc<DatabaseConnection>,
}

impl BpmService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    /// 从流程定义配置中解析首个任务节点（`pub(crate)`：bpm_ops::instance 子模块的 `create_first_task_or_complete` 调用。）
    pub(crate) fn resolve_first_task_node(
        flow_def: &serde_json::Value,
    ) -> Option<&serde_json::Value> {
        let nodes = flow_def.get("nodes").and_then(|n| n.as_array())?;

        // 查找 start_event 节点
        let start_node = nodes
            .iter()
            .find(|n| n.get("type").and_then(|t| t.as_str()) == Some("start_event"));

        let mut first_task_node = None;

        if let Some(start) = start_node {
            let start_id = start.get("id").and_then(|i| i.as_str()).unwrap_or("");
            if let Some(edges) = flow_def.get("edges").and_then(|e| e.as_array()) {
                if let Some(edge) = edges
                    .iter()
                    .find(|e| e.get("source").and_then(|s| s.as_str()) == Some(start_id))
                {
                    let target_id = edge.get("target").and_then(|t| t.as_str()).unwrap_or("");
                    first_task_node = nodes
                        .iter()
                        .find(|n| n.get("id").and_then(|i| i.as_str()) == Some(target_id));
                }
            }
        }

        // 未通过 start_event 边找到，则回退到第一个 user_task
        if first_task_node.is_none() {
            first_task_node = nodes
                .iter()
                .find(|n| n.get("type").and_then(|t| t.as_str()) == Some("user_task"));
        }

        first_task_node
    }
}
