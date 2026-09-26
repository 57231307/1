import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface Department {
  id: number;
  name: string;
  code: string;
  parent_id?: number;
  manager_id?: number;
  manager_name?: string;
  sort_order: number;
  is_active: boolean;
  created_at: string;
  updated_at: string;
  children?: Department[];
}

export interface DepartmentCreateRequest {
  name: string;
  code: string;
  parent_id?: number;
  manager_id?: number;
  sort_order?: number;
}

export interface DepartmentUpdateRequest {
  name?: string;
  manager_id?: number;
  sort_order?: number;
  is_active?: boolean;
}

// 部门列表查询参数：字段集严格对齐后端 DepartmentListQuery（handlers/department_handler.rs）。
// 搜索键是 search（不是 keyword）；后端不读 order_by/order_dir/status 等通用键。
export interface DepartmentListQuery {
  page?: number;
  page_size?: number;
  parent_id?: number;
  search?: string;
}

// 后端 department_handler::list（define_crud_handlers 宏）返回 PaginatedResponse { items, total, page, page_size }
export function getDepartmentList(
  params?: DepartmentListQuery
): Promise<ApiResponse<{ items: Department[]; total: number; page: number; page_size: number }>> {
  return request.get('/departments', { params });
}

export function getDepartment(id: number): Promise<ApiResponse<Department>> {
  return request.get(`/departments/${id}`);
}

export function createDepartment(data: DepartmentCreateRequest): Promise<ApiResponse<Department>> {
  return request.post('/departments', data);
}

export function updateDepartment(
  id: number,
  data: DepartmentUpdateRequest
): Promise<ApiResponse<Department>> {
  return request.put(`/departments/${id}`, data);
}

export function deleteDepartment(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/departments/${id}`);
}

export function getDepartmentTree(): Promise<ApiResponse<Department[]>> {
  return request.get('/departments/tree');
}
