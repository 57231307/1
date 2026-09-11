import { request } from './request';
import type { ApiResponse, QueryParams } from '@/types/api';

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

export function getRoleList(params?: QueryParams): Promise<ApiResponse<Role[]>> {
  return request.get('/roles', { params });
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

export function assignPermission(
  id: number,
  data: AssignPermissionRequest
): Promise<ApiResponse<void>> {
  return request.post(`/roles/${id}/permissions`, data);
}

export function deletePermission(
  _roleId: number,
  permissionId: number
): Promise<ApiResponse<void>> {
  return request.delete(`/roles/permissions/${permissionId}`);
}

export function getPermissionList(): Promise<ApiResponse<Permission[]>> {
  return request.get('/permissions');
}
