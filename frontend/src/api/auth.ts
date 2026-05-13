import request from './request'
import type { LoginRequest, LoginResponse, UserInfo } from '@/types/api'

export function login(data: LoginRequest): Promise<LoginResponse> {
  return request.post('/auth/login', data)
}

export function logout(): Promise<void> {
  return request.post('/auth/logout')
}

export function refreshToken(refreshToken: string): Promise<{ token: string }> {
  return request.post('/auth/refresh', { refresh_token: refreshToken })
}

export function getUserInfo(): Promise<UserInfo> {
  return request.get('/auth/me')
}
