// V15 P0-F20 Batch 480：8D 质量管理流程 DTO
//
// 包含：
//   - StartEightDRequest：启动 8D 流程请求体
//   - AdvanceStepPayload：推进到下一 D 阶段的负载（枚举：每阶段必填字段不同）
//   - CloseEightDRequest：关闭 8D 流程请求体
//   - ListEightDQuery：列表查询参数
//
// 11 态状态机推进规则（service 层校验）：
//   not_started → d0_plan：StartEightDRequest（含 prepared_by + plan）
//   d0_plan → d1_team：AdvanceStepPayload::D1Team（含 team_members）
//   d1_team → d2_problem：AdvanceStepPayload::D2Problem（含 problem_description）
//   d2_problem → d3_interim：AdvanceStepPayload::D3Interim（含 interim_action）
//   d3_interim → d4_root_cause：AdvanceStepPayload::D4RootCause（含 method + detail + summary）
//   d4_root_cause → d5_permanent：AdvanceStepPayload::D5Permanent（含 action + owner + due_date）
//   d5_permanent → d6_verify：AdvanceStepPayload::D6Verify（含 verification_result）
//   d6_verify → d7_prevent：AdvanceStepPayload::D7Prevent（含 prevention_action）
//   d7_prevent → d8_recognize：AdvanceStepPayload::D8Recognize（含 closure_summary）
//   d8_recognize → closed：CloseEightDRequest（含 closed_by）

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// 启动 8D 流程请求（not_started → d0_plan）
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct StartEightDRequest {
    pub quality_issue_id: i64,
    /// D0 准备人 user_id
    pub prepared_by: i32,
    /// D0 准备阶段计划说明（可选）
    pub plan: Option<String>,
}

/// 关闭 8D 流程请求（d8_recognize → closed）
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CloseEightDRequest {
    pub closed_by: i32,
}

/// D4 根因分析方法（缺陷 4.2 P1 修复）
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
pub enum RootCauseMethod {
    /// 5Why 五问法（serde rename 为 "5why"，因 Rust 标识符不能以数字开头）
    #[serde(rename = "5why")]
    Why5,
    /// 鱼骨图（Ishikawa）
    #[serde(rename = "fishbone")]
    Fishbone,
    /// 其他方法
    #[serde(rename = "other")]
    Other,
}

impl RootCauseMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Why5 => "5why",
            Self::Fishbone => "fishbone",
            Self::Other => "other",
        }
    }
}

/// 推进到下一 D 阶段的负载枚举
///
/// 调用方根据当前 status 选择对应的 payload 变体；
/// service 层校验 payload 变体与当前 status 的匹配关系。
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(tag = "step", rename_all = "snake_case")]
pub enum AdvanceStepPayload {
    /// d0_plan → d1_team
    D1Team {
        team_members: String,
    },
    /// d1_team → d2_problem
    D2Problem {
        problem_description: String,
    },
    /// d2_problem → d3_interim
    D3Interim {
        interim_action: String,
    },
    /// d3_interim → d4_root_cause
    D4RootCause {
        method: RootCauseMethod,
        /// 根因分析详细过程（5Why 五层追问 / 鱼骨图分类 等，JSON 或文本均可）
        detail: String,
        /// 根因总结
        summary: String,
    },
    /// d4_root_cause → d5_permanent
    D5Permanent {
        permanent_action: String,
        /// 责任人姓名或工号（缺陷 4.3 P1 修复：跟踪 + 超期告警）
        action_owner: String,
        /// 计划完成日期
        due_date: NaiveDate,
    },
    /// d5_permanent → d6_verify
    D6Verify {
        verification_result: String,
    },
    /// d6_verify → d7_prevent
    D7Prevent {
        prevention_action: String,
    },
    /// d7_prevent → d8_recognize
    D8Recognize {
        closure_summary: String,
    },
}

/// 列表查询参数
#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct ListEightDQuery {
    pub page: Option<u64>,
    pub page_size: Option<u64>,
    pub quality_issue_id: Option<i64>,
    pub status: Option<String>,
}
