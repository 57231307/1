import { request } from './request'
import type { ApiResponse } from '@/types/api'

export interface UserProfile {
  id: number
  username: string
  real_name: string
  email?: string
  phone?: string
  avatar?: string
  department_id?: number
  department_name?: string
  role_ids?: number[]
  role_names?: string[]
  status: number
  created_at: string
  updated_at: string
}

export interface UserProfileUpdateRequest {
  real_name?: string
  email?: string
  phone?: string
  department_id?: number
  role_ids?: number[]
}

export interface ChangePasswordRequest {
  old_password: string
  new_password: string
  confirm_password: string
}

export interface AvatarUploadResponse {
  avatar_url: string
}

export function getUserProfile(): Promise<ApiResponse<UserProfile>> {
  return request.get('/user/profile')
}

export function updateUserProfile(
  data: UserProfileUpdateRequest
): Promise<ApiResponse<UserProfile>> {
  return request.put('/user/profile', data)
}

export function changePassword(data: ChangePasswordRequest): Promise<ApiResponse<void>> {
  return request.post('/user/change-password', data)
}

export function uploadAvatar(file: File): Promise<ApiResponse<AvatarUploadResponse>> {
  const formData = new FormData()
  formData.append('avatar', file)
  return request.post('/user/avatar', formData, {
    headers: {
      'Content-Type': 'multipart/form-data',
    },
  })
}
