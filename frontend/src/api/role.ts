import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface Role {
  id: number;
  name: string;
  code: string;
  description?: string;
  // 后端 RoleResponse 真实字段：无 status 列（roles 表不提供启停用语义）
  is_system: boolean;
  created_at: string;
  updated_at: string;
  permissions?: Permission[];
}

export interface Permission {
  // 后端 PermissionResponse 真实字段（/permissions、/roles/{id}/permissions 均返回该结构，无 name/code/type）：
  // id 为"角色-权限关联记录 id"（仅 DELETE /roles/permissions/{id} 有意义）。
  // 赋权语义是 resource_type + action + allowed（与 v-permission 码 `${resource_type}:${action}` 对齐）。
  id: number;
  resource_type: string;
  resource_id?: number | null;
  action: string;
  allowed: boolean;
  // 以下为旧契约遗留的可选字段（后端不返回，保持类型兼容）
  name?: string;
  code?: string;
  type?: string;
  parent_id?: number;
}

export interface RoleCreateRequest {
  name: string;
  code: string;
  description?: string;
}

// 与后端 UpdateRolePayload（name/description/is_system）对齐；
// 旧字段 status 后端不识别（roles 表无该列），已移除。
export interface RoleUpdateRequest {
  name?: string;
  description?: string;
}

// 与后端 AssignPermissionPayload 对齐：单条赋权模式（resource_type+action+allowed）。
// 旧字段 permission_ids 后端不识别（axum Json 强类型反序列化直接 400），已废弃。
export interface AssignPermissionRequest {
  resource_type: string;
  resource_id?: number | null;
  action: string;
  allowed: boolean;
}

// 整批替换端点的单条期望授予，字段与后端 BatchPermissionEntry
// （backend/src/services/role_permission_service.rs）逐字一致：三项均必填、snake_case。
// 该结构刻意不含 resource_id：整批替换的作用面只有 resource_id IS NULL 的角色级授予行，
// 记录级授权行仍由单条 POST/DELETE 维护。
export interface RolePermissionBatchEntry {
  resource_type: string;
  action: string;
  allowed: boolean;
}

// 后端 role_handler::list_roles 返回 ApiResponse<RoleListResponse>，
// RoleListResponse { roles: Vec<RoleResponse>; total: u64 }（信封键为 roles，非 items）。
// 后端 role_handler::list_roles 无 Query<T> 提取器（一次性返回全部角色），
// 页面若展示分页控件即为客户端分页，不能假装是服务端分页。
export function getRoleList(): Promise<ApiResponse<{ roles: Role[]; total: number }>> {
  return request.get('/roles');
}

export function getRole(id: number): Promise<ApiResponse<Role>> {
  return request.get(`/roles/${id}`);
}

export function createRole(data: RoleCreateRequest): Promise<ApiResponse<Role>> {
  return request.post('/roles', data);
}

export function updateRole(id: number, data: RoleUpdateRequest): Promise<ApiResponse<Role>> {
  return request.put(`/roles/${id}`, data);
}

export function deleteRole(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/roles/${id}`);
}

export function getRolePermissions(id: number): Promise<ApiResponse<Permission[]>> {
  return request.get(`/roles/${id}/permissions`);
}

// 单条赋权：作用面是记录级授权行（可带 resource_id）。角色级授予的批量维护走
// replaceRolePermissions（整批替换），不要把逐条 POST 当成角色级保存路径。
export function assignPermission(
  id: number,
  data: AssignPermissionRequest
): Promise<ApiResponse<void>> {
  return request.post(`/roles/${id}/permissions`, data);
}

// 按关联行主键删除单条授予记录（记录级授权行用）；角色级授予的删除由整批替换按
// 「期望集外的既有行」处理，不需要逐条 DELETE。
export function deletePermission(
  _roleId: number,
  permissionId: number
): Promise<ApiResponse<void>> {
  return request.delete(`/roles/permissions/${permissionId}`);
}

// 整批替换角色的角色级权限授予：一次请求提交期望的完整终集合（裸数组，非对象包装），
// 服务端在同一事务内做新增/仅 allowed 翻转的更新/期望集外的删除三态复算并写一条批量审计。
// 入参：角色 id + 目标终集合；出参：替换后的角色级授予全集（与 GET /roles/{id}/permissions 同形状）。
// 权限门：后端 handler 首行 require_admin_role，非管理员角色调用必返 403 + FORBIDDEN，
// 调用方的入口/按钮可见性必须与该门一致，不得留下点下去必然失败的入口。
export function replaceRolePermissions(
  id: number,
  entries: RolePermissionBatchEntry[]
): Promise<ApiResponse<Permission[]>> {
  return request.put(`/roles/${id}/permissions`, entries);
}

export function getPermissionList(): Promise<ApiResponse<Permission[]>> {
  return request.get('/permissions');
}
