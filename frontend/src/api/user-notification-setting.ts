import { request } from './request'
import type { ApiResponse } from './request'

export interface UserNotificationSetting {
  id?: number
  userId?: number
  notificationType?: string
  enableInternal?: boolean
  enableEmail?: boolean
  enableSms?: boolean
  enableWechat?: boolean
  createdAt?: string
  updatedAt?: string
}

export interface NotificationSettingUpdateRequest {
  notificationType: string
  enableInternal?: boolean
  enableEmail?: boolean
  enableSms?: boolean
  enableWechat?: boolean
}

export const userNotificationSettingApi = {
  getMySettings: () =>
    request.get<ApiResponse<{ settings: UserNotificationSetting[] }>>('/user/notification-setting'),

  updateSetting: (data: NotificationSettingUpdateRequest) =>
    request.put<ApiResponse<UserNotificationSetting>>('/user/notification-setting', data),
}
