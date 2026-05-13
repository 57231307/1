import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface Department {
  id: number
  name: string
  code: string
  parent_id?: number
  manager_id?: number
  manager_name?: string
  sort_order: number
  status: number
  created_at: string
  updated_at: string
  children?: Department[]
}

export interface DepartmentCreateRequest {
  name: string
  code: string
  parent_id?: number
  manager_id?: number
  sort_order?: number
}

export interface DepartmentUpdateRequest {
  name?: string
  manager_id?: number
  sort_order?: number
  status?: number
}

export function listDepartments(params?: QueryParams): Promise<ApiResponse<Department[]>> {
  return request.get('/api/v1/erp/departments', { params })
}

export function getDepartment(id: number): Promise<ApiResponse<Department>> {
  return request.get(`/api/v1/erp/departments/${id}`)
}

export function createDepartment(data: DepartmentCreateRequest): Promise<ApiResponse<Department>> {
  return request.post('/api/v1/erp/departments', data)
}

export function updateDepartment(id: number, data: DepartmentUpdateRequest): Promise<ApiResponse<Department>> {
  return request.put(`/api/v1/erp/departments/${id}`, data)
}

export function deleteDepartment(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/api/v1/erp/departments/${id}`)
}

export function getDepartmentTree(): Promise<ApiResponse<Department[]>> {
  return request.get('/api/v1/erp/departments/tree')
}
