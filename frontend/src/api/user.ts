import { request } from './request'
import type { ApiResponse, QueryParams } from '@/types/api'

export interface User {
  id: number
  username: string
  real_name: string
  email?: string
  phone?: string
  department_id?: number
  department_name?: string
  role_ids?: number[]
  role_names?: string[]
  status: number
  created_at: string
  updated_at: string
}

export interface UserCreateRequest {
  username: string
  password: string
  real_name: string
  email?: string
  phone?: string
  department_id?: number
  role_ids?: number[]
}

export interface UserUpdateRequest {
  real_name?: string
  email?: string
  phone?: string
  department_id?: number
  role_ids?: number[]
  status?: number
}

export interface ChangePasswordRequest {
  old_password: string
  new_password: string
}

export function listUsers(params?: QueryParams): Promise<ApiResponse<User[]>> {
  return request.get('/users', { params })
}

export function getUser(id: number): Promise<ApiResponse<User>> {
  return request.get(`/users/${id}`)
}

export function createUser(data: UserCreateRequest): Promise<ApiResponse<User>> {
  return request.post('/users', data)
}

export function updateUser(id: number, data: UserUpdateRequest): Promise<ApiResponse<User>> {
  return request.put(`/users/${id}`, data)
}

export function deleteUser(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/users/${id}`)
}

export function changePassword(data: ChangePasswordRequest): Promise<ApiResponse<void>> {
  return request.post('/users/change-password', data)
}
