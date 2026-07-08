// 慢查询审计 API 模块（P13 批 1 B-慢查询审计）
// 与后端 slow_query_handler 配套：分页 + 多维筛选 / 聚合统计 / 手动刷新

import { request } from './request'

/** 慢查询日志项 */
export interface SlowQueryItem {
  id: number
  query_text: string
  execution_time_ms: number
  calls: number
  rows_examined: number
  database_name?: string | null
  captured_at?: string | null
}

/** 慢查询聚合统计项（按 query_text 分组） */
export interface SlowQueryStatItem {
  query_text: string
  max_exec_time_ms: number
  total_calls: number
  avg_rows: number
  sample_count: number
}

/** 列表查询参数（全部可选） */
export interface SlowQueryListParams {
  start_time?: string
  end_time?: string
  /** 最小执行时间（毫秒） */
  min_duration?: number
  /** 关键词搜索（模糊匹配 query_text） */
  keyword?: string
  page?: number
  page_size?: number
}

/** 列表响应包装 */
export interface SlowQueryListResponse {
  items: SlowQueryItem[]
  total: number
  page: number
  page_size: number
}

/** 统计接口响应 */
export interface SlowQueryStatsResponse {
  top10: SlowQueryStatItem[]
  total_count: number
  time_range: string
}

/** 手动刷新响应 */
export interface SlowQueryRefreshResponse {
  inserted: number
  message: string
}

/** API 通用响应 */
interface ApiResponse<T> {
  code: number
  message: string
  data: T
}

/**
 * 分页查询慢查询日志
 */
export async function listSlowQueries(
  params: SlowQueryListParams = {},
): Promise<SlowQueryListResponse> {
  const res = await request.get<ApiResponse<SlowQueryListResponse>>('/slow-queries', { params })
  return res.data
}

/**
 * 获取慢查询聚合统计（TOP 10 + 总数）
 */
export async function getSlowQueryStats(): Promise<SlowQueryStatsResponse> {
  const res = await request.get<ApiResponse<SlowQueryStatsResponse>>('/slow-queries/stats')
  return res.data
}

/**
 * 手动触发一次慢查询采集
 */
export async function refreshSlowQueries(): Promise<SlowQueryRefreshResponse> {
  const res = await request.post<ApiResponse<SlowQueryRefreshResponse>>('/slow-queries/refresh')
  return res.data
}
