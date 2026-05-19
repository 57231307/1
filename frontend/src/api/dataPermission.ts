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
  return request.post('/data-permissions', data)
}

export function listRoleDataPermissions(roleId: number) {
  return request.get(`/data-permissions/roles/${roleId}`)
}

export function getDataPermission(roleId: number, resourceType: string) {
  return request.get(`/data-permissions/roles/${roleId}/${resourceType}`)
}

export function deleteDataPermission(roleId: number, resourceType: string) {
  return request.delete(`/data-permissions/roles/${roleId}/${resourceType}`)
}

export function listScopeTypes() {
  return request.get('/data-permissions/scope-types')
}
