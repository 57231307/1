import { request } from './request'
import type { ApiResponse } from './request'

export interface DataPermission {
  id?: number
  roleId?: number
  roleName?: string
  resourceType: string
  resourceName?: string
  permissionType: 'all' | 'custom' | 'none'
  customCondition?: string
  allowedFields?: string[]
  hiddenFields?: string[]
  description?: string
  createdBy?: number
  createdAt?: string
  updatedAt?: string
}

export interface DataPermissionQueryParams {
  roleId?: number
  resourceType?: string
}

export const dataPermissionApi = {
  list: (params?: DataPermissionQueryParams) =>
    request.get<ApiResponse<{ list: DataPermission[]; total: number }>>('/data-permissions', { params }),

  getByRole: (roleId: number) =>
    request.get<ApiResponse<{ permissions: DataPermission[] }>>(`/data-permissions/role/${roleId}`),

  create: (data: Partial<DataPermission>) =>
    request.post<ApiResponse<DataPermission>>('/data-permissions', data),

  update: (id: number, data: Partial<DataPermission>) =>
    request.put<ApiResponse<DataPermission>>(`/data-permissions/${id}`, data),

  delete: (id: number) =>
    request.delete<ApiResponse<void>>(`/data-permissions/${id}`),
}
