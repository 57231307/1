import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface ExportApprovalRequest {
  id: number;
  applicant_user_id: number;
  applicant_username: string;
  resource_type: string;
  export_params: Record<string, unknown>;
  estimated_rows: number | null;
  file_format: string | null;
  status: string;
  risk_level: string | null;
  /** 后端 Model.approval_level（i32 必填）：1 一级审批即可，2 需二级审批 */
  approval_level: number;
  approval_context: unknown;
  download_token: string | null;
  token_expires_at: string | null;
  download_count: number;
  max_downloads: number;
  file_path: string | null;
  file_size_bytes: number | null;
  file_checksum: string | null;
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
