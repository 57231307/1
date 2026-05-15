import { request } from './request'
import type { ApiResponse } from './request'

export interface SystemUpdateInfo {
  currentVersion: string
  latestVersion: string
  updateAvailable: boolean
  releaseNotes?: string
  downloadUrl?: string
  fileSize?: number
  publishedAt?: string
}

export interface UpdateStatus {
  status: 'idle' | 'checking' | 'downloading' | 'installing' | 'completed' | 'failed'
  progress?: number
  message?: string
  error?: string
}

export const systemUpdateApi = {
  checkForUpdates: () =>
    request.get<ApiResponse<SystemUpdateInfo>>('/system-update/check'),

  downloadAndUpdate: () =>
    request.post<ApiResponse<{ message: string }>>('/system-update/update'),

  getVersion: () =>
    request.get<ApiResponse<{ version: string }>>('/system-update/version'),

  getStatus: () =>
    request.get<ApiResponse<UpdateStatus>>('/system-update/status'),
}
