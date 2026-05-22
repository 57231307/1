import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface Role {
  id: number
  name: string
  code: string
  description?: string
  status: number
  created_at: string
  updated_at: string
  permissions?: Permission[]
}

export interface Permission {
  id: number
  name: string
  code: string
  type: string
  resource?: string
  action?: string
  parent_id?: number
}

export interface RoleCreateRequest {
  name: string
  code: string
  description?: string
}

export interface RoleUpdateRequest {
  name?: string
  description?: string
  status?: number
}

export interface AssignPermissionRequest {
  permission_ids: number[]
}

export function listRoles(params?: QueryParams): Promise<ApiResponse<Role[]>> {
  return request.get('/roles', { params })
}

export function getRole(id: number): Promise<ApiResponse<Role>> {
  return request.get(`/roles/${id}`)
}

export function createRole(data: RoleCreateRequest): Promise<ApiResponse<Role>> {
  return request.post('/roles', data)
}

export function updateRole(id: number, data: RoleUpdateRequest): Promise<ApiResponse<Role>> {
  return request.put(`/roles/${id}`, data)
}

export function deleteRole(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/roles/${id}`)
}

export function getRolePermissions(id: number): Promise<ApiResponse<Permission[]>> {
  return request.get(`/roles/${id}/permissions`)
}

export function assignPermission(id: number, data: AssignPermissionRequest): Promise<ApiResponse<void>> {
  return request.post(`/roles/${id}/permissions`, data)
}

export function removePermission(_roleId: number, permissionId: number): Promise<ApiResponse<void>> {
  return request.delete(`/roles/permissions/${permissionId}`)
}

export function listPermissions(): Promise<ApiResponse<Permission[]>> {
  return request.get('/permissions')
}
