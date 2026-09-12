import { request } from './request';
import type { ApiResponse, QueryParams, PageResult } from '@/types/api';

export interface User {
  id: number;
  username: string;
  real_name: string;
  email?: string;
  phone?: string;
  department_id?: number;
  department_name?: string;
  role_ids?: number[];
  role_names?: string[];
  // 后端 UserResponse 序列化字段为 is_active (bool)，非 status 数字
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

export interface UserCreateRequest {
  username: string;
  password: string;
  real_name: string;
  email?: string;
  phone?: string;
  department_id?: number;
  // 后端 CreateUserRequest.role_id 为 Option<i32>（单值，非数组）
  role_id?: number;
}

export interface UserUpdateRequest {
  real_name?: string;
  email?: string;
  phone?: string;
  department_id?: number;
  // 后端 UpdateUserRequest.role_id 为 Option<i32>（单值，非数组）
  role_id?: number;
  // 后端 UpdateUserRequest.status 为 "active"/"inactive" 字符串（非数字）
  status?: string;
}

export interface ChangePasswordRequest {
  old_password: string;
  new_password: string;
}

export function getUserList(params?: QueryParams): Promise<ApiResponse<PageResult<User>>> {
  return request.get('/users', { params });
}

export function getUser(id: number): Promise<ApiResponse<User>> {
  return request.get(`/users/${id}`);
}

export function createUser(data: UserCreateRequest): Promise<ApiResponse<User>> {
  return request.post('/users', data);
}

export function updateUser(id: number, data: UserUpdateRequest): Promise<ApiResponse<User>> {
  return request.put(`/users/${id}`, data);
}

export function deleteUser(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/users/${id}`);
}

export function changePassword(data: ChangePasswordRequest): Promise<ApiResponse<void>> {
  return request.post('/users/change-password', data);
}
