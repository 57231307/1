import { request } from './request';

// 权限委托
export function getDelegationList(params?: Record<string, unknown>) {
  return request.get('/permission-delegations', { params });
}

export function getActiveDelegatedPermissions(delegateeId: number) {
  return request.get(`/permission-delegations/active/${delegateeId}`);
}

// 查询键唯一真相：permission_delegation_handler::DelegatedPermissionQuery
// （delegatee_id: i32 必填、permission_code: String 必填，Query<T> 缺失即 400；
// 原键名 permission 后端不存在）。
export function checkDelegatedPermission(params: {
  delegatee_id: number;
  permission_code: string;
}) {
  return request.get('/permission-delegations/check', { params });
}

export function expireOverdueDelegations() {
  return request.post('/permission-delegations/expire-overdue');
}

export function getDelegation(id: number) {
  return request.get(`/permission-delegations/${id}`);
}

export function revokeDelegation(id: number) {
  return request.post(`/permission-delegations/${id}/revoke`);
}

// 角色关系
export function getRoleRelations(params?: Record<string, unknown>) {
  return request.get('/role-relations', { params });
}

export function createRoleRelation(data: Record<string, unknown>) {
  return request.post('/role-relations', data);
}

export function getRoleRelationsBetween(roleA: string, roleB: string) {
  return request.get(`/role-relations/between/${roleA}/${roleB}`);
}

export function getInheritedRoles(role: string) {
  return request.get(`/role-relations/inherited/${role}`);
}

export function deleteRoleRelation(id: number) {
  return request.delete(`/role-relations/${id}`);
}

// AI 模型治理
export function createModelVersion(data: Record<string, unknown>) {
  return request.post('/ai-models/versions', data);
}

export function getModelVersionList(params?: Record<string, unknown>) {
  return request.get('/ai-models/versions', { params });
}

export function getActiveModelVersion(modelName: string) {
  return request.get(`/ai-models/versions/active/${modelName}`);
}

export function approveModelVersion(versionId: number, data?: Record<string, unknown>) {
  return request.post(`/ai-models/versions/${versionId}/approve`, data ?? {});
}

export function changeModelStatus(versionId: number, data: Record<string, unknown>) {
  return request.post(`/ai-models/versions/${versionId}/status`, data);
}

export function createModelEvaluation(data: Record<string, unknown>) {
  return request.post('/ai-models/evaluations', data);
}

export function getModelEvaluations(versionId: number) {
  return request.get(`/ai-models/evaluations/${versionId}`);
}

export function detectModelDrift(versionId: number) {
  return request.get(`/ai-models/evaluations/${versionId}/drift`);
}

export function logDecision(data: Record<string, unknown>) {
  return request.post('/ai-models/decisions', data);
}

export function getDecisionLogs(params?: Record<string, unknown>) {
  return request.get('/ai-models/decisions', { params });
}

export function reconcileMonthly() {
  return request.post('/ai-models/reconcile');
}

export function getAccuracyReports(params?: Record<string, unknown>) {
  return request.get('/ai-models/accuracy-reports', { params });
}

// 设备连接
export function getDeviceList(params?: Record<string, unknown>) {
  return request.get('/device-connections', { params });
}

export function registerDevice(data: Record<string, unknown>) {
  return request.post('/device-connections/register', data);
}

export function getOnlineDeviceCount() {
  return request.get('/device-connections/online/count');
}

// ===== 角色变更审批（合并自 permission-admin.ts，状态机 pending_l1 → pending_l2 → approved/rejected/cancelled）=====

/** 角色变更审批状态 */
export type RoleChangeApprovalStatus =
  'pending_l1' | 'pending_l2' | 'approved' | 'rejected' | 'cancelled';

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
  [key: string]: unknown;
}

export interface RoleChangeApprovalQuery {
  page?: number;
  page_size?: number;
  status?: RoleChangeApprovalStatus | string;
  change_type?: RoleChangeType | string;
}

/**
 * POST /role-change-approvals 载荷（唯一真相：role_change_approval_service::CreateRoleChangeApprovalRequest）。
 * change_type/target_role_id/target_role_code 后端非 Option 必填（缺失 → serde 422）；
 * change_type 取值域为服务层白名单 assign_role|assign_permission|remove_permission（越界 422）。
 * 后端 DTO 与审批表均无 reason 字段（原载荷 reason 被静默丢弃——变更原因落库属后端缺口，
 * 已登记串行清单），故不再声明。
 */
