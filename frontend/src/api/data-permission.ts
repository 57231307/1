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

export const getDataPermissionList = (params?: DataPermissionQueryParams) =>
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

export const getRoleDataPermissionList = (roleId: number) =>
  request.get<ApiResponse<DataPermissionRole[]>>(`/data-permissions/roles/${roleId}`)

export const getDataPermissionByRole = (roleId: number, resourceType: string) =>
  request.get<ApiResponse<DataPermissionRole>>(`/data-permissions/roles/${roleId}/${resourceType}`)

export const deleteDataPermissionByRole = (roleId: number, resourceType: string) =>
  request.delete<ApiResponse<void>>(`/data-permissions/roles/${roleId}/${resourceType}`)

export const getScopeTypeList = () =>
  request.get<ApiResponse<ScopeType[]>>('/data-permissions/scope-types')

/// 数据权限范围类型默认值（v11 P1-5：API 失败时的兜底常量，避免 view 层硬编码）
export const DEFAULT_SCOPE_TYPES: ScopeType[] = [
  { value: 'ALL', label: '全部数据', description: '可以查看所有数据' },
  { value: 'DEPT', label: '本部门数据', description: '只能查看本部门的数据' },
  {
    value: 'DEPT_AND_BELOW',
    label: '本部门及以下',
    description: '可以查看本部门及下级部门的数据',
  },
  { value: 'SELF', label: '仅本人数据', description: '只能查看自己创建的数据' },
  { value: 'CUSTOM', label: '自定义', description: '通过自定义条件过滤数据' },
]
