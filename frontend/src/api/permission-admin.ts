import { request } from './request';
import type { ApiResponse, PaginatedResponse, QueryParams } from '@/types/api';

/**
 * 角色变更审批状态
 * 后端状态机：pending_l1 → pending_l2 → approved / rejected / cancelled
 */
export type RoleChangeApprovalStatus =
  | 'pending_l1'
  | 'pending_l2'
  | 'approved'
  | 'rejected'
  | 'cancelled';

/** 角色变更类型（后端 ChangeType 枚举） */
export type RoleChangeType = 'assign_role' | 'assign_permission' | 'remove_permission';

export interface RoleChangeApproval {
  id: number;
  approval_no: string;
  change_type: RoleChangeType | string;
  target_user_id: number | null;
  target_role_id: number;
  target_role_code: string;
  proposed_permission_id: number | null;
  proposed_resource_type: string | null;
  proposed_action: string | null;
  proposed_allowed: boolean | null;
  applicant_id: number;
  applicant_username: string;
  l1_approver_id: number | null;
  l1_approver_name: string | null;
  l1_comments: string | null;
  l1_approved_at: string | null;
  l2_approver_id: number | null;
  l2_approver_name: string | null;
  l2_comments: string | null;
  l2_approved_at: string | null;
  status: RoleChangeApprovalStatus | string;
  current_level: number;
  completed_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface RoleChangeApprovalQuery extends QueryParams {
  status?: string;
  change_type?: string;
  applicant_id?: number;
}

export interface CreateRoleChangeApprovalPayload {
  /** 变更类型：assign_role / assign_permission / remove_permission */
  change_type: RoleChangeType | string;
  target_user_id?: number;
  target_role_id: number;
  target_role_code: string;
  proposed_permission_id?: number;
  proposed_resource_type?: string;
  proposed_action?: string;
  proposed_allowed?: boolean;
}

export interface ApproveRoleChangePayload {
  comments?: string;
}

export interface RoleRelation {
  id: number;
  parent_role_code: string;
  child_role_code: string;
  relation_type: 'inherit' | 'mutual_exclusive' | string;
  description: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreateRoleRelationPayload {
  parent_role_code: string;
  child_role_code: string;
  relation_type: 'inherit' | 'mutual_exclusive' | string;
  description?: string;
}

export interface PermissionDelegation {
  id: number;
  delegator_id: number;
  delegatee_id: number;
  permission_code: string;
  valid_from: string;
  valid_until: string;
  is_chain_allowed: boolean;
  status: 'pending' | 'active' | 'expired' | 'revoked' | string;
  reason: string | null;
  revoked_at: string | null;
  created_at: string;
  updated_at: string;
}

export interface CreatePermissionDelegationPayload {
  delegator_id: number;
  delegatee_id: number;
  permission_code: string;
  valid_from: string;
  valid_until: string;
  is_chain_allowed?: boolean;
  reason?: string;
}

export interface PermissionDelegationListQuery extends QueryParams {
  /** 过滤用户 ID（配合 as_delegator 决定按委托人/被委托人过滤） */
  user_id?: number;
  /** true=按委托人（delegator_id）过滤，false=按被委托人（delegatee_id）过滤 */
  as_delegator?: boolean;
}

export interface RevokePermissionDelegationPayload {
  revoke_reason?: string;
}

export interface CheckDelegatedPermissionQuery {
  delegatee_id: number;
  permission_code: string;
}

// ================ 角色变更审批 ================

export function getRoleChangeApprovals(
  params?: RoleChangeApprovalQuery
): Promise<ApiResponse<PaginatedResponse<RoleChangeApproval>>> {
  return request.get('/role-change-approvals', { params });
}

export function getRoleChangeApproval(id: number): Promise<ApiResponse<RoleChangeApproval>> {
  return request.get(`/role-change-approvals/${id}`);
}

export function createRoleChangeApproval(
  data: CreateRoleChangeApprovalPayload
): Promise<ApiResponse<RoleChangeApproval>> {
  return request.post('/role-change-approvals', data);
}

export function approveRoleChangeL1(
  id: number,
  data?: ApproveRoleChangePayload
): Promise<ApiResponse<RoleChangeApproval>> {
  return request.post(`/role-change-approvals/${id}/approve-l1`, data ?? {});
}

export function approveRoleChangeL2(
  id: number,
  data?: ApproveRoleChangePayload
): Promise<ApiResponse<RoleChangeApproval>> {
  return request.post(`/role-change-approvals/${id}/approve-l2`, data ?? {});
}

export function rejectRoleChangeApproval(
  id: number,
  data?: ApproveRoleChangePayload
): Promise<ApiResponse<RoleChangeApproval>> {
  return request.post(`/role-change-approvals/${id}/reject`, data ?? {});
}

export function cancelRoleChangeApproval(id: number): Promise<ApiResponse<RoleChangeApproval>> {
  return request.post(`/role-change-approvals/${id}/cancel`);
}

// ================ 权限委托 ================

export function getPermissionDelegations(
  params?: PermissionDelegationListQuery
): Promise<ApiResponse<PermissionDelegation[]>> {
  return request.get('/permission-delegations', { params });
}

export function getPermissionDelegation(id: number): Promise<ApiResponse<PermissionDelegation>> {
  return request.get(`/permission-delegations/${id}`);
}

export function createPermissionDelegation(
  data: CreatePermissionDelegationPayload
): Promise<ApiResponse<PermissionDelegation>> {
  return request.post('/permission-delegations', data);
}

export function revokePermissionDelegation(
  id: number,
  data?: RevokePermissionDelegationPayload
): Promise<ApiResponse<void>> {
  return request.post(`/permission-delegations/${id}/revoke`, data ?? {});
}

export function getActiveDelegatedPermissions(
  delegateeId: number
): Promise<ApiResponse<PermissionDelegation[]>> {
  return request.get(`/permission-delegations/active/${delegateeId}`);
}

export function checkDelegatedPermission(
  params: CheckDelegatedPermissionQuery
): Promise<ApiResponse<boolean>> {
  return request.get('/permission-delegations/check', { params });
}

export function expireOverdueDelegations(): Promise<ApiResponse<number>> {
  return request.post('/permission-delegations/expire-overdue');
}

// ================ 角色关系（继承/互斥） ================

export function getRoleRelations(
  params?: { relation_type?: string }
): Promise<ApiResponse<RoleRelation[]>> {
  return request.get('/role-relations', { params });
}

export function createRoleRelation(
  data: CreateRoleRelationPayload
): Promise<ApiResponse<RoleRelation>> {
  return request.post('/role-relations', data);
}

export function deleteRoleRelation(relationId: number): Promise<ApiResponse<void>> {
  return request.delete(`/role-relations/${relationId}`);
}

export function getRelationBetween(
  roleACode: string,
  roleBCode: string
): Promise<ApiResponse<RoleRelation[]>> {
  return request.get(`/role-relations/between/${roleACode}/${roleBCode}`);
}

export function getInheritedRoleCodes(roleCode: string): Promise<ApiResponse<string[]>> {
  return request.get(`/role-relations/inherited/${roleCode}`);
}

export function checkMutualExclusive(
  roleCode: string,
  data: { existing_role_codes: string[] }
): Promise<ApiResponse<{ is_exclusive: boolean; role_code: string }>> {
  return request.post(`/role-relations/check-mutual-exclusive/${roleCode}`, data);
}
