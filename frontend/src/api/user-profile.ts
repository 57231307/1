import { request } from './request';
import type { ApiResponse } from '@/types/api';

export interface UserProfile {
  id: number;
  username: string;
  real_name: string;
  email?: string;
  phone?: string;
  avatar?: string;
  department_id?: number;
  department_name?: string;
  role_ids?: number[];
  role_names?: string[];
  status: number;
  created_at: string;
  updated_at: string;
}

// 后端 UpdateCurrentUserProfileRequest 仅接受 real_name/email/phone；
// department_id、role_ids 属管理字段，自助资料更新不读取（已移出请求体）。
export interface UserProfileUpdateRequest {
  real_name?: string;
  email?: string;
  phone?: string;
}

/**
 * 修改密码请求 —— 逐键对齐后端 handlers/user_handler.rs::ChangePasswordRequest（:687-692），
 * DTO 只有 old_password/new_password 两键。confirm_password 是纯 UI 一致性校验项
 * （表单层 rules 拦截），不在后端读取范围，此前随载荷提交属契约漂移，已从请求类型移除
 * （user.ts 的同名类型早已只留两键，本文件与其同源口径）。
 */
export interface ChangePasswordRequest {
  old_password: string;
  new_password: string;
}

export interface AvatarUploadResponse {
  avatar_url: string;
}

export function getUserProfile(): Promise<ApiResponse<UserProfile>> {
  return request.get('/user/profile');
}

export function updateUserProfile(
  data: UserProfileUpdateRequest
): Promise<ApiResponse<UserProfile>> {
  return request.put('/user/profile', data);
}

export function changePassword(data: ChangePasswordRequest): Promise<ApiResponse<void>> {
  return request.post('/users/change-password', data);
}

export function uploadAvatar(file: File): Promise<ApiResponse<AvatarUploadResponse>> {
  const formData = new FormData();
  formData.append('avatar', file);
  return request.post('/user/avatar', formData, {
    headers: {
      'Content-Type': 'multipart/form-data',
    },
  });
}
