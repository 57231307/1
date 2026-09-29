import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

export interface SystemVersion {
  id: number;
  version: string;
  release_date: string;
  release_notes: string;
  features: string[];
  bug_fixes: string[];
  breaking_changes: string[];
  download_url: string;
  file_size: number;
  checksum: string;
  status: 'available' | 'downloading' | 'downloaded' | 'installing' | 'installed' | 'failed';
  created_at: string;
}

export interface UpdateTask {
  id: number;
  task_code: string;
  /** 任务类型（对齐后端 system_update_task.task_type） */
  task_type?: string;
  from_version: string;
  to_version: string;
  status:
    | 'pending'
    | 'downloading'
    | 'downloaded'
    | 'installing'
    | 'completed'
    | 'failed'
    | 'rolled_back';
  progress: number;
  error_message: string;
  backup_path: string;
  started_at: string;
  completed_at: string;
  created_by: number;
  created_by_name: string;
  created_at: string;
}

export interface SystemBackup {
  id: number;
  backup_code: string;
  backup_type: 'full' | 'incremental' | 'database' | 'files';
  file_path: string;
  file_size: number;
  description: string;
  status: 'creating' | 'completed' | 'failed';
  created_by: number;
  created_by_name: string;
  created_at: string;
}

/**
 * GET /system-update/check 的响应载荷，逐字段对齐后端
 * `backend/src/handlers/system_update_handler.rs:73` 的 `CheckUpdateResponse`。
 * 后端无 `version` 字段（旧前端误用 SystemVersion.version 自算 hasUpdate 导致
 * "已是最新仍永远提示有更新"）。has_update 为后端权威布尔，前端直接采用，不再自算。
 * 各 Option 字段后端未加 skip_serializing_if，序列化时键恒在、值可为 null。
 */
export interface CheckUpdateResult {
  has_update: boolean;
  current_version: string;
  latest_version: string;
  download_url: string | null;
  file_size: number | null;
  release_notes: string | null;
  published_at: string | null;
  current_release_notes: string | null;
  current_published_at: string | null;
}

export function checkForUpdates(): Promise<ApiResponse<CheckUpdateResult>> {
  return request.get('/system-update/check');
}

// 后端 system_update_handler::get_backup_versions() 无参数提取器
export function getSystemVersionList(): Promise<ApiResponse<SystemVersion[]>> {
  return request.get('/system-update/versions');
}

export function getSystemVersion(id: number): Promise<ApiResponse<SystemVersion>> {
  return request.get(`/system-update/versions/${id}`);
}

export function downloadUpdate(versionId: number): Promise<ApiResponse<UpdateTask>> {
  return request.post(`/system-update/versions/${versionId}/download`);
}

export function installUpdate(versionId: number): Promise<ApiResponse<UpdateTask>> {
  return request.post(`/system-update/versions/${versionId}/install`);
}

// 后端 handler: system_update_handler::list_update_tasks (GET /system-update/tasks)
export function getUpdateTaskList(): Promise<ApiResponse<PaginatedResponse<UpdateTask>>> {
  return request.get('/system-update/tasks');
}

// 后端 handler: system_update_handler::list_backup_tasks (GET /system-update/backups)
export function getSystemBackupList(): Promise<ApiResponse<PaginatedResponse<SystemBackup>>> {
  return request.get('/system-update/backups');
}

export function getUpdateTask(id: number): Promise<ApiResponse<UpdateTask>> {
  return request.get(`/system-update/tasks/${id}`);
}

export function cancelUpdateTask(id: number): Promise<ApiResponse<void>> {
  return request.post(`/system-update/tasks/${id}/cancel`);
}

/**
 * 回滚到指定版本
 * 后端路由 POST /api/v1/erp/system-update/rollback
 * 请求体：{ version: string }（后端 RollbackRequest 需要 version 字段，非 taskId）
 */
export function rollbackUpdate(version: string): Promise<ApiResponse<void>> {
  return request.post('/system-update/rollback', { version });
}

export function getSystemBackup(id: number): Promise<ApiResponse<SystemBackup>> {
  return request.get(`/system-update/backups/${id}`);
}

export function createSystemBackup(
  data: Partial<SystemBackup>
): Promise<ApiResponse<SystemBackup>> {
  return request.post('/system-update/backups', data);
}

export function deleteSystemBackup(id: number): Promise<ApiResponse<void>> {
  return request.delete(`/system-update/backups/${id}`);
}

export function restoreFromBackup(id: number): Promise<ApiResponse<void>> {
  return request.post(`/system-update/backups/${id}/restore`);
}

export function downloadBackup(id: number): Promise<Blob> {
  return request.get(`/system-update/backups/${id}/download`, {
    responseType: 'blob',
  });
}

export function getCurrentVersion(): Promise<ApiResponse<{ version: string; build_date: string }>> {
  return request.get('/system-update/current-version');
}
