import { request } from './request';
import type { ApiResponse, PaginatedResponse } from '@/types/api';

/**
 * 应用更新（POST /system-update/update）为下载+校验+二进制替换+落库的同步长任务，
 * 远超全局 axios 默认 30s。仅对这一条请求放宽 per-request timeout，避免长任务被
 * 前端提前掐断而误判失败（后端真实耗时由部署环境决定，此处给足上界）。
 */
const APPLY_UPDATE_TIMEOUT_MS = 300_000;

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

/**
 * GET /system-update/update-status 载荷，逐字段对齐后端
 * `backend/src/handlers/system_update_handler.rs` 的 `UpdateStatusResponse`。
 * 除既有 is_updating（不确定态进度判据）外，新增消费后端本轮落地的真实生效字段：
 * - applied_version：已落库标记为当前的应用版本（system_version.is_current 出口），
 *   更新已应用但进程尚未重启时可能与运行期编译版本不一致，故为 string 或 null。
 * - restart_pending：applied_version 已落库但与当前运行进程编译版本不一致时为 true，
 *   即「已应用待重启生效」；前端据此如实展示生效态，不臆测服务是否已重启。
 * 其余字段（current_version / last_update_time / backup_versions）后端仍返回但更新页不消费，
 * 不在此声明以免与其语义漂移（禁止把读不到的字段当已核实）。
 */
export interface UpdateStatusResult {
  is_updating: boolean;
  applied_version: string | null;
  restart_pending: boolean;
}

/**
 * 查询后端更新应用状态（真实生效态轮询源）。
 * 后端 apply（POST /system-update/update）为同步单请求：请求返回即已落库应用版本，
 * 但进程是否已加载新版本由 restart_pending / applied_version 与运行期版本比对决定，
 * 前端据此轮询直至生效或超时，绝不伪造百分比或谎称已重启。
 */
export function getUpdateStatus(): Promise<ApiResponse<UpdateStatusResult>> {
  return request.get('/system-update/update-status');
}

/**
 * POST /system-update/update 的响应载荷，逐字段对齐后端 `UpdateResult`
 * （success / message / new_version）。message 含后端权威重启结论
 * （已受控自退拉起 / 未探测到监督器需人工重启 / 门控关闭需人工重启），前端如实展示。
 */
export interface UpdateApplyResult {
  success: boolean;
  message: string;
  new_version: string | null;
}

/**
 * 触发应用更新（POST /system-update/update）。
 * 后端 download_and_update 为下载+校验+替换+落库的同步单请求，耗时远超默认 30s，
 * 故给本条长任务单独放宽 per-request timeout（仅 applyUpdate，不影响全局默认）。
 */
export function applyUpdate(): Promise<ApiResponse<UpdateApplyResult>> {
  return request.post('/system-update/update', undefined, { timeout: APPLY_UPDATE_TIMEOUT_MS });
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

export function getCurrentVersion(): Promise<
  ApiResponse<{
    version: string;
    release_date: string | null;
    changelog: string | null;
  }>
> {
  return request.get('/system-update/current-version');
}
