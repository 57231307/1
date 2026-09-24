import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface Budget {
  id: number;
  budget_no: string;
  name: string;
  period: string;
  department_id: number;
  department_name?: string;
  total_amount: number;
  status: 'draft' | 'pending' | 'approved' | 'rejected';
  remark?: string;
  created_at?: string;
  updated_at?: string;
}

export const BUDGET_STATUS = {
  draft: { label: '草稿', type: 'info' },
  pending: { label: '待审核', type: 'warning' },
  approved: { label: '已批准', type: 'success' },
  rejected: { label: '已拒绝', type: 'danger' },
};

/**
 * GET /budgets（列表主体为 budget_items 明细行）查询参数：
 * 后端 handlers/budget_management_handler.rs::list_budgets 读取
 * page / page_size / item_type / status / plan_id 五个键（其余键被静默丢弃），
 * 支持按所属预算方案 plan_id 过滤。故此处仅列这五个真实生效字段。
 */
export interface BudgetListQuery {
  page?: number;
  page_size?: number;
  item_type?: string;
  status?: string;
  /** 按所属预算方案筛选（并入 plan 主线后新增） */
  plan_id?: number;
}

export function getBudgetList(
  params?: BudgetListQuery
): Promise<ApiResponse<{ items: Budget[]; total: number }>> {
  return request.get('/budgets', { params });
}

export function createBudget(data: Partial<Budget>): Promise<ApiResponse<Budget>> {
  return request.post('/budgets', data);
}

export function updateBudget(id: number, data: Partial<Budget>): Promise<ApiResponse<Budget>> {
  return request.put(`/budgets/${id}`, data);
}

