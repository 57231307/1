import { request } from './request'

export interface DataPermission {
  id?: number
  user_id: number
  resource_type: string
  resource_id?: number
  department_id?: number
  permissions: string[]
}

export const listDataPermissions = (params?: any) =>
  request.get('/data-permissions', { params })

export const getDataPermission = (id: number) =>
  request.get(`/data-permissions/${id}`)

export const createDataPermission = (data: Partial<DataPermission>) =>
  request.post('/data-permissions', data)

export const updateDataPermission = (id: number, data: Partial<DataPermission>) =>
  request.put(`/data-permissions/${id}`, data)

export const deleteDataPermission = (id: number) =>
  request.delete(`/data-permissions/${id}`)
