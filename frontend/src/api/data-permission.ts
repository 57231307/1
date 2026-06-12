import { request } from './request'

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
  customCondition?: any
  allowedFields?: any
  hiddenFields?: any
  isEnabled?: boolean
}

export interface SetDataPermissionRequest {
  roleId: number
  resourceType: string
  scopeType: string
  customCondition?: any
  allowedFields?: any
  hiddenFields?: any
}

export interface ScopeType {
  value: string
  label: string
  description: string
}

export const listDataPermissions = (params?: any) => request.get('/data-permissions/', { params })

export const getDataPermission = (id: number) => request.get(`/data-permissions/${id}`)

export const createDataPermission = (data: Partial<DataPermission>) =>
  request.post('/data-permissions/', data)

export const updateDataPermission = (id: number, data: Partial<DataPermission>) =>
  request.put(`/data-permissions/${id}`, data)

export const deleteDataPermission = (id: number) => request.delete(`/data-permissions/${id}`)

export const setDataPermission = (data: SetDataPermissionRequest) =>
  request.post('/data-permissions/', data)

export const listRoleDataPermissions = (roleId: number) =>
  request.get(`/data-permissions/roles/${roleId}`)

export const getDataPermissionByRole = (roleId: number, resourceType: string) =>
  request.get(`/data-permissions/roles/${roleId}/${resourceType}`)

export const deleteDataPermissionByRole = (roleId: number, resourceType: string) =>
  request.delete(`/data-permissions/roles/${roleId}/${resourceType}`)

export const listScopeTypes = () => request.get('/data-permissions/scope-types')
