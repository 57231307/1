import { request } from './request'
import type { ApiResponse, QueryParams } from '../types/api'

export interface DataPermission {
  id: number
  roleId: number
  resourceType: string
  scopeType: string
  scopeValues: string[]
  createdAt: string
}

export interface DataPermissionQueryParams extends QueryParams {
  roleId?: number
  resourceType?: string
}

export interface SetDataPermissionRequest {
  roleId: number
  resourceType: string
  scopeType: string
  scopeValues: string[]
}

export interface ScopeType {
  type: string
  name: string
  description: string
}

export function listDataPermissions(params?: DataPermissionQueryParams): Promise<ApiResponse<{ list: DataPermission[]; total: number }>> {
  return request.get('/data-permissions', { params })
}

export function getDataPermission(roleId: number, resourceType: string): Promise<ApiResponse<DataPermission>> {
  return request.get(`/data-permissions/${roleId}/${resourceType}`)
}

export function setDataPermission(data: SetDataPermissionRequest): Promise<ApiResponse<DataPermission>> {
  return request.post('/data-permissions', data)
}

export function deleteDataPermission(roleId: number, resourceType: string): Promise<ApiResponse<void>> {
  return request.delete(`/data-permissions/${roleId}/${resourceType}`)
}

export function listScopeTypes(): Promise<ApiResponse<ScopeType[]>> {
  return request.get('/data-permissions/scope-types')
}

export function getRoleDataPermissions(roleId: number): Promise<ApiResponse<DataPermission[]>> {
  return request.get(`/data-permissions/roles/${roleId}`)
}
