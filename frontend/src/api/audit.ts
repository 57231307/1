// 审计日志 API 模块（P13 批 1 P3-2）
// 与后端 audit_log_handler 配套：分页 + 多维筛选 / 详情 / CSV 导出

import { request } from './request'

/** 操作类型枚举（与后端 OperationType 同步） */
export type OperationType =
  | 'CREATE'
  | 'UPDATE'
  | 'DELETE'
  | 'LOGIN'
  | 'LOGOUT'
  | 'EXPORT'
  | 'QUERY'
  | 'OTHER'

/** 严重级别枚举（与后端 Severity 同步） */
export type Severity = 'INFO' | 'WARN' | 'ERROR' | 'CRITICAL'

/** 审计日志列表项 */
export interface AuditLogItem {
  id: number
  user_id?: number | null
  username?: string | null
  operation_type?: OperationType | null
  severity?: Severity | null
  resource_type?: string | null
  resource_id?: string | null
  resource_name?: string | null
  description?: string | null
  ip_address?: string | null
  user_agent?: string | null
  request_id?: string | null
  request_method?: string | null
  request_path?: string | null
  created_at?: string | null
}

/** 审计日志详情（叠加 before/after 快照） */
export interface AuditLogDetail extends AuditLogItem {
  before_snapshot?: unknown
  after_snapshot?: unknown
  old_value?: unknown
  new_value?: unknown
}

/** 列表查询参数（全部可选） */
export interface AuditLogListParams {
  start_time?: string
  end_time?: string
  user_id?: number
  operation_type?: OperationType
  severity?: Severity
  resource_type?: string
  request_id?: string
  keyword?: string
  page?: number
  page_size?: number
}

/** 列表响应包装 */
export interface AuditLogListResponse {
  items: AuditLogItem[]
  total: number
  page: number
  page_size: number
}

/** API 通用响应 */
interface ApiResponse<T> {
  code: number
  message: string
  data: T
}

/**
 * 分页查询审计日志
 */
export async function getAuditLogList(params: AuditLogListParams = {}): Promise<AuditLogListResponse> {
  const res = await request.get<ApiResponse<AuditLogListResponse>>('/audit-logs', { params })
  return res.data
}

/**
 * 获取审计日志详情
 */
export async function getAuditLog(id: number): Promise<AuditLogDetail> {
  const res = await request.get<ApiResponse<AuditLogDetail>>(`/audit-logs/${id}`)
  return res.data
}

/**
 * 导出审计日志（CSV 文件流）
 * 返回 blob，前端用 `URL.createObjectURL` 触发下载
 */
export function exportAuditLogs(params: AuditLogListParams = {}): Promise<Blob> {
  return request.get<Blob>('/audit-logs/export', {
    params,
    responseType: 'blob',
  })
}

/**
 * V15 P1-5-3：前端打印审计埋点请求参数
 *
 * printData/printSingleDocument 纯前端 window.print 不经过后端 handler，
 * 需通过此 API 上报到后端 audit_logs（OperationType::Print）。
 */
export interface RecordPrintEventParams {
  /** 资源类型（如 customer / supplier / warehouse，与权限码 resource_type 对应） */
  resourceType: string
  /** 打印记录数（data.length） */
  recordCount: number
  /** 打印文档标题（PrintOptions.title） */
  title: string
  /** 资源 ID（可选，单据打印时传入） */
  resourceId?: string
}

/**
 * V15 P1-5-3：前端打印审计埋点
 *
 * best-effort 上报，调用方应 .catch() 吞掉异常避免影响打印流程。
 * 后端端点 /audit-logs/record-print 仅需认证，不要求特定权限码。
 */
export function recordPrintAudit(params: RecordPrintEventParams): Promise<void> {
  return request.post('/audit-logs/record-print', {
    resource_type: params.resourceType,
    record_count: params.recordCount,
    title: params.title,
    resource_id: params.resourceId,
  })
}
