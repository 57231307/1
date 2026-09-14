import { request } from './request';

// 权限委托
export function getDelegationList(params?: Record<string, unknown>) {
  return request.get('/permission-delegations', { params });
}

export function getActiveDelegatedPermissions(delegateeId: number) {
  return request.get(`/permission-delegations/active/${delegateeId}`);
}

export function checkDelegatedPermission(params: { delegatee_id: number; permission: string }) {
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

export function checkMutualExclusive(roleA: string, roleB: string) {
  return request.get(`/role-relations/check-mutual-exclusive/${roleA}/${roleB}`);
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
  return request.post(`/ai-models/evaluations/${versionId}/drift`);
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