export function deleteBudget(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/budgets/${id}`);
}

// 批次 278：已被资产预算审批出口（api/asset.ts re-export）接入使用
export function approveBudget(id: number): Promise<ApiResponse<void>> {
  return request.post(`/budgets/${id}/approve`, {});
}

// ============== 预算审批链（budget_management / budget_version，routes/finance.rs）==============

/**
 * 预算科目（budget_management 表），与后端 models/budget_management.rs::Model 对齐。
 * 状态值：draft 草稿 / pending 待审批 / approved 已批准 / rejected 已驳回 / active 执行中
 */
export type BudgetItemStatus = 'draft' | 'pending' | 'approved' | 'rejected' | 'active';

export interface BudgetItem {
  id: number;
  item_code: string;
  item_name: string;
  parent_id: number | null;
  item_type: string;
  level: number;
  status: BudgetItemStatus;
  /**
   * 所属预算方案 ID（外键 → budget_plans.id）。对齐后端 models/budget_management.rs
   * 的 NOT NULL 列，故此处不可标可选（标 `?` 会掩盖后端缺键）。
   */
  plan_id: number;
  budget_year: number | null;
  planned_amount: string | number;
  remark: string | null;
  account_subject_id: number | null;
  created_at: string;
  updated_at: string;
}

/**
 * 预算明细期间分解行（budget_item_periods 表），对齐后端 models/budget_item_periods.rs::Model。
 * period 形如 '2026-01'（月）/ '2026-Q1'（季）/ '2026-FY'（年度合计）。
 */
export interface BudgetItemPeriod {
  id: number;
  item_id: number;
  period: string;
  planned_amount: string | number;
  actual_amount: string | number;
  created_at: string;
  updated_at: string;
}

/**
 * 明细创建/更新入参里的期间行（对齐后端 BudgetItemPeriodInput）：只带 period + planned_amount，
 * id/item_id/actual_amount/时间戳由服务层在替换时生成。
 */
export interface BudgetItemPeriodInput {
  period: string;
  planned_amount: number | string;
}

/**
 * 详情出参（BudgetItemWithPeriods）：明细主体全列（含 NOT NULL 的 plan_id）+ 同层的 periods 数组。
 * 后端以 #[serde(flatten)] 展平主体，故详情 JSON = BudgetItem 全部键 + periods 键。
 */
export interface BudgetItemWithPeriods extends BudgetItem {
  periods: BudgetItemPeriod[];
}

/**
 * 预算方案（budget_plans 表），对齐后端 models/budget_plan.rs::Model。
 * 状态：draft/approved/rejected/active/closed。
 */
export interface BudgetPlan {
  id: number;
  plan_no: string;
  plan_name: string;
  budget_year: number;
  budget_type: string;
  department_id: number | null;
  total_amount: string | number;
  status: string | null;
  prepared_by: number | null;
  approved_by: number | null;
  approved_at: string | null;
  remark: string | null;
  created_at: string | null;
  updated_at: string | null;
}

/** 创建预算方案入参（对齐 handler CreateBudgetPlanRequest；department_id 后端必填） */
export interface CreateBudgetPlanPayload {
  plan_no?: string;
  plan_name?: string;
  budget_year?: number;
  budget_type?: string;
  department_id: number;
  total_amount?: number;
  remark?: string;
}

/**
 * 预算执行明细（budget_executions 表），对齐后端 models/budget_execution.rs::Model。
 * item_id 为可空列：方案级下达/调整不绑定具体明细时为 null。
 */
export interface BudgetExecution {
  id: number;
  plan_id: number;
  item_id: number | null;
  execution_type: string;
  amount: string | number;
  expense_type: string | null;
  expense_date: string;
  related_document_type: string | null;
  related_document_id: number | null;
  remark: string | null;
  created_by: number | null;
  created_at: string;
}

/**
 * 对齐后端 budget_management_handler::CreateBudgetDto（预算明细行/科目）。
 * item_name（预算名称）与 planned_amount（计划金额）为后端必填字段；
 * plan_id（所属方案）为后端 NOT NULL 必填列（create 必填）——两级模式下由所属方案带出，
 * 明细编辑表单在 UI 层强制必填并始终随请求提交。
 * periods 为空时后端按年度自动生成单条 'FY' 合计行。
 */
export interface CreateBudgetItemPayload {
  item_name: string;
  item_code?: string;
  item_type?: string;
  plan_id?: number;
  budget_year?: number;
  planned_amount: number;
  periods?: BudgetItemPeriodInput[];
  remark?: string;
}

/** 对齐后端 budget_management_handler::UpdateBudgetDto：periods 提供时整体替换该明细期间行 */
export interface UpdateBudgetItemPayload {
  item_name?: string;
  item_type?: string;
  planned_amount?: number;
  periods?: BudgetItemPeriodInput[];
  status?: string;
  remark?: string;
}

/** 预算版本审批记录（budget_version 表），作为审批记录子表数据源 */
export interface BudgetVersion {
  id: number;
  plan_id: number;
  version_no: string;
  version_name: string;
  total_amount: string | number;
  status: string;
  change_reason: string | null;
  approved_by: number | null;
  approved_at: string | null;
  created_by: number;
  created_at: string;
  updated_at: string;
}

export function getBudgetItemList(
  params?: BudgetListQuery
): Promise<ApiResponse<{ items: BudgetItem[]; total: number; page: number; page_size: number }>> {
  return request.get('/budgets', { params });
}

export function getBudgetDetail(id: number): Promise<ApiResponse<BudgetItemWithPeriods>> {
  return request.get(`/budgets/${id}`);
}

/** 明细行详情（/budgets/items/{id}），出参同为 BudgetItemWithPeriods */
export function getBudgetItemDetail(id: number): Promise<ApiResponse<BudgetItemWithPeriods>> {
  return request.get(`/budgets/items/${id}`);
}

export function createBudgetItem(data: CreateBudgetItemPayload): Promise<ApiResponse<BudgetItem>> {
  return request.post('/budgets', data);
}

export function updateBudgetItem(
  id: number,
  data: UpdateBudgetItemPayload
): Promise<ApiResponse<BudgetItem>> {
  return request.put(`/budgets/${id}`, data);
}

/** 提交审批：draft → pending（走 UpdateBudgetDto 的 status 字段） */
export function submitBudget(id: number): Promise<ApiResponse<BudgetItem>> {
  return request.put(`/budgets/${id}`, { status: 'pending' });
}

/** 审批通过：POST /budgets/{id}/approve，请求体 ApproveBudgetDto { opinion? } */
export function approveBudgetRecord(
  id: number,
  opinion?: string
): Promise<ApiResponse<{ id: number }>> {
  return request.post(`/budgets/${id}/approve`, { opinion });
}

/** 驳回：置为 rejected，驳回意见写入 remark（UpdateBudgetDto） */
export function rejectBudget(id: number, opinion?: string): Promise<ApiResponse<BudgetItem>> {
  return request.put(`/budgets/${id}`, { status: 'rejected', remark: opinion });
}

/** 预算调整申请：POST /budgets/adjust（AdjustBudgetRequest） */
export function adjustBudget(data: {
  item_id: number;
  adjust_amount: number;
  reason?: string;
}): Promise<ApiResponse<unknown>> {
  return request.post('/budgets/adjust', data);
}

/** 审批通过预算调整：POST /budgets/adjust/{id}/approve */
export function approveBudgetAdjustment(id: number): Promise<ApiResponse<unknown>> {
  return request.post(`/budgets/adjust/${id}/approve`);
}

/** 驳回预算调整：POST /budgets/adjust/{id}/reject */
export function rejectBudgetAdjustment(id: number): Promise<ApiResponse<unknown>> {
  return request.post(`/budgets/adjust/${id}/reject`);
}

/** 审批记录子表：GET /budgets/versions/{plan_id}，返回该方案的版本审批历史 */
export function getBudgetVersions(planId: number): Promise<ApiResponse<BudgetVersion[]>> {
  return request.get(`/budgets/versions/${planId}`);
}

// ============== 预算方案（budget_plans，routes/finance.rs /budgets/plans）==============

/**
 * 预算方案列表：GET /budgets/plans（后端返回 Vec<budget_plan::Model>，顶层非分页包装）。
 * page / page_size 作为查询参数传入（后端 clamp），出参为方案数组。
 */
export function getBudgetPlanList(params?: {
  page?: number;
  page_size?: number;
}): Promise<ApiResponse<BudgetPlan[]>> {
  return request.get('/budgets/plans', { params });
}

/** 创建预算方案：POST /budgets/plans（department_id 后端必填，缺失返回 4xx） */
export function createBudgetPlan(data: CreateBudgetPlanPayload): Promise<ApiResponse<BudgetPlan>> {
  return request.post('/budgets/plans', data);
}

/** 审批方案：POST /budgets/plans/{id}/approve，请求体 BudgetApproveRequest { approval_comment? } */
export function approveBudgetPlan(
  id: number,
  approvalComment?: string
): Promise<ApiResponse<string>> {
  return request.post(`/budgets/plans/${id}/approve`, { approval_comment: approvalComment });
}

/** 驳回方案：POST /budgets/plans/{id}/reject */
export function rejectBudgetPlan(
  id: number,
  approvalComment?: string
): Promise<ApiResponse<string>> {
  return request.post(`/budgets/plans/${id}/reject`, { approval_comment: approvalComment });
}

/** 方案执行明细：GET /budgets/plans/{id}/executions，返回 Vec<budget_execution::Model> */
export function getPlanExecutions(planId: number): Promise<ApiResponse<BudgetExecution[]>> {
  return request.get(`/budgets/plans/${planId}/executions`);
}
