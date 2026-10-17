import { request } from './request';
import type { ApiResponse } from '@/types/api';

/**
 * 出参键与 backend/src/models/export_approval_request.rs 的 Model 序列化逐字段对齐
 * （snake_case；Option→可空、非 Option→必存在，NOT NULL 列不得标可空）。
 * export_params 为后端 Option<ExportParams> 新类型透传 JSON 值。
 */
export interface ExportApprovalRequest {
  id: number;
  applicant_user_id: number;
  applicant_username: string;
  approver_user_id: number | null;
  approver_username: string | null;
  resource_type: string;
  export_params: Record<string, unknown> | null;
  estimated_rows: number | null;
  file_format: string;
  status: string;
  approval_level: number;
  approver_comments: string | null;
  approved_at: string | null;
  rejected_at: string | null;
  download_token: string | null;
  token_expires_at: string | null;
  download_count: number;
  max_downloads: number;
  file_path: string | null;
  file_size_bytes: number | null;
  file_checksum: string | null;
  applicant_ip: string | null;
  approver_ip: string | null;
  applicant_user_agent: string | null;
  /** 风险等级（后端 Model.risk_level，String NOT NULL） */
  risk_level: string;
  /** 审批上下文（后端 Model.context，含 current_approval_step） */
  context: Record<string, unknown> | null;
  created_at: string;
  updated_at: string;
  completed_at: string | null;
}

export interface CreateApprovalPayload {
  resource_type: string;
  export_params: Record<string, unknown>;
  estimated_rows?: number;
  file_format?: string;
}

export interface ListApprovalQuery {
  page?: number;
  page_size?: number;
  status?: string;
  resource_type?: string;
  applicant_user_id?: number;
}

export function createApproval(body: CreateApprovalPayload) {
  return request.post<ApiResponse<ExportApprovalRequest>>('/export-approvals', body);
}

export function listApprovals(params: ListApprovalQuery) {
  return request.get<
    ApiResponse<{ items: ExportApprovalRequest[]; total: number; page: number; page_size: number }>
  >('/export-approvals', { params });
}

export function listPendingForMe(params: ListApprovalQuery) {
  return request.get<
    ApiResponse<{ items: ExportApprovalRequest[]; total: number; page: number; page_size: number }>
  >('/export-approvals/pending-for-me', { params });
}

export function getApprovalDetail(id: number) {
  return request.get<ApiResponse<ExportApprovalRequest>>(`/export-approvals/${id}`);
}

export function approveRequest(id: number, comments?: string) {
  return request.post<ApiResponse<ExportApprovalRequest>>(`/export-approvals/${id}/approve`, {
    comments,
  });
}

export function rejectRequest(id: number, comments?: string) {
  return request.post<ApiResponse<ExportApprovalRequest>>(`/export-approvals/${id}/reject`, {
    comments,
  });
}

export function cancelRequest(id: number) {
  return request.post<ApiResponse<ExportApprovalRequest>>(`/export-approvals/${id}/cancel`);
}

/**
 * GET /export-approvals/verify-token 出参为完整审批请求 Model（后端 handler 直接
 * serde_json::to_value(model)，含 status/download_token/token_expires_at/download_count 等），
 * 并非 valid 布尔信封；令牌失效（过期/次数超限/状态非 approved）走非 2xx 失败信封。
 */
export function verifyToken(token: string) {
  return request.get<ApiResponse<ExportApprovalRequest>>('/export-approvals/verify-token', {
    params: { token },
  });
}
