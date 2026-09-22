import { request } from './request';
import type { ApiResponse, QueryParams } from '@/types/api';

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

export function getBudgetList(
  params?: QueryParams
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

// v11 批次 159 P2-4 修复：已被 BudgetListTab.vue 接入使用，移除过时 TODO 注释
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
  budget_year: number | null;
  planned_amount: string | number;
  remark: string | null;
  account_subject_id: number | null;
  created_at: string;
  updated_at: string;
}

/** 对齐后端 budget_management_handler::CreateBudgetDto */
export interface CreateBudgetItemPayload {
  /** 预算名称：必填 */
  item_name: string;
  item_code?: string;
  item_type?: string;
  budget_year?: number;
  /** 计划金额：必填 */
  planned_amount: number;
  remark?: string;
}

/** 对齐后端 budget_management_handler::UpdateBudgetDto */
export interface UpdateBudgetItemPayload {
  item_name?: string;
  item_type?: string;
  planned_amount?: number;
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

export interface BudgetItemListQuery extends QueryParams {
  item_type?: string;
  status?: string;
}

export function getBudgetItemList(
  params?: BudgetItemListQuery
): Promise<ApiResponse<{ items: BudgetItem[]; total: number; page: number; page_size: number }>> {
  return request.get('/budgets', { params });
}

export function getBudgetDetail(id: number): Promise<ApiResponse<BudgetItem>> {
  return request.get(`/budgets/${id}`);
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
