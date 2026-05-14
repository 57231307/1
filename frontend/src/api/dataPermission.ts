import { request } from './request'

export interface DataPermission {
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

export function setDataPermission(data: SetDataPermissionRequest) {
  return request.post('/api/v1/data-permissions', data)
}

export function listRoleDataPermissions(roleId: number) {
  return request.get(`/api/v1/data-permissions/roles/${roleId}`)
}

export function getDataPermission(roleId: number, resourceType: string) {
  return request.get(`/api/v1/data-permissions/roles/${roleId}/resources/${resourceType}`)
}

export function deleteDataPermission(roleId: number, resourceType: string) {
  return request.delete(`/api/v1/data-permissions/roles/${roleId}/resources/${resourceType}`)
}

export function listScopeTypes() {
  return request.get('/api/v1/data-permissions/scope-types')
}
