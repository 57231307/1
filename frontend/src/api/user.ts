import { request } from './request';
import type { ApiResponse, PageResult } from '@/types/api';

export interface User {
  id: number;
  username: string;
  real_name: string;
  email?: string;
  phone?: string;
  department_id?: number;
  department_name?: string;
  // 后端 UserResponse.role_id (Option<i32>)
  role_id?: number;
  role_ids?: number[];
  role_names?: string[];
  // 后端 UserResponse 序列化字段为 is_active (bool)，非 status 数字
  is_active: boolean;
  created_at: string;
  updated_at: string;
}

// 后端 CreateUserRequest 无 real_name（真实姓名仅 UserResponse 返回、无写入通道），
// role_id / department_id 为可选单值。
export interface UserCreateRequest {
  username: string;
  password: string;
  email?: string;
  phone?: string;
  role_id?: number;
  department_id?: number;
}

// 后端 UpdateUserRequest 无 username/password/real_name；status 为 "active"/"inactive" 字符串。
export interface UserUpdateRequest {
  email?: string;
  phone?: string;
  role_id?: number;
  department_id?: number;
  status?: string;
}

export interface ChangePasswordRequest {
  old_password: string;
  new_password: string;
}

/**
 * GET /users 查询参数。对应后端 user_handler::ListUsersParams
 * （backend/src/handlers/user_handler.rs:402），无 rename_all → snake_case，全 Option → 可选。
 * status 为数字（Option<i8>，1=启用 0=停用），非字符串。
 */
export interface UserListParams {
  page?: number;
  page_size?: number;
  keyword?: string;
  status?: number;
}

export function getUserList(params?: UserListParams): Promise<ApiResponse<PageResult<User>>> {
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
