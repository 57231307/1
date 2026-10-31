import { request } from './request';
import type { ApiResponse } from '@/types/api';

/**
 * 出参键与 backend/src/models/export_approval_request.rs 的 Model 序列化逐字段对齐
 * （snake_case；Option→可空、非 Option→必存在，NOT NULL 列不得标可空）。
 * export_params 为后端 Option<ExportParams> 新类型透传 JSON 值。
 * 令牌生命周期改造（m0105）后出参脱敏：download_token 与 token_hash 恒为 null
 * （后端 handlers 走 masked_json），明文只经 token-once 端点一次性交付一次；
 * token_fingerprint 为永久可见的前缀指纹。
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
  /** 恒为 null（明文仅 token-once 首查返回一次） */
  download_token: null;
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
  /** 审批上下文（后端 Model.context，含 current_approval_step 与失败留痕） */
  context: Record<string, unknown> | null;
  created_at: string;
  updated_at: string;
  completed_at: string | null;
  /** 恒为 null（哈希属服务端校验面，不外发） */
  token_hash: null;
  /** 令牌前缀指纹（exp_ 加明文前 8 位，供辨认） */
  token_fingerprint: string | null;
  /** 明文是否已被申请人取走过（true 后 token-once 只回指纹） */
  token_viewed: boolean;
  /** 未成功前提下的累计重试次数 */
  retry_count: number;
  /** 最近一次导出尝试时刻（重试窗口判定源） */
  last_attempt_at: string | null;
  /** 允许的最大重试次数（默认 3） */
  max_retries: number;
  /** 令牌是否已被成功导出消费 */
  token_consumed: boolean;
  /** 令牌消费时刻 */
  token_consumed_at: string | null;
  /** 关联 BPM 流程实例 ID（审批办理与进度回看的正规入口） */
  bpm_instance_id: number | null;
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
 * GET /export-approvals/verify-token 出参为脱敏后的审批请求 JSON（后端 handler 经
 * masked_json 屏蔽 download_token 与 token_hash），并非 valid 布尔信封；
 * 令牌失效（过期/已消费/次数用尽/窗口未到）走非 2xx 失败信封，文案可区分各态。
 */
export function verifyToken(token: string) {
  return request.get<ApiResponse<ExportApprovalRequest>>('/export-approvals/verify-token', {
    params: { token },
  });
}

/** token-once 出参：首查 available=true 带一次性明文；再查 available=false 只带指纹 */
export interface TokenOnceResult {
  available: boolean;
  token?: string;
  fingerprint?: string;
  message: string;
}

/**
 * GET /export-approvals/:id/token-once 一次性取走令牌明文（仅申请人本人、仅首次）。
 * 后端取走即置 token_viewed 并清空明文列，第二次调用只回指纹。
 */
export function viewTokenOnce(id: number) {
  return request.get<ApiResponse<TokenOnceResult>>(`/export-approvals/${id}/token-once`);
}

/** rotate 出参：新令牌一次性明文（旧令牌已在服务端作废） */
export interface RotateTokenResult {
  available: boolean;
  token: string;
  fingerprint: string;
  message: string;
}

/**
 * POST /export-approvals/:id/rotate-token 重发令牌：旧令牌即刻作废，
 * 返回新令牌明文一次（仅申请人本人）。
 */
export function rotateToken(id: number) {
  return request.post<ApiResponse<RotateTokenResult>>(`/export-approvals/${id}/rotate-token`);
}
