import { request } from './request'
import type { ApiResponse } from '../types/api'

export interface UserNotificationSetting {
  id: number
  userId: number
  channel: 'email' | 'sms' | 'in_app' | 'push'
  eventType: string
  enabled: boolean
  threshold?: number
  createdAt: string
  updatedAt: string
}

export interface NotificationPreference {
  email: boolean
  sms: boolean
  inApp: boolean
  push: boolean
}

export interface UpdateNotificationSettingRequest {
  channel: 'email' | 'sms' | 'in_app' | 'push'
  eventType: string
  enabled: boolean
  threshold?: number
}

export interface NotificationEventType {
  type: string
  name: string
  description: string
  channels: string[]
}

export function getUserNotificationSettings(): Promise<ApiResponse<UserNotificationSetting[]>> {
  return request.get('/user/notification-settings')
}

export function updateUserNotificationSetting(data: UpdateNotificationSettingRequest): Promise<ApiResponse<UserNotificationSetting>> {
  return request.put('/user/notification-settings', data)
}

export function batchUpdateNotificationSettings(data: UpdateNotificationSettingRequest[]): Promise<ApiResponse<void>> {
  return request.post('/user/notification-settings/batch', data)
}

export function resetNotificationSettings(): Promise<ApiResponse<void>> {
  return request.post('/user/notification-settings/reset')
}

export function getNotificationEventTypes(): Promise<ApiResponse<NotificationEventType[]>> {
  return request.get('/user/notification-settings/event-types')
}

export function getNotificationPreferences(): Promise<ApiResponse<NotificationPreference>> {
  return request.get('/user/notification-settings/preferences')
}

export function updateNotificationPreferences(data: NotificationPreference): Promise<ApiResponse<NotificationPreference>> {
  return request.put('/user/notification-settings/preferences', data)
}
