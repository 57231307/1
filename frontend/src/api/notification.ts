import { request } from './request'
import type { ApiResponse } from './request'

export interface Notification {
  id: number
  title: string
  content: string
  notification_type: string
  is_read: boolean
  business_type?: string
  business_id?: number
  action_url?: string
  sender_id?: number
  sender_name?: string
  created_at: string
}

export interface NotificationSettings {
  email_enabled: boolean
  internal_enabled: boolean
  approval_enabled: boolean
  inventory_alert_enabled: boolean
  order_notification_enabled: boolean
}

export const notificationApi = {
  list: (params?: { page?: number; page_size?: number; is_read?: boolean }) =>
    request.get<ApiResponse<{ list: Notification[]; total: number }>>('/notifications', { params }),

  getById: (id: number) =>
    request.get<ApiResponse<Notification>>(`/notifications/${id}`),

  markAsRead: (id: number) =>
    request.post<ApiResponse<null>>(`/notifications/${id}/read`),

  markAllAsRead: () =>
    request.post<ApiResponse<null>>('/notifications/read-all'),

  batchMarkAsRead: (ids: number[]) =>
    request.post<ApiResponse<null>>('/notifications/batch-read', { ids }),

  delete: (id: number) =>
    request.delete<ApiResponse<null>>(`/notifications/${id}`),

  getUnreadCount: () =>
    request.get<ApiResponse<{ count: number }>>('/notifications/unread-count'),

  getSettings: () =>
    request.get<ApiResponse<NotificationSettings>>('/notifications/settings'),

  updateSettings: (settings: Partial<NotificationSettings>) =>
    request.put<ApiResponse<NotificationSettings>>('/notifications/settings', settings),
}

export interface UserNotificationSetting {
  email_enabled: boolean
  internal_enabled: boolean
  approval_enabled: boolean
  inventory_alert_enabled: boolean
  order_notification_enabled: boolean
}

export const userNotificationSettingApi = {
  getSetting: () =>
    request.get<ApiResponse<UserNotificationSetting>>('/user/notification-setting'),

  updateSetting: (settings: Partial<UserNotificationSetting>) =>
    request.put<ApiResponse<UserNotificationSetting>>('/user/notification-setting', settings),
}