export interface CreateRoleChangeApprovalPayload {
  change_type: RoleChangeType;
  target_user_id?: number;
  target_role_id: number;
  target_role_code: string;
  proposed_permission_id?: number;
  proposed_resource_type?: string;
  proposed_action?: string;
  proposed_allowed?: boolean;
}

/**
 * L1/L2 审批、驳回共用载荷（唯一真相：role_change_approval_service::ApproveRoleChangeRequest，
 * 三处 handler 均 Json<ApproveRoleChangeRequest>，后端落 approverN_comment 的键为 comments；
 * 原键名 opinion 后端不存在，被 serde 静默丢弃 → 审批意见恒空）。
 */
export interface ApproveRoleChangePayload {
  comments?: string;
}

/** 委托记录行（唯一真相：models/permission_delegation.rs Model 原样序列化，单数 permission_code） */
export interface PermissionDelegation {
  id: number;
  delegator_id: number;
  delegatee_id: number;
  permission_code: string;
  valid_from: string;
  valid_until: string;
  is_chain_allowed: boolean;
  status: string;
  reason: string | null;
  revoked_at: string | null;
  revoked_by: number | null;
  revoke_reason: string | null;
  created_by: number | null;
  created_at: string;
  updated_at: string;
}

/**
 * POST /permission-delegations 载荷（唯一真相：permission_delegation_service::CreateDelegationRequest）。
 * delegator_id/delegatee_id/permission_code/valid_from/valid_until 均为后端非 Option 必填；
 * 一条委托仅一个 permission_code（后端列为 String，批量多码属后端缺口，已登记串行清单）；
 * valid_from/valid_until 为 DateTime<Utc> ← 必须发 RFC3339（new Date(...).toISOString()），
 * 且后端校验 valid_from≥now-5min、valid_until>valid_from、时长≤90 天。
 */
export interface CreatePermissionDelegationPayload {
  delegator_id: number;
  delegatee_id: number;
  permission_code: string;
  valid_from: string;
  valid_until: string;
  is_chain_allowed?: boolean;
  reason?: string;
}

export function getRoleChangeApprovals(params?: RoleChangeApprovalQuery) {
  return request.get('/role-change-approvals', { params });
}

export function getRoleChangeApproval(id: number) {
  return request.get(`/role-change-approvals/${id}`);
}

export function createRoleChangeApproval(data: CreateRoleChangeApprovalPayload) {
  return request.post('/role-change-approvals', data);
}

export function approveRoleChangeL1(id: number, data?: ApproveRoleChangePayload) {
  return request.post(`/role-change-approvals/${id}/approve-l1`, data ?? {});
}

export function approveRoleChangeL2(id: number, data?: ApproveRoleChangePayload) {
  return request.post(`/role-change-approvals/${id}/approve-l2`, data ?? {});
}

export function rejectRoleChangeApproval(id: number, data?: ApproveRoleChangePayload) {
  return request.post(`/role-change-approvals/${id}/reject`, data ?? {});
}

export function cancelRoleChangeApproval(id: number) {
  return request.post(`/role-change-approvals/${id}/cancel`);
}

export function createPermissionDelegation(data: CreatePermissionDelegationPayload) {
  return request.post('/permission-delegations', data);
}

export function getRelationBetween(roleACode: string, roleBCode: string) {
  return request.get(`/role-relations/between/${roleACode}/${roleBCode}`);
}

export function getInheritedRoleCodes(roleCode: string) {
  return request.get(`/role-relations/inherited/${roleCode}`);
}

export function checkMutualExclusive(roleCode: string, data: { existing_role_codes: string[] }) {
  return request.post(`/role-relations/check-mutual-exclusive/${roleCode}`, data);
}

// ===== 设备连接扩展（合并自 device-connection.ts）=====

/** 设备连接状态：后端按 last_heartbeat_at 超时窗口判定 online/timeout/offline */
export type DeviceConnectionStatus = 'online' | 'offline' | 'timeout';

/** 设备类型（RegisterDeviceRequest.device_type） */
export type DeviceType = 'pda' | 'industrial_terminal' | 'scanner' | 'other';

export function getDeviceConnection(deviceId: string) {
  return request.get(`/device-connections/${deviceId}`);
}

export function sendDeviceHeartbeat(deviceId: string, data?: Record<string, unknown>) {
  return request.post(`/device-connections/${deviceId}/heartbeat`, data ?? {});
}

export function disconnectDevice(deviceId: string) {
  return request.post(`/device-connections/${deviceId}/disconnect`);
}

export function cleanupTimeoutDevices() {
  return request.post('/device-connections/cleanup-timeout');
}
