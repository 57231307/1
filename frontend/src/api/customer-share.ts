import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface Signature {
  id: number;
  status: string;
  [key: string]: unknown;
}

export function signContract(data: Record<string, unknown>) {
  return request.post('/contract-signatures/sign', data);
}

export function verifySignature(id: number) {
  return request.get(`/contract-signatures/${id}/verify`);
}

/**
 * POST /contract-signatures/{id}/revoke —— 后端 handlers/contract_signature_handler.rs::revoke_signature
 * 仅 Path(contract_id) + AuthContext 提取器，**无 Json 请求体**；多发 body 会被整体丢弃，故不接收任何载荷。
 */
export function revokeSignature(id: number) {
  return request.post(`/contract-signatures/${id}/revoke`);
}

/**
 * GET /contract-signatures —— 后端 contract_signature_handler.rs::list_signed_contracts
 * 仅 State + AuthContext 提取器，**无 Query<T>**（不接受任何筛选参数）；
 * 传 params 会被 Axum 静默丢弃，形成"有筛选框却永无效果"的假功能，故前端不声明参数。
 * 需要筛选能力属后端缺陷，已列入后端串行处理清单。
 */
export function listSignedContracts() {
  return request.get('/contract-signatures');
}

/**
 * POST /customer-shares 请求体，对齐后端 services/crm/customer_team_share_service.rs::ShareCustomerRequest
 * （Deserialize 无 rename_all）：customer_id/shared_to_user_id 非 Option 必填；
 * permission 为 Option<String>，缺省后端默认 view，且服务层 validate_share_permission_type
 * （customer_team_share_service.rs:677-687）强校验词表 **view/edit/full**——
 * 前端旧键名 target_user_id 与旧取值 read/write 均不在后端契约内（read/write 必 422）。
 */
export interface CreateCustomerShareInput {
  customer_id: number;
  shared_to_user_id: number;
  permission?: 'view' | 'edit' | 'full';
  duration_days?: number;
  share_reason?: string;
}

export function createCustomerShare(data: CreateCustomerShareInput) {
  return request.post('/customer-shares', data);
}

export function listCustomerShares(params?: Record<string, unknown>) {
  return request.get('/customer-shares', { params });
}

export function getSharesByCustomer(customerId: number) {
  return request.get('/customer-shares/by-customer', { params: { customer_id: customerId } });
}

export function getSharesByUser(userId: number) {
  return request.get('/customer-shares/by-user', { params: { user_id: userId } });
}

export function checkSharePermission(params: Record<string, unknown>) {
  return request.get<ApiResponse<Record<string, unknown>>>('/customer-shares/check', { params });
}

export function revokeShare(shareId: number, data?: Record<string, unknown>) {
  return request.post(`/customer-shares/revoke`, { share_id: shareId, ...(data ?? {}) });
}

export function expireOverdueShares() {
  return request.post('/customer-shares/expire-overdue');
}

export function addTeamMember(data: Record<string, unknown>) {
  return request.post('/customer-team-members', data);
}

export function removeTeamMember(memberId: number) {
  return request.delete(`/customer-team-members/${memberId}`);
}

export function listTeamMembers(customerId: number) {
  return request.get(`/customer-team-members/by-customer/${customerId}`);
}

export function listUserTeams(userId: number) {
  return request.get<ApiResponse<Record<string, unknown>[]>>('/customer-team-members/by-user', {
    params: { user_id: userId },
  });
}

export function isTeamMember(params: Record<string, unknown>) {
  return request.get<ApiResponse<{ is_member?: boolean }>>('/customer-team-members/check', {
    params,
  });
}
