import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

// 自定义条件类型
export type CustomCondition = Record<string, string | number | boolean | null>

// 允许字段类型
export type AllowedFields = string[]

// 隐藏字段类型
export type HiddenFields = string[]

export interface DataPermission {
  id?: number
  user_id: number
  resource_type: string
  resource_id?: number
  department_id?: number
  permissions: string[]
}

export interface DataPermissionRole {
  id?: number
  roleId?: number
  resourceType?: string
  scopeType?: string
  customCondition?: CustomCondition
  allowedFields?: AllowedFields
  hiddenFields?: HiddenFields
  isEnabled?: boolean
}

export interface SetDataPermissionRequest {
  roleId: number
  resourceType: string
  scopeType: string
  customCondition?: CustomCondition
  allowedFields?: AllowedFields
  hiddenFields?: HiddenFields
}

export interface ScopeType {
  value: string
  label: string
  description: string
}

// 数据权限查询参数
export interface DataPermissionQueryParams extends QueryParams {
  user_id?: number
  resource_type?: string
  department_id?: number
}

export const listDataPermissions = (params?: DataPermissionQueryParams) =>
  request.get<ApiResponse<DataPermission[]>>('/data-permissions/', { params })

export const getDataPermission = (id: number) =>
  request.get<ApiResponse<DataPermission>>(`/data-permissions/${id}`)

export const createDataPermission = (data: Partial<DataPermission>) =>
  request.post<ApiResponse<DataPermission>>('/data-permissions/', data)

export const updateDataPermission = (id: number, data: Partial<DataPermission>) =>
  request.put<ApiResponse<DataPermission>>(`/data-permissions/${id}`, data)

export const deleteDataPermission = (id: number) =>
  request.delete<ApiResponse<void>>(`/data-permissions/${id}`)

export const setDataPermission = (data: SetDataPermissionRequest) =>
  request.post<ApiResponse<DataPermissionRole>>('/data-permissions/', data)

export const listRoleDataPermissions = (roleId: number) =>
  request.get<ApiResponse<DataPermissionRole[]>>(`/data-permissions/roles/${roleId}`)

export const getDataPermissionByRole = (roleId: number, resourceType: string) =>
  request.get<ApiResponse<DataPermissionRole>>(`/data-permissions/roles/${roleId}/${resourceType}`)

export const deleteDataPermissionByRole = (roleId: number, resourceType: string) =>
  request.delete<ApiResponse<void>>(`/data-permissions/roles/${roleId}/${resourceType}`)

export const listScopeTypes = () =>
  request.get<ApiResponse<ScopeType[]>>('/data-permissions/scope-types')
